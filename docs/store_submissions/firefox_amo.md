# FocusMe — Firefox AMO (addons.mozilla.org) Submission Prep

> **FILE:** docs/store_submissions/firefox_amo.md
> **TASK:** T-056
> **AUTHOR:** FocusMe Co-Pilot (Claude Opus)
> **SESSION:** 5
> **PURPOSE:** Complete store listing, permission justifications, and AMO-specific requirements for Firefox Add-ons submission.

---

## Store Listing

### Add-on Name
FocusMe — Website & App Blocker

### Summary (max 250 characters)
Block distracting websites on a schedule with enforced, tamper-resistant blocking. Uses a native daemon for DNS-level enforcement that works even if the extension is disabled.

### Description
FocusMe is a website blocker that enforces blocking at the DNS and network level through a native daemon service. Unlike browser-only blockers, FocusMe prevents circumvention by blocking distracting websites before they even resolve in your browser.

**Features:**
• Domain, URL pattern, and keyword blocking
• Day-of-week and time-range scheduling
• Forced Mode — lock into focus sessions with Argon2id-protected unlock
• Element hiding — remove specific page elements (feeds, comments, sidebars)
• Usage statistics and focus time tracking
• Cross-browser support (Firefox, Chrome, Edge)
• Native daemon enforcement — blocking persists even without the extension

**How It Works:**
The FocusMe browser extension provides the user interface (popup, block pages, element hiding) while a locally-installed daemon service handles enforcement at the DNS layer. Communication between the extension and daemon uses Firefox's Native Messaging API.

**Privacy First:**
All data stays on your device. No browsing data is sent to external servers. The extension communicates only with the local daemon — never with remote servers.

### Categories
- Productivity
- Privacy & Security

### Tags
focus, blocker, website blocker, distraction blocker, productivity, time management, parental controls

### License
Proprietary

### Privacy Policy URL
`[PLACEHOLDER — requires live URL before submission]`

### Homepage URL
`[PLACEHOLDER — focusme.com]`

### Support URL
`[PLACEHOLDER — focusme.com/support]`

---

## Permission Justifications

Firefox AMO is particularly strict about `webRequest` and `webRequestBlocking` permissions. Provide thorough justification.

### `webRequest` + `webRequestBlocking` — Justification

> **Purpose:** FocusMe uses `webRequest` and `webRequestBlocking` (MV2) to intercept and cancel navigation requests to user-specified blocked domains. When a user creates a blocking plan, all HTTP/HTTPS requests to those domains are intercepted in `onBeforeRequest` and redirected to an internal block page.
>
> **Why this is necessary:** Firefox MV2 does not have `declarativeNetRequest`. The `webRequest` API with blocking capability is the only mechanism to prevent page loads before they complete. Without this, the extension cannot enforce website blocking.
>
> **Data handling:** The extension only examines the request URL to check against the user's blocking rules. No request headers, bodies, or response data are read, modified, collected, or transmitted. The blocking decision is a simple domain membership check against a locally-stored set.
>
> **Scope limitation:** The extension only blocks domains explicitly listed in the user's blocking plans. It does not inspect, modify, or log any traffic to non-blocked domains.

### `nativeMessaging` — Justification

> **Purpose:** FocusMe communicates with a locally-installed daemon service (`focusme-daemon`) via Firefox's Native Messaging API. This provides tamper-resistant blocking at the DNS/network level that persists even if the extension is disabled.
>
> **Why this is necessary:** Browser-only blockers are trivially bypassed. The native daemon provides enforcement at a lower level of the networking stack. Communication is strictly local — no network requests are made.
>
> **Data exchanged:** Plan configurations, blocked domain lists, status queries. No personal browsing data is transmitted.

### `<all_urls>` — Justification

> **Purpose:** The element blocker content script (`element_blocker.ts`) needs to run on all pages to hide user-specified distracting page elements. It only activates on pages matching the user's element blocking rules.
>
> **Data handling:** The script applies CSS `visibility: hidden` to elements matching user-specified CSS selectors. It does not read, collect, or transmit any page content, form data, or user input.

### `storage` — Justification

> **Purpose:** Stores user preferences, cached blocking rules, and element hiding selectors locally. No data is synced to external servers.

### `tabs` — Justification

> **Purpose:** Redirects the active tab to a block page when a blocked navigation is detected. Does not read or collect tab content.

---

## Source Code Disclosure

Firefox AMO requires source code to be available for review if the submitted add-on contains minified, obfuscated, or compiled code.

### Source Code Policy

FocusMe's Firefox extension is built from TypeScript source using webpack. The submitted `.xpi` contains compiled JavaScript that is NOT human-readable.

**Source code submission requirements:**
1. Provide a ZIP of the full `extension/` source directory
2. Include `package.json`, `tsconfig.json`, `webpack.config.js`
3. Include build instructions:
   ```bash
   cd extension/
   npm install
   npm run build:firefox
   ```
4. The reviewer can verify that the built output matches the submitted `.xpi`

**Build reproducibility:**
- Node.js 18+ required
- `npm ci` for deterministic installs
- webpack produces deterministic output with same Node version

---

## Self-Distribution vs AMO-Hosted

### Decision: AMO-Hosted (recommended)

| Factor | AMO-Hosted | Self-Distributed |
|--------|-----------|-----------------|
| Auto-updates | ✅ AMO handles | ❌ Must host update manifest |
| User trust | ✅ AMO badge | ❌ "Unknown publisher" warning |
| Review time | 🟡 1-5 days | ✅ Immediate |
| Install flow | ✅ Standard AMO page | ❌ Manual .xpi sideload |
| MV2 support | ✅ AMO supports MV2 | ✅ Same |

**Decision:** Use AMO-hosted distribution for public users. Self-distribution only for enterprise deployments where IT controls the install.

---

## Screenshots Specification

AMO supports up to 10 screenshots. Recommended: 1280×800 or equivalent.

| # | Content | Caption |
|---|---------|---------|
| 1 | Extension popup with active plan | "Quick status view from the toolbar" |
| 2 | Block page when visiting blocked site | "Clean block page during focus time" |
| 3 | Plan wizard with schedule | "Flexible scheduling — block by day and time" |
| 4 | Stats page with charts | "Track your focus time trends" |

---

## AMO-Specific Considerations

1. **MV2 support:** Firefox continues to support MV2 with no announced deprecation date. FocusMe uses MV2 for `webRequestBlocking` which has no MV3 equivalent in Firefox yet.

2. **Recommended extensions program:** Apply for Mozilla's "Recommended Extensions" badge after initial launch + user reviews. Requirements: security review passes, responsive support, regular updates.

3. **Content Security Policy:** Ensure manifest includes appropriate CSP. No `eval()`, no inline scripts, no remote code loading.

4. **Firefox for Android:** The extension manifest should declare `"applications.gecko.strict_min_version"` to control mobile support. FocusMe is desktop-only for Firefox.

---

## Submission Checklist

- [ ] `.xpi` file built and tested on Firefox ESR, Release, and Beta
- [ ] Source code ZIP prepared with build instructions
- [ ] All icons present (48×48, 128×128 SVG or PNG)
- [ ] Privacy policy hosted at live URL
- [ ] All permission justifications drafted above reviewed by team
- [ ] Screenshots captured
- [ ] `manifest.v2.json` includes `browser_specific_settings.gecko.id`
- [ ] CSP policy reviewed — no unsafe-eval, no remote code
- [ ] Firefox Developer Hub account created
- [ ] Self-distribution signing key generated (for enterprise builds only)

---

*Generated Session 5. Complete all checklist items before submitting to AMO.*
