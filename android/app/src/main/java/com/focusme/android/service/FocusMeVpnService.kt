// ============================================================
// FILE:        FocusMeVpnService.kt
// MODULE:      Layer 1 — Enforcement Engine > Android VPN-based DNS Blocking
// TASK:        T-042
// PLATFORM:    android
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Phase 3, Android DNS blocking via local VPN
// DEPENDENCIES: VpnService API (Android 4.0+)
// TEST COVERAGE: Test: DNS query for blocked domain returns NXDOMAIN
// KNOWN LIMITATIONS: Only one VPN can be active at a time on Android.
//                    User must grant VPN permission (system dialog).
//                    Cannot coexist with other VPN apps.
//                    DNS-over-HTTPS (DoH) in apps may bypass.
// ============================================================

package com.focusme.android.service

import android.content.Intent
import android.net.VpnService
import android.os.ParcelFileDescriptor
import android.util.Log
import java.io.FileInputStream
import java.io.FileOutputStream
import java.net.InetAddress
import java.nio.ByteBuffer

/**
 * FocusMeVpnService — intercepts DNS queries via a local VPN tunnel
 * and returns NXDOMAIN for blocked domains.
 *
 * How it works:
 * 1. Establishes a local VPN tunnel (tun interface)
 * 2. Routes all DNS traffic (port 53) through the tunnel
 * 3. Inspects DNS queries and blocks matching domains with NXDOMAIN
 * 4. Forwards allowed queries to the upstream DNS resolver
 *
 * Why VpnService for DNS:
 * - No root required (unlike iptables)
 * - System-wide DNS interception
 * - Works on all Android 4.0+ devices
 * - See decision D-007 (Android DNS blocking via local VPN)
 */
class FocusMeVpnService : VpnService() {

    companion object {
        private const val TAG = "FocusMeVPN"

        /** Virtual TUN interface address */
        private const val VPN_ADDRESS = "10.255.255.1"

        /** DNS server to intercept */
        private const val VPN_DNS = "10.255.255.2"

        /** MTU for the TUN interface */
        private const val VPN_MTU = 1500

        /** Upstream DNS server (Google Public DNS) */
        private const val UPSTREAM_DNS = "8.8.8.8"

        /** Singleton reference for cross-service communication */
        @Volatile
        var instance: FocusMeVpnService? = null
            private set
    }

    // ---- State ----

    /** VPN tunnel file descriptor */
    private var vpnInterface: ParcelFileDescriptor? = null

    /** Set of blocked domain names */
    private val blockedDomains = mutableSetOf<String>()

    /** Lock for thread-safe access */
    private val lock = Any()

    /** Running flag */
    @Volatile
    private var isRunning = false

    /** DNS processing thread */
    private var processingThread: Thread? = null

    // ---- Lifecycle ----

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        Log.i(TAG, "VPN service starting")
        instance = this

        if (vpnInterface == null) {
            establishVpn()
        }

        return START_STICKY
    }

    override fun onDestroy() {
        super.onDestroy()
        isRunning = false
        instance = null
        processingThread?.interrupt()
        vpnInterface?.close()
        vpnInterface = null
        Log.i(TAG, "VPN service destroyed")
    }

    // ---- VPN Setup ----

    /**
     * Establish the local VPN tunnel
     */
    private fun establishVpn() {
        try {
            val builder = Builder()
                .setSession("FocusMe DNS Filter")
                .addAddress(VPN_ADDRESS, 32)
                .addDnsServer(VPN_DNS)
                .setMtu(VPN_MTU)
                .setBlocking(true)

            // Only route DNS traffic through the VPN (port 53)
            // This avoids routing all traffic and impacting performance
            // TODO: Route only UDP port 53 traffic
            builder.addRoute(VPN_DNS, 32) // Only route to our virtual DNS

            vpnInterface = builder.establish()

            if (vpnInterface != null) {
                isRunning = true
                startDnsProcessing()
                Log.i(TAG, "VPN tunnel established")
            } else {
                Log.e(TAG, "Failed to establish VPN tunnel — user may have denied permission")
            }
        } catch (e: Exception) {
            Log.e(TAG, "Error establishing VPN", e)
        }
    }

    // ---- DNS Processing ----

    /**
     * Start the DNS packet processing loop on a background thread
     */
    private fun startDnsProcessing() {
        processingThread = Thread({
            val vpnFd = vpnInterface ?: return@Thread
            val inputStream = FileInputStream(vpnFd.fileDescriptor)
            val outputStream = FileOutputStream(vpnFd.fileDescriptor)
            val buffer = ByteBuffer.allocate(VPN_MTU)

            while (isRunning) {
                try {
                    buffer.clear()
                    val length = inputStream.read(buffer.array())
                    if (length <= 0) continue

                    buffer.limit(length)

                    // Parse IP packet → extract DNS query
                    val dnsQuery = extractDnsQuery(buffer)
                    if (dnsQuery != null) {
                        val domain = dnsQuery.domain

                        val isBlocked = synchronized(lock) {
                            isDomainBlocked(domain)
                        }

                        if (isBlocked) {
                            Log.i(TAG, "DNS BLOCKED: $domain")
                            val nxdomainResponse = synthesizeNxdomain(buffer, dnsQuery)
                            outputStream.write(nxdomainResponse)
                            outputStream.flush()
                        } else {
                            // Forward to upstream DNS
                            forwardDnsQuery(buffer, outputStream)
                        }
                    }
                } catch (e: InterruptedException) {
                    break
                } catch (e: Exception) {
                    Log.w(TAG, "Error processing DNS packet", e)
                }
            }
        }, "FocusMe-DNS-Processor").apply {
            isDaemon = true
            start()
        }
    }

    // ---- DNS Parsing ----

    /** Parsed DNS query data */
    data class DnsQuery(
        val domain: String,
        val transactionId: Short,
        val queryType: Short,
    )

    /**
     * Extract the DNS query domain from an IP packet
     *
     * IP header (20 bytes) → UDP header (8 bytes) → DNS payload
     * DNS name format: length-prefixed labels, e.g. \x06reddit\x03com\x00
     */
    private fun extractDnsQuery(packet: ByteBuffer): DnsQuery? {
        try {
            if (packet.remaining() < 28) return null // Minimum IP(20) + UDP(8)

            packet.position(0)

            // ── IP Header ──
            val versionIhl = packet.get().toInt() and 0xFF
            val ipVersion = (versionIhl shr 4) and 0x0F
            if (ipVersion != 4) return null // IPv4 only for now

            val ihl = (versionIhl and 0x0F) * 4 // Header length in bytes
            if (ihl < 20 || packet.remaining() < ihl) return null

            // Skip to protocol field (byte 9)
            packet.position(9)
            val protocol = packet.get().toInt() and 0xFF
            if (protocol != 17) return null // UDP only

            // Skip to end of IP header
            packet.position(ihl)

            // ── UDP Header (8 bytes) ──
            if (packet.remaining() < 8) return null
            val srcPort = packet.short.toInt() and 0xFFFF
            val dstPort = packet.short.toInt() and 0xFFFF
            val udpLength = packet.short.toInt() and 0xFFFF
            val udpChecksum = packet.short // skip

            // Only process DNS queries (destination port 53)
            if (dstPort != 53) return null
            if (packet.remaining() < 12) return null // Minimum DNS header

            // ── DNS Header (12 bytes) ──
            val transactionId = packet.short
            val flags = packet.short.toInt() and 0xFFFF
            val isQuery = (flags and 0x8000) == 0 // QR bit = 0 → query
            if (!isQuery) return null

            val qdCount = packet.short.toInt() and 0xFFFF // Question count
            val anCount = packet.short // skip
            val nsCount = packet.short // skip
            val arCount = packet.short // skip

            if (qdCount < 1) return null

            // ── DNS Question Section ──
            val domainBuilder = StringBuilder()
            while (packet.hasRemaining()) {
                val labelLen = packet.get().toInt() and 0xFF
                if (labelLen == 0) break // Root label (end of name)
                if (labelLen > 63) return null // Invalid label length
                if (packet.remaining() < labelLen) return null

                if (domainBuilder.isNotEmpty()) domainBuilder.append('.')
                val label = ByteArray(labelLen)
                packet.get(label)
                domainBuilder.append(String(label, Charsets.US_ASCII))
            }

            if (domainBuilder.isEmpty()) return null

            // Query type (2 bytes) + query class (2 bytes)
            if (packet.remaining() < 4) return null
            val queryType = packet.short

            return DnsQuery(
                domain = domainBuilder.toString(),
                transactionId = transactionId,
                queryType = queryType,
            )
        } catch (e: Exception) {
            Log.w(TAG, "DNS parse error", e)
            return null
        }
    }

    /**
     * Synthesize an NXDOMAIN DNS response for a blocked domain.
     *
     * Constructs a valid IP/UDP/DNS response packet:
     * - Swaps IP src/dst addresses
     * - Swaps UDP src/dst ports
     * - Sets DNS flags: QR=1, RCODE=3 (NXDOMAIN)
     * - Copies the question section, no answer section
     */
    private fun synthesizeNxdomain(originalPacket: ByteBuffer, query: DnsQuery): ByteArray {
        try {
            originalPacket.position(0)
            val originalBytes = ByteArray(originalPacket.remaining())
            originalPacket.get(originalBytes)

            // Parse IP header length
            val ihl = (originalBytes[0].toInt() and 0x0F) * 4

            // ── Build response DNS payload ──
            // DNS header (12 bytes) + copied question section
            val questionStart = ihl + 8 + 12 // IP header + UDP header + DNS header
            // Find end of question: scan labels until 0x00, then +4 (QTYPE + QCLASS)
            var qEnd = questionStart
            while (qEnd < originalBytes.size) {
                val labelLen = originalBytes[qEnd].toInt() and 0xFF
                if (labelLen == 0) { qEnd += 1 + 4; break } // null + QTYPE(2) + QCLASS(2)
                qEnd += 1 + labelLen
            }
            qEnd = minOf(qEnd, originalBytes.size)

            val questionSection = originalBytes.copyOfRange(questionStart, qEnd)
            val dnsPayloadSize = 12 + questionSection.size

            // DNS response header
            val dnsHeader = ByteBuffer.allocate(12)
            dnsHeader.putShort(query.transactionId)
            // Flags: QR=1, OPCODE=0, AA=1, TC=0, RD=1, RA=1, RCODE=3 (NXDOMAIN)
            dnsHeader.putShort(0x8583.toShort()) // 1000 0101 1000 0011
            dnsHeader.putShort(1) // QDCOUNT = 1 (copy question)
            dnsHeader.putShort(0) // ANCOUNT = 0
            dnsHeader.putShort(0) // NSCOUNT = 0
            dnsHeader.putShort(0) // ARCOUNT = 0

            val dnsPayload = dnsHeader.array() + questionSection

            // ── Build UDP header (8 bytes) ──
            val udpTotalLen = 8 + dnsPayload.size
            val udpHeader = ByteBuffer.allocate(8)
            // Swap ports: src ← original dst (53), dst ← original src
            val origSrcPort = ((originalBytes[ihl].toInt() and 0xFF) shl 8) or
                (originalBytes[ihl + 1].toInt() and 0xFF)
            udpHeader.putShort(53.toShort()) // src = DNS server
            udpHeader.putShort(origSrcPort.toShort()) // dst = original client port
            udpHeader.putShort(udpTotalLen.toShort())
            udpHeader.putShort(0) // Checksum = 0 (optional for UDP over IPv4)

            // ── Build IP header (20 bytes, no options) ──
            val totalLen = 20 + udpTotalLen
            val ipHeader = ByteBuffer.allocate(20)
            ipHeader.put(0x45.toByte()) // Version=4, IHL=5 (20 bytes)
            ipHeader.put(0x00.toByte()) // DSCP/ECN
            ipHeader.putShort(totalLen.toShort())
            ipHeader.putShort(0) // Identification
            ipHeader.putShort(0x4000.toShort()) // Flags: Don't Fragment
            ipHeader.put(64.toByte()) // TTL
            ipHeader.put(17.toByte()) // Protocol = UDP
            ipHeader.putShort(0) // Checksum placeholder

            // Swap src/dst IP addresses
            val srcIp = originalBytes.copyOfRange(16, 20) // original dst → new src
            val dstIp = originalBytes.copyOfRange(12, 16) // original src → new dst
            ipHeader.put(srcIp)
            ipHeader.put(dstIp)

            // Calculate IP header checksum
            val ipBytes = ipHeader.array()
            var checksum = 0L
            for (i in ipBytes.indices step 2) {
                val word = ((ipBytes[i].toInt() and 0xFF) shl 8) or
                    (ipBytes[i + 1].toInt() and 0xFF)
                checksum += word
            }
            while (checksum shr 16 != 0L) {
                checksum = (checksum and 0xFFFF) + (checksum shr 16)
            }
            val checksumShort = (checksum.inv() and 0xFFFF).toShort()
            ipBytes[10] = ((checksumShort.toInt() shr 8) and 0xFF).toByte()
            ipBytes[11] = (checksumShort.toInt() and 0xFF).toByte()

            return ipBytes + udpHeader.array() + dnsPayload
        } catch (e: Exception) {
            Log.e(TAG, "NXDOMAIN synthesis error", e)
            return ByteArray(0)
        }
    }

    /**
     * Forward a DNS query to the upstream DNS server via a protect()ed socket
     * to avoid routing loops through the VPN tunnel.
     */
    private fun forwardDnsQuery(packet: ByteBuffer, outputStream: FileOutputStream) {
        try {
            packet.position(0)
            val rawBytes = ByteArray(packet.remaining())
            packet.get(rawBytes)

            val ihl = (rawBytes[0].toInt() and 0x0F) * 4
            val dnsPayloadStart = ihl + 8 // IP header + UDP header
            if (dnsPayloadStart >= rawBytes.size) return

            val dnsPayload = rawBytes.copyOfRange(dnsPayloadStart, rawBytes.size)

            // Extract original client port
            val clientPort = ((rawBytes[ihl].toInt() and 0xFF) shl 8) or
                (rawBytes[ihl + 1].toInt() and 0xFF)

            // Send DNS query to upstream via protected socket (bypasses VPN)
            val socket = java.net.DatagramSocket()
            protect(socket) // Critical: prevents routing loop
            socket.soTimeout = 5000 // 5s timeout

            val upstreamAddr = InetAddress.getByName(UPSTREAM_DNS)
            val sendPacket = java.net.DatagramPacket(dnsPayload, dnsPayload.size, upstreamAddr, 53)
            socket.send(sendPacket)

            // Receive response
            val responseBuf = ByteArray(1500)
            val recvPacket = java.net.DatagramPacket(responseBuf, responseBuf.size)
            socket.receive(recvPacket)
            socket.close()

            val responsePayload = responseBuf.copyOfRange(0, recvPacket.length)

            // Wrap response in IP/UDP headers and write to TUN
            val udpTotalLen = 8 + responsePayload.size
            val totalLen = 20 + udpTotalLen

            val response = ByteBuffer.allocate(totalLen)

            // IP header (swap addresses from original)
            response.put(0x45.toByte())
            response.put(0x00.toByte())
            response.putShort(totalLen.toShort())
            response.putShort(0) // ID
            response.putShort(0x4000.toShort()) // Don't Fragment
            response.put(64.toByte()) // TTL
            response.put(17.toByte()) // UDP
            response.putShort(0) // Checksum placeholder
            response.put(rawBytes, 16, 4) // Original dst → new src
            response.put(rawBytes, 12, 4) // Original src → new dst

            // Calculate IP checksum
            val ipArr = response.array()
            var cksum = 0L
            for (i in 0 until 20 step 2) {
                cksum += ((ipArr[i].toInt() and 0xFF) shl 8) or (ipArr[i + 1].toInt() and 0xFF)
            }
            while (cksum shr 16 != 0L) cksum = (cksum and 0xFFFF) + (cksum shr 16)
            val cs = (cksum.inv() and 0xFFFF).toShort()
            ipArr[10] = ((cs.toInt() shr 8) and 0xFF).toByte()
            ipArr[11] = (cs.toInt() and 0xFF).toByte()

            // UDP header
            response.position(20)
            response.putShort(53.toShort()) // src = DNS
            response.putShort(clientPort.toShort()) // dst = client
            response.putShort(udpTotalLen.toShort())
            response.putShort(0) // checksum optional

            // DNS payload
            response.put(responsePayload)

            outputStream.write(response.array(), 0, totalLen)
            outputStream.flush()
        } catch (e: Exception) {
            Log.w(TAG, "DNS forward error", e)
        }
    }

    // ---- Domain Matching ----

    /**
     * Check if a domain should be blocked (exact + subdomain matching)
     */
    private fun isDomainBlocked(domain: String): Boolean {
        val normalizedDomain = domain.lowercase().trimEnd('.')

        // Exact match
        if (blockedDomains.contains(normalizedDomain)) return true

        // Subdomain match: "sub.reddit.com" blocked if "reddit.com" is blocked
        val parts = normalizedDomain.split(".")
        for (i in 1 until parts.size) {
            val parent = parts.subList(i, parts.size).joinToString(".")
            if (blockedDomains.contains(parent)) return true
        }

        return false
    }

    // ---- Rule Management ----

    /**
     * Update the set of blocked domains from active plans
     * Called by FocusMeDaemonService when plans change
     */
    fun updateBlockedDomains(domains: Set<String>) {
        synchronized(lock) {
            blockedDomains.clear()
            blockedDomains.addAll(domains.map { it.lowercase().trimEnd('.') })
        }
        Log.i(TAG, "Updated blocked domains: ${domains.size} entries")
    }
}
