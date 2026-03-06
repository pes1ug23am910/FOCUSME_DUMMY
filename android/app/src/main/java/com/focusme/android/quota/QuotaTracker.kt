// ============================================================
// FILE:        QuotaTracker.kt
// MODULE:      Layer 2 — Plan & Scheduling Engine > Android Quota Tracker
// TASK:        T-043
// PLATFORM:    android
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Phase 3, Android quota tracking
// DEPENDENCIES: UsageStatsManager API, Room database
// TEST COVERAGE: Test: app usage decrements quota, triggers block at 0
// KNOWN LIMITATIONS: UsageStatsManager requires PACKAGE_USAGE_STATS permission
//                    (user must grant in Settings > Apps > Special access > Usage access).
//                    Minimum polling interval ~5s for usage stats.
//                    Real-time tracking uses AccessibilityService foreground detection.
// ============================================================

package com.focusme.android.quota

import android.app.usage.UsageStatsManager
import android.content.Context
import android.util.Log
import java.time.Duration
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId

/**
 * QuotaTracker — tracks per-app and per-domain usage time against
 * configured daily/weekly quotas.
 *
 * Quota types (from Section 2.2):
 * - daily_minutes: Max N minutes per day for an app/domain
 * - weekly_minutes: Max N minutes per week
 * - session_minutes: Max continuous session before break
 *
 * When quota is exhausted:
 * - App blocking: notifies AccessibilityService to block the app
 * - URL blocking: notifies VpnService to block the domain
 * - Resets at midnight local time (daily) or Monday (weekly)
 */
class QuotaTracker(private val context: Context) {

    companion object {
        private const val TAG = "FocusMeQuota"

        /** Maximum quota value in seconds (24h = 86400s, per policy schema) */
        const val MAX_QUOTA_SECONDS = 86400
    }

    // ---- Types ----

    /** A configured quota for an app or domain */
    data class Quota(
        val id: String,
        val planId: String,
        val target: String,             // Package name or domain
        val targetType: TargetType,
        val dailyLimitSeconds: Int?,    // Null = no daily limit
        val weeklyLimitSeconds: Int?,   // Null = no weekly limit
        val sessionLimitSeconds: Int?,  // Null = no session limit
    )

    enum class TargetType { APP, DOMAIN }

    /** Tracked usage for a quota target */
    data class UsageRecord(
        val target: String,
        val targetType: TargetType,
        val dailyUsedSeconds: Int = 0,
        val weeklyUsedSeconds: Int = 0,
        val currentSessionSeconds: Int = 0,
        val sessionStartedAt: Instant? = null,
        val lastUpdated: Instant = Instant.now(),
    )

    /** Quota status result */
    data class QuotaStatus(
        val target: String,
        val isExhausted: Boolean,
        val dailyRemaining: Int?,       // Seconds remaining, null if no daily quota
        val weeklyRemaining: Int?,
        val sessionRemaining: Int?,
        val reason: String?,            // Human-readable reason if exhausted
    )

    // ---- State ----

    /** Active quotas from plans */
    private val quotas = mutableListOf<Quota>()

    /** Current usage records */
    private val usageRecords = mutableMapOf<String, UsageRecord>()

    /** Lock for thread-safe access */
    private val lock = Any()

    // ---- Public API ----

    /**
     * Update the active quotas from plan configuration
     */
    fun updateQuotas(newQuotas: List<Quota>) {
        synchronized(lock) {
            quotas.clear()
            quotas.addAll(newQuotas)
        }
        Log.i(TAG, "Updated quotas: ${newQuotas.size} entries")
    }

    /**
     * Record usage time for an app or domain
     *
     * @param target Package name or domain
     * @param targetType APP or DOMAIN
     * @param durationSeconds Seconds of usage to record
     */
    fun recordUsage(target: String, targetType: TargetType, durationSeconds: Int) {
        synchronized(lock) {
            val record = usageRecords.getOrPut(target) {
                UsageRecord(target = target, targetType = targetType)
            }

            usageRecords[target] = record.copy(
                dailyUsedSeconds = record.dailyUsedSeconds + durationSeconds,
                weeklyUsedSeconds = record.weeklyUsedSeconds + durationSeconds,
                currentSessionSeconds = record.currentSessionSeconds + durationSeconds,
                lastUpdated = Instant.now(),
            )
        }
    }

    /**
     * Start a new usage session for an app or domain
     */
    fun startSession(target: String, targetType: TargetType) {
        synchronized(lock) {
            val record = usageRecords.getOrPut(target) {
                UsageRecord(target = target, targetType = targetType)
            }

            usageRecords[target] = record.copy(
                currentSessionSeconds = 0,
                sessionStartedAt = Instant.now(),
            )
        }
        Log.d(TAG, "Session started: $target")
    }

    /**
     * End the current usage session for an app or domain
     */
    fun endSession(target: String) {
        synchronized(lock) {
            val record = usageRecords[target] ?: return
            usageRecords[target] = record.copy(
                currentSessionSeconds = 0,
                sessionStartedAt = null,
            )
        }
        Log.d(TAG, "Session ended: $target")
    }

    /**
     * Check quota status for a target
     *
     * @return QuotaStatus with remaining time and exhaustion flag
     */
    fun checkQuota(target: String): QuotaStatus {
        synchronized(lock) {
            val quota = quotas.find { it.target == target }
                ?: return QuotaStatus(
                    target = target,
                    isExhausted = false,
                    dailyRemaining = null,
                    weeklyRemaining = null,
                    sessionRemaining = null,
                    reason = null,
                )

            val record = usageRecords[target]
                ?: return QuotaStatus(
                    target = target,
                    isExhausted = false,
                    dailyRemaining = quota.dailyLimitSeconds,
                    weeklyRemaining = quota.weeklyLimitSeconds,
                    sessionRemaining = quota.sessionLimitSeconds,
                    reason = null,
                )

            // Calculate remaining time
            val dailyRemaining = quota.dailyLimitSeconds?.let {
                maxOf(0, it - record.dailyUsedSeconds)
            }
            val weeklyRemaining = quota.weeklyLimitSeconds?.let {
                maxOf(0, it - record.weeklyUsedSeconds)
            }
            val sessionRemaining = quota.sessionLimitSeconds?.let {
                maxOf(0, it - record.currentSessionSeconds)
            }

            // Determine if exhausted
            val isExhausted = (dailyRemaining != null && dailyRemaining <= 0) ||
                (weeklyRemaining != null && weeklyRemaining <= 0) ||
                (sessionRemaining != null && sessionRemaining <= 0)

            val reason = when {
                dailyRemaining != null && dailyRemaining <= 0 -> "Daily quota exhausted"
                weeklyRemaining != null && weeklyRemaining <= 0 -> "Weekly quota exhausted"
                sessionRemaining != null && sessionRemaining <= 0 -> "Session limit reached — take a break"
                else -> null
            }

            return QuotaStatus(
                target = target,
                isExhausted = isExhausted,
                dailyRemaining = dailyRemaining,
                weeklyRemaining = weeklyRemaining,
                sessionRemaining = sessionRemaining,
                reason = reason,
            )
        }
    }

    /**
     * Reset daily quotas — call at midnight local time
     */
    fun resetDailyQuotas() {
        synchronized(lock) {
            for ((key, record) in usageRecords) {
                usageRecords[key] = record.copy(dailyUsedSeconds = 0)
            }
        }
        Log.i(TAG, "Daily quotas reset")
    }

    /**
     * Reset weekly quotas — call at Monday midnight local time
     */
    fun resetWeeklyQuotas() {
        synchronized(lock) {
            for ((key, record) in usageRecords) {
                usageRecords[key] = record.copy(weeklyUsedSeconds = 0)
            }
        }
        Log.i(TAG, "Weekly quotas reset")
    }

    // ---- UsageStatsManager Integration ----

    /**
     * Sync usage data from Android's UsageStatsManager.
     * Used as a supplementary/cross-check signal alongside AccessibilityService events.
     *
     * This polls the last 5 minutes of foreground usage and reconciles with our
     * internal tracking. If UsageStatsManager reports more time than we've tracked,
     * we treat the difference as untracked usage (e.g. from before service was running).
     */
    fun syncWithUsageStats() {
        try {
            val usageStatsManager = context.getSystemService(Context.USAGE_STATS_SERVICE)
                as? UsageStatsManager
            if (usageStatsManager == null) {
                Log.w(TAG, "UsageStatsManager not available")
                return
            }

            val endTime = System.currentTimeMillis()
            // Query today's usage from midnight local time
            val todayStart = LocalDate.now()
                .atStartOfDay(ZoneId.systemDefault())
                .toInstant()
                .toEpochMilli()

            val stats = usageStatsManager.queryUsageStats(
                UsageStatsManager.INTERVAL_DAILY, todayStart, endTime
            )

            if (stats.isNullOrEmpty()) {
                Log.d(TAG, "No usage stats available (permission may not be granted)")
                return
            }

            synchronized(lock) {
                // Get all app targets we're tracking
                val appQuotaTargets = quotas
                    .filter { it.targetType == TargetType.APP }
                    .map { it.target }
                    .toSet()

                for (stat in stats) {
                    val packageName = stat.packageName
                    if (packageName !in appQuotaTargets) continue

                    val systemReportedSeconds =
                        (stat.totalTimeInForeground / 1000).toInt()

                    val record = usageRecords[packageName]
                    if (record != null) {
                        // If system reports more usage than we tracked, bump ours up
                        if (systemReportedSeconds > record.dailyUsedSeconds) {
                            val delta = systemReportedSeconds - record.dailyUsedSeconds
                            Log.d(TAG, "UsageStats correction for $packageName: +${delta}s")
                            usageRecords[packageName] = record.copy(
                                dailyUsedSeconds = systemReportedSeconds,
                                weeklyUsedSeconds = record.weeklyUsedSeconds + delta,
                                lastUpdated = Instant.now(),
                            )
                        }
                    } else if (systemReportedSeconds > 0) {
                        // We weren't tracking this app yet but system shows usage
                        usageRecords[packageName] = UsageRecord(
                            target = packageName,
                            targetType = TargetType.APP,
                            dailyUsedSeconds = systemReportedSeconds,
                            weeklyUsedSeconds = systemReportedSeconds,
                            lastUpdated = Instant.now(),
                        )
                    }
                }
            }

            Log.d(TAG, "UsageStats sync complete")
        } catch (e: SecurityException) {
            Log.w(TAG, "UsageStats permission not granted", e)
        } catch (e: Exception) {
            Log.e(TAG, "UsageStats sync error", e)
        }
    }
}
