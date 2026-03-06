# ============================================================
# FILE:        proguard-rules.pro
# MODULE:      Android > ProGuard/R8 Shrink & Obfuscation Rules
# PLATFORM:    android
# ============================================================

# ---- FocusMe Services (must not be renamed — declared in AndroidManifest) ----
-keep class com.focusme.android.service.FocusMeVpnService { *; }
-keep class com.focusme.android.service.FocusMeAccessibilityService { *; }
-keep class com.focusme.android.service.FocusMeDaemonService { *; }
-keep class com.focusme.android.quota.QuotaTracker { *; }

# ---- Data classes used in JSON serialization ----
-keepclassmembers class com.focusme.android.service.FocusMeDaemonService$PlanData { *; }
-keepclassmembers class com.focusme.android.service.FocusMeDaemonService$PlanRule { *; }
-keepclassmembers class com.focusme.android.quota.QuotaTracker$Quota { *; }
-keepclassmembers class com.focusme.android.quota.QuotaTracker$UsageRecord { *; }
-keepclassmembers class com.focusme.android.quota.QuotaTracker$QuotaStatus { *; }

# ---- VPN packet parsing: don't strip DNS query data class ----
-keepclassmembers class com.focusme.android.service.FocusMeVpnService$DnsQuery { *; }

# ---- AndroidX / Jetpack ----
-dontwarn androidx.**
-keep class androidx.** { *; }

# ---- Kotlin ----
-dontwarn kotlin.**
-keep class kotlin.Metadata { *; }
-keepclassmembers class **$WhenMappings { <fields>; }

# ---- Standard Android ----
-keepattributes Signature
-keepattributes *Annotation*
-keepattributes EnclosingMethod
-keepattributes InnerClasses

# ---- Debugging: keep line numbers in stack traces ----
-keepattributes SourceFile,LineNumberTable
-renamesourcefileattribute SourceFile
