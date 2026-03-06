# FocusMe — Google Play Store Submission Prep

> **FILE:** docs/store_submissions/google_play.md
> **TASK:** T-057
> **AUTHOR:** FocusMe Co-Pilot (Claude Opus)
> **SESSION:** 5
> **PURPOSE:** Complete Play Store listing, AccessibilityService declaration (S-007), data safety form, and submission requirements.

---

## App Listing

### App Name (max 30 characters)
FocusMe - Focus & App Blocker

### Short Description (max 80 characters)
Block distracting apps & websites on a schedule. Enforced focus with no easy undo.

### Full Description (max 4000 characters)
FocusMe helps you stay focused by blocking distracting apps and websites on a customizable schedule. Set up blocking plans, enable Forced Mode for deep focus sessions, and track your screen time — all locally on your device with no data sent to the cloud.

🛡️ ENFORCED BLOCKING
Unlike simple timers, FocusMe enforces blocking at the system level. Apps are blocked via Android's AccessibilityService, and websites are blocked via a local VPN that filters DNS requests. No easy workaround — that's the point.

📅 FLEXIBLE SCHEDULING
Create multiple blocking plans with different schedules. Block social media during work hours (9am-5pm, Mon-Fri), gaming apps during study time, or anything else. Each plan has its own schedule, app list, and website list.

⏱️ DAILY TIME LIMITS (QUOTAS)
Set daily time limits instead of full blocks. Allow 30 minutes of Instagram per day, then FocusMe blocks it for the rest of the day. Quotas reset at midnight.

🔒 FORCED MODE
Lock yourself into a focus session for a set duration. During Forced Mode, you cannot disable blocking or modify your plans. Emergency unlock requires a complex password (Argon2id verified) — designed to be inconvenient enough to deter impulsive disabling.

📊 USAGE STATISTICS
Track your daily app usage and focus time. See which apps consume the most time and how your focus habits trend over weeks.

🌐 DNS-LEVEL WEBSITE BLOCKING
FocusMe uses a local VPN service to intercept DNS queries and block distracting websites. This works across all browsers — Chrome, Firefox, Samsung Internet, and any other app that loads web content.

🔒 PRIVACY FIRST
• All data stays on your device — nothing is sent to external servers
• No ads, no tracking, no analytics
• No account required
• VPN service is local only — your traffic is NOT routed through any remote server

⚙️ PERMISSIONS EXPLAINED
• Accessibility Service: Detects which app is in the foreground to enforce app blocking
• VPN Service: Creates a local-only VPN to filter DNS queries for website blocking
• Usage Stats: Tracks daily app usage time for quota enforcement
• Display Over Other Apps: Shows a blocking overlay when a blocked app is opened
• Boot Completed: Restarts blocking on device reboot

📱 REQUIREMENTS
• Android 12 or later
• Accessibility Service must be enabled for app blocking
• VPN permission required for website blocking

### Category
Productivity

### Content Rating
Everyone (IARC questionnaire — no violence, no user-generated content, no gambling)

### Target Audience
18+ (focus/productivity tool, not designed for children)

### Contact Email
`[PLACEHOLDER — support@focusme.com]`

### Privacy Policy URL
`[PLACEHOLDER — requires live URL before submission]`

---

## AccessibilityService Declaration (S-007)

Google Play requires explicit justification when an app uses `AccessibilityService`. Apps that don't meet the policy criteria are rejected.

### Policy Compliance Statement

> **FocusMe uses Android's AccessibilityService solely to detect the currently foreground application package name for the purpose of enforcing user-configured app blocking schedules.**
>
> Specifically, the AccessibilityService:
>
> 1. **Listens for `TYPE_WINDOW_STATE_CHANGED` events only** — no other accessibility event types are consumed
> 2. **Reads only `event.packageName`** — the package name of the foreground app
> 3. **Does NOT read screen content** — no `getText()`, `getContentDescription()`, or node tree traversal
> 4. **Does NOT intercept or modify user input** — no key logging, gesture capture, or input injection
> 5. **Does NOT interact with other apps' UI elements** — no clicking, scrolling, or form filling
> 6. **Sole purpose:** Compare the foreground app's package name against the user's blocking plan. If the app is blocked, display a full-screen overlay informing the user
>
> **Why AccessibilityService is necessary:**
> Android does not provide a public API for detecting the foreground app that is reliable across all Android versions (12+). `UsageStatsManager.queryUsageStats()` is used for historical data but has a delay that makes it unsuitable for real-time blocking. `AccessibilityService` with `TYPE_WINDOW_STATE_CHANGED` is the only reliable mechanism for immediate foreground app detection.
>
> **User control:**
> - The user explicitly enables the AccessibilityService in Android Settings
> - The user creates all blocking plans — the app does not block anything by default
> - The user can disable the AccessibilityService at any time (outside of Forced Mode)
>
> **Core functionality:**
> App blocking IS the core functionality of FocusMe. The AccessibilityService is not a secondary feature — it is essential to the app's primary purpose as published in the store listing.

### AccessibilityService Metadata (from AndroidManifest.xml)

```xml
<meta-data
    android:name="android.accessibilityservice"
    android:resource="@xml/accessibility_service_config" />
```

```xml
<!-- res/xml/accessibility_service_config.xml -->
<accessibility-service
    xmlns:android="http://schemas.android.com/apk/res/android"
    android:description="@string/accessibility_service_description"
    android:accessibilityEventTypes="typeWindowStateChanged"
    android:accessibilityFeedbackType="feedbackGeneric"
    android:notificationTimeout="100"
    android:canRetrieveWindowContent="false"
    android:settingsActivity="com.focusme.android.ui.screens.SettingsActivity" />
```

**Key points for review:**
- `accessibilityEventTypes` = `typeWindowStateChanged` ONLY (minimum scope)
- `canRetrieveWindowContent` = `false` (explicitly opted out)
- Clear `description` string explaining the purpose to the user

---

## Data Safety Section

Google Play requires a Data Safety form. Fill in the Developer Console with these values.

### Data Collected

| Data Type | Collected? | Purpose | Shared? | Required? |
|-----------|-----------|---------|---------|-----------|
| App activity (app usage time) | Yes — stored locally only | App functionality (quota tracking) | No | Yes (for quotas) |
| Device identifiers | No | — | — | — |
| Location | No | — | — | — |
| Personal info (name, email) | No | — | — | — |
| Financial info | No | — | — | — |
| Health & fitness | No | — | — | — |
| Messages | No | — | — | — |
| Photos/videos | No | — | — | — |
| Audio | No | — | — | — |
| Files & docs | No | — | — | — |
| Calendar | No | — | — | — |
| Contacts | No | — | — | — |
| Web browsing history | No | — | — | — |
| Search history | No | — | — | — |
| Crash logs | Future (PostHog) | Analytics & diagnostics | No | No |

### Data Handling Practices

- **Data encrypted in transit:** N/A — no data is transmitted to external servers
- **Data encrypted at rest:** Yes — SQLCipher encrypted database (AES-256)
- **Users can request data deletion:** Yes — Settings → Privacy → Delete All Data
- **Data retention:** All data stored locally, deleted on app uninstall
- **Independent security review:** Planned (see docs/security_review.md)

### Data Safety Declaration Text

> FocusMe does not collect, transmit, or share any personal data with external servers. All app usage data (for quota tracking) is stored locally on your device in an encrypted database. No accounts are required. The local VPN service does not route traffic through any remote server — all DNS filtering happens on-device. You can delete all data at any time from Settings → Privacy → Delete All Data, or by uninstalling the app.

---

## Content Rating Questionnaire

| Question | Answer |
|----------|--------|
| Violence | No |
| Sexual content | No |
| Language | No |
| Controlled substance | No |
| Gambling | No |
| User-generated content | No |
| Account creation | No |
| Data sharing | No |
| Ads | No |
| In-app purchases | Future (subscription — not at launch) |
| Location-based services | No |

**Expected rating:** IARC 3+ / ESRB Everyone / PEGI 3

---

## Store Assets Specification

### App Icon
- 512×512 PNG, 32-bit color, no transparency
- Design: shield or focus ring shape in brand colors
- No text on icon (Google Play policy for icons under 1024px)

### Feature Graphic
- 1024×500 PNG or JPEG
- Content: app name + tagline + device mockup showing block page

### Screenshots (phone)
- 2-8 screenshots, recommended 1080×1920 (16:9 portrait)

| # | Content |
|---|---------|
| 1 | Plan list screen with 2-3 active plans |
| 2 | Block overlay appearing over a social media app |
| 3 | Plan editor with schedule configuration |
| 4 | Stats page showing daily usage chart |
| 5 | Settings screen showing enabled permissions |
| 6 | Forced Mode active countdown timer |

### Screenshots (tablet — optional)
- Same content at 1920×1200 or similar tablet resolution

---

## Submission Checklist

- [ ] Signed APK/AAB with release keystore
- [ ] `versionCode` and `versionName` set correctly in build.gradle
- [ ] `minSdk = 31` (Android 12)
- [ ] `targetSdk = 34` (latest)
- [ ] ProGuard rules tested — no runtime crashes
- [ ] Privacy policy hosted at live URL
- [ ] AccessibilityService declaration written (above)
- [ ] Data safety form filled in Developer Console
- [ ] Content rating questionnaire completed
- [ ] App icon 512×512 PNG created
- [ ] Feature graphic 1024×500 created
- [ ] Phone screenshots captured (min 2, max 8)
- [ ] `accessibility_service_config.xml` reviewed for minimum scope
- [ ] VPN service declared with `BIND_VPN_SERVICE` permission
- [ ] `QUERY_ALL_PACKAGES` justified (foreground app detection backup)
- [ ] Internal testing track → open testing → production rollout
- [ ] Google Play Developer account ($25 registration fee)

---

## Review Timeline

- **Internal testing:** Available immediately after upload
- **Closed/Open testing:** Review takes 1-3 days
- **Production:** Review takes 1-7 days (AccessibilityService apps may take longer)
- **AccessibilityService apps:** Google may request a video demo of the accessibility feature

### Prepare Video Demo
Record a 1-2 minute screen recording showing:
1. Enabling the AccessibilityService in Settings
2. Creating a blocking plan (block Instagram)
3. Switching to Instagram → overlay appears
4. Demonstrating that the service ONLY detects the foreground app name
5. Showing that `canRetrieveWindowContent="false"` in the config

---

*Generated Session 5. Resolves S-007 (AccessibilityService justification). Complete all checklist items before submitting to Google Play.*
