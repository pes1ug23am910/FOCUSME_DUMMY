# FocusMe — Chrome Web Store Submission Prep

> **FILE:** docs/store_submissions/chrome_web_store.md
> **TASK:** T-056
> **AUTHOR:** FocusMe Co-Pilot (Claude Opus)
> **SESSION:** 5
> **PURPOSE:** Complete store listing copy, permission justifications, and submission checklist for Chrome Web Store.

---

## Store Listing

### Extension Name
FocusMe — Website & App Blocker

### Short Description (max 132 characters)
Block distracting websites on a schedule. Enforced blocking with native daemon — no easy bypass. Forced Mode for deep focus.

### Detailed Description
FocusMe is a powerful website blocker that helps you stay focused by blocking distracting websites on a customizable schedule. Unlike browser-only blockers, FocusMe uses a native daemon service that enforces blocking at the DNS and network level — making it significantly harder to circumvent.

**Key Features:**
• Block websites by domain, URL pattern, or keyword
• Schedule blocks by day of week and time range
• Forced Mode — lock yourself into a focus session with no easy undo
• Element hiding — remove distracting page elements (comment sections, feeds, sidebars)
• Usage statistics — track your browsing habits and focus time
• Cross-browser support — works in Chrome, Firefox, and Edge
• Native enforcement — blocking works even if you disable the extension

**How It Works:**
FocusMe consists of a browser extension (for UI and block pages) and a native daemon service (for enforcement). The browser extension communicates with the daemon via Native Messaging to provide tamper-resistant blocking that cannot be bypassed by simply disabling the extension.

**Plans & Scheduling:**
Create multiple blocking plans with different schedules. Each plan can block specific websites during work hours, study time, or any custom schedule. Plans support daily time limits (quotas) and Forced Mode sessions.

**Privacy:**
FocusMe operates entirely locally. No browsing data is sent to external servers. All blocking rules and statistics are stored on your device. See our privacy policy for details.

### Category
Productivity

### Language
English

### Privacy Policy URL
`[PLACEHOLDER — requires live URL before submission]`

### Website
`[PLACEHOLDER — focusme.com]`

---

## Permission Justifications

Chrome Web Store requires justification for each permission requested. These are reviewed by the Chrome team.

### `declarativeNetRequest` — Justification

> **Purpose:** FocusMe uses the `declarativeNetRequest` API to block navigation to user-specified distracting websites. When a user creates a blocking plan that includes specific domains (e.g., facebook.com, reddit.com), FocusMe converts those domains into declarativeNetRequest rules that redirect matching navigations to an internal block page.
>
> **Why this permission is necessary:** This is the core functionality of the extension — website blocking. Without this permission, FocusMe cannot prevent the user from navigating to blocked websites. The rules are dynamically managed based on the user's blocking schedule and are updated when plans activate or deactivate.
>
> **Data handling:** No browsing data is collected or transmitted. The rules are derived from the user's own blocking configuration and are stored locally via `chrome.storage.sync`.

### `nativeMessaging` — Justification

> **Purpose:** FocusMe uses Native Messaging to communicate with a locally-installed daemon service (`focusme-daemon`) that provides tamper-resistant enforcement. The extension sends plan status queries and receives blocking rules from the daemon.
>
> **Why this permission is necessary:** Browser-only blockers can be trivially bypassed by disabling the extension. By communicating with a native daemon, FocusMe provides enforcement at the DNS and network level that persists even if the extension is temporarily disabled. The native component is installed separately by the user.
>
> **Data handling:** Messages between the extension and daemon contain only plan configurations, blocked domain lists, and status information. No personal browsing data is transmitted. All communication is local (no network requests).

### `storage` — Justification

> **Purpose:** FocusMe uses `chrome.storage.sync` to persist user preferences, cached blocking rules, and element hiding selectors. This enables settings to sync across devices where the user is signed into Chrome.
>
> **Data handling:** Only FocusMe configuration data is stored (plan names, domain lists, UI preferences). No browsing history, passwords, or personal data is stored.

### `tabs` — Justification

> **Purpose:** FocusMe uses the `tabs` API to redirect the current tab to a block page when a BLOCK response is received from the native daemon. This handles cases where `declarativeNetRequest` redirect is not applicable (e.g., the page was already loaded before rules were updated).
>
> **Data handling:** FocusMe only reads the current tab URL to check against active blocking rules. No tab data is collected, stored, or transmitted.

### `alarms` — Justification

> **Purpose:** FocusMe uses `chrome.alarms` to periodically sync blocking rules with the native daemon (every 30 seconds). This ensures rules stay current when plans activate/deactivate on schedule.
>
> **Data handling:** No data is collected. Alarms are used solely for scheduling internal sync operations.

### Host Permissions — `<all_urls>` Justification (if content scripts)

> **Purpose:** The `element_blocker.ts` content script needs to run on all pages to hide user-specified distracting elements (e.g., YouTube comments, Twitter sidebar). The script only operates on pages that match the user's element blocking rules.
>
> **Data handling:** The content script reads DOM elements to apply CSS `visibility: hidden` to user-specified selectors. No page content is read, collected, or transmitted. The script does not access form data, passwords, or any user input.

---

## Screenshots Specification

Chrome Web Store requires 1-5 screenshots. Recommended size: 1280×800 or 640×400.

| # | Content | Description Text |
|---|---------|-----------------|
| 1 | Extension popup showing connected status + active plan | "FocusMe popup — see your active plans and blocking status at a glance" |
| 2 | Block page (blocked.html) shown when visiting a blocked site | "Clean block page when you try to visit a distraction during focus time" |
| 3 | Plan wizard showing schedule configuration | "Create custom blocking plans with flexible scheduling" |
| 4 | Stats page showing daily focus time chart | "Track your focus time and browsing habits with detailed statistics" |
| 5 | Element blocker in action (YouTube without comments) | "Hide distracting page elements like comment sections and sidebars" |

**Screenshot generation:**
- Use Chrome DevTools device toolbar for consistent dimensions
- Dark mode variation recommended for screenshot 1 or 2
- Use realistic-looking plan data (not test/debug data)

---

## Single Purpose Description

> FocusMe blocks distracting websites and apps on a user-defined schedule to help users maintain focus and productivity. It enforces blocking through a native daemon service that operates at the network level, making it significantly harder to circumvent than browser-only solutions.

---

## Submission Checklist

- [ ] Extension package (.zip) built and tested
- [ ] All icons present (16×16, 48×48, 128×128 PNG)
- [ ] Privacy policy hosted at live URL
- [ ] All permission justifications reviewed for accuracy
- [ ] Screenshots captured at correct dimensions
- [ ] "Single purpose" description under 500 characters
- [ ] No minified/obfuscated code (CWS requires readable source)
- [ ] Source code ZIP prepared (if CWS requests it)
- [ ] Developer account verified ($5 registration fee)
- [ ] Extension ID noted: `EXTENSION_ID_HERE` → update NMH manifest after publishing

---

## Review Timeline

Chrome Web Store review typically takes 1-3 business days for new submissions.
Extensions with `nativeMessaging` and `declarativeNetRequest` may receive additional scrutiny.
Prepare a detailed response to potential reviewer questions about native messaging usage.

---

*Generated Session 5. Complete all checklist items before submitting.*
