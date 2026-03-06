// ============================================================
// FILE:        DNSProxyProvider.swift
// MODULE:      Layer 1 — Enforcement Engine > macOS DNS Blocking
// TASK:        T-015
// PLATFORM:    macos
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Phase 1, macOS DNS proxy
// DEPENDENCIES: NetworkExtension.framework
// TEST COVERAGE: Test: blocked domain returns NXDOMAIN
// KNOWN LIMITATIONS: Requires user approval in System Settings > Network Extensions.
//                    User can revoke Network Extension permission at any time.
// ============================================================

import Foundation
import NetworkExtension

// MARK: - FocusMeDNSProxyProvider

/// DNS Proxy Provider that intercepts DNS queries and returns NXDOMAIN
/// for blocked domains. Uses NEDNSProxyProvider from NetworkExtension framework.
///
/// The user must approve this Network Extension in System Settings > Privacy & Security.
/// This is a UX friction point (see OQ-06).
class FocusMeDNSProxyProvider: NEDNSProxyProvider {

    /// Set of blocked domain names (loaded from daemon via IPC)
    private var blockedDomains: Set<String> = []

    /// Lock for thread-safe access
    private let lock = NSLock()

    // MARK: - NEDNSProxyProvider Lifecycle

    override func startProxy(options: [String: Any]? = nil, completionHandler: @escaping (Error?) -> Void) {
        NSLog("[FocusMe DNS] DNS Proxy Provider starting...")

        // TODO: Connect to FocusMe daemon via IPC to receive blocked domain list
        // TODO: Load initial blocked domains from persistent cache

        // Load default blocked domains for testing
        // In production, these come from the daemon via UDS IPC
        blockedDomains = [] // Loaded dynamically

        NSLog("[FocusMe DNS] DNS Proxy started with \(blockedDomains.count) blocked domains")
        completionHandler(nil)
    }

    override func stopProxy(with reason: NEProviderStopReason, completionHandler: @escaping () -> Void) {
        NSLog("[FocusMe DNS] DNS Proxy stopping, reason: \(reason)")
        completionHandler()
    }

    // MARK: - DNS Query Handling

    override func handleNewFlow(_ flow: NEAppProxyFlow) -> Bool {
        guard let udpFlow = flow as? NEAppProxyUDPFlow else {
            NSLog("[FocusMe DNS] Ignoring non-UDP flow")
            return false
        }

        // Read DNS datagrams from the flow
        udpFlow.readDatagrams { [weak self] datagrams, endpoints, error in
            guard let self = self else { return }

            if let error = error {
                NSLog("[FocusMe DNS] Read error: \(error.localizedDescription)")
                udpFlow.closeReadWithError(error)
                return
            }

            guard let datagrams = datagrams, let endpoints = endpoints else {
                udpFlow.closeReadWithError(nil)
                return
            }

            for (i, datagram) in datagrams.enumerated() {
                self.processDnsQuery(datagram: datagram, endpoint: endpoints[i], flow: udpFlow)
            }
        }

        return true // We handle this flow
    }

    /// Process a single DNS query datagram.
    /// If the queried domain is blocked, return NXDOMAIN.
    /// Otherwise, forward to the upstream DNS resolver.
    private func processDnsQuery(datagram: Data, endpoint: NWEndpoint, flow: NEAppProxyUDPFlow) {
        // Parse the DNS query domain
        guard let domain = extractDomainFromQuery(datagram) else {
            // Not a valid DNS query or can't parse — forward as-is
            forwardDnsQuery(datagram: datagram, endpoint: endpoint, flow: flow)
            return
        }

        if isDomainBlocked(domain) {
            NSLog("[FocusMe DNS] BLOCKED: \(domain)")
            let nxdomainResponse = synthesizeNXDOMAIN(queryData: datagram)
            if !nxdomainResponse.isEmpty {
                flow.writeDatagrams([nxdomainResponse], sentBy: [endpoint]) { error in
                    if let error = error {
                        NSLog("[FocusMe DNS] Write NXDOMAIN error: \(error.localizedDescription)")
                    }
                }
            }
        } else {
            NSLog("[FocusMe DNS] ALLOWED: \(domain)")
            forwardDnsQuery(datagram: datagram, endpoint: endpoint, flow: flow)
        }
    }

    /// Forward an allowed DNS query to the upstream resolver via a real UDP socket.
    private func forwardDnsQuery(datagram: Data, endpoint: NWEndpoint, flow: NEAppProxyUDPFlow) {
        let upstreamHost = "8.8.8.8"
        let upstreamPort: UInt16 = 53

        // Create a UDP socket to forward the query
        let sock = socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP)
        guard sock >= 0 else {
            NSLog("[FocusMe DNS] Failed to create UDP socket")
            return
        }
        defer { close(sock) }

        // Set receive timeout (5 seconds)
        var timeout = timeval(tv_sec: 5, tv_usec: 0)
        setsockopt(sock, SOL_SOCKET, SO_RCVTIMEO, &timeout, socklen_t(MemoryLayout<timeval>.size))

        // Build upstream address
        var addr = sockaddr_in()
        addr.sin_family = sa_family_t(AF_INET)
        addr.sin_port = upstreamPort.bigEndian
        inet_pton(AF_INET, upstreamHost, &addr.sin_addr)

        // Send DNS query
        let sent = datagram.withUnsafeBytes { rawBuf -> Int in
            withUnsafePointer(to: &addr) { addrPtr -> Int in
                addrPtr.withMemoryRebound(to: sockaddr.self, capacity: 1) { sockAddrPtr in
                    sendto(sock, rawBuf.baseAddress, datagram.count, 0, sockAddrPtr, socklen_t(MemoryLayout<sockaddr_in>.size))
                }
            }
        }

        if sent < 0 {
            NSLog("[FocusMe DNS] sendto error: \(errno)")
            return
        }

        // Receive response
        var responseBuf = [UInt8](repeating: 0, count: 1500)
        let recvLen = recv(sock, &responseBuf, responseBuf.count, 0)

        if recvLen > 0 {
            let responseData = Data(responseBuf[0..<recvLen])
            flow.writeDatagrams([responseData], sentBy: [endpoint]) { error in
                if let error = error {
                    NSLog("[FocusMe DNS] Write forward response error: \(error.localizedDescription)")
                }
            }
        }
    }

    // MARK: - Domain Blocking

    /// Update the set of blocked domains
    func updateBlockedDomains(_ domains: Set<String>) {
        lock.lock()
        blockedDomains = domains
        lock.unlock()
        NSLog("[FocusMe DNS] Updated blocked domains: \(domains.count) entries")
    }

    /// Check if a domain should be blocked
    func isDomainBlocked(_ domain: String) -> Bool {
        let normalizedDomain = domain.lowercased().trimmingCharacters(in: .init(charactersIn: "."))

        lock.lock()
        defer { lock.unlock() }

        // Exact match
        if blockedDomains.contains(normalizedDomain) {
            return true
        }

        // Check parent domains (e.g., "sub.reddit.com" blocked if "reddit.com" is blocked)
        var parts = normalizedDomain.split(separator: ".")
        while parts.count > 1 {
            parts.removeFirst()
            let parentDomain = parts.joined(separator: ".")
            if blockedDomains.contains(parentDomain) {
                return true
            }
        }

        return false
    }

    /// Extract the queried domain name from a raw DNS query datagram.
    ///
    /// DNS wire format:
    ///   Header (12 bytes): Transaction ID(2) + Flags(2) + QDCount(2) + ANCount(2) + NSCount(2) + ARCount(2)
    ///   Question: Name (length-prefixed labels terminated by 0x00) + QType(2) + QClass(2)
    private func extractDomainFromQuery(_ data: Data) -> String? {
        guard data.count >= 12 else { return nil } // Minimum DNS header

        let bytes = [UInt8](data)

        // Check QR bit = 0 (query)
        let flags = (UInt16(bytes[2]) << 8) | UInt16(bytes[3])
        if (flags & 0x8000) != 0 { return nil } // Not a query

        // QDCOUNT ≥ 1
        let qdCount = (UInt16(bytes[4]) << 8) | UInt16(bytes[5])
        if qdCount < 1 { return nil }

        // Parse question name starting at byte 12
        var offset = 12
        var labels: [String] = []

        while offset < bytes.count {
            let labelLen = Int(bytes[offset])
            offset += 1

            if labelLen == 0 { break } // Root label — end of name
            if labelLen > 63 { return nil } // Invalid or compressed (unexpected in queries)
            guard offset + labelLen <= bytes.count else { return nil }

            let label = String(bytes: bytes[offset..<(offset + labelLen)], encoding: .ascii) ?? ""
            labels.append(label)
            offset += labelLen
        }

        return labels.isEmpty ? nil : labels.joined(separator: ".").lowercased()
    }

    /// Synthesize an NXDOMAIN DNS response for a blocked domain.
    ///
    /// Constructs a valid DNS response:
    /// - Copies transaction ID from query
    /// - Sets QR=1, AA=1, RCODE=3 (NXDOMAIN)
    /// - Copies the question section verbatim
    /// - No answer/authority/additional sections
    private func synthesizeNXDOMAIN(queryData: Data) -> Data {
        guard queryData.count >= 12 else { return Data() }

        let queryBytes = [UInt8](queryData)

        // Find end of question section (past the name + 4 bytes for QTYPE+QCLASS)
        var offset = 12
        while offset < queryBytes.count {
            let labelLen = Int(queryBytes[offset])
            offset += 1
            if labelLen == 0 { break }
            if labelLen > 63 { return Data() }
            offset += labelLen
        }
        offset += 4 // QTYPE(2) + QCLASS(2)
        guard offset <= queryBytes.count else { return Data() }

        let questionSection = queryBytes[12..<offset]

        // Build response
        var response = [UInt8]()
        response.reserveCapacity(12 + questionSection.count)

        // Transaction ID — copy from query
        response.append(queryBytes[0])
        response.append(queryBytes[1])

        // Flags: QR=1, OPCODE=0, AA=1, TC=0, RD=1, RA=1, Z=0, RCODE=3 (NXDOMAIN)
        // Binary: 1 0000 1 0 1  1 000 0011 = 0x8583
        response.append(0x85)
        response.append(0x83)

        // QDCOUNT = 1
        response.append(0x00)
        response.append(0x01)

        // ANCOUNT = 0
        response.append(0x00)
        response.append(0x00)

        // NSCOUNT = 0
        response.append(0x00)
        response.append(0x00)

        // ARCOUNT = 0
        response.append(0x00)
        response.append(0x00)

        // Question section (copied verbatim)
        response.append(contentsOf: questionSection)

        return Data(response)
    }
}
