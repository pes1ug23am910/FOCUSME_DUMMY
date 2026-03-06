# FocusMe — Store Submission Go/No-Go Checklist

> **Purpose:** Single document a PM reviews before pressing "submit" on any store.
> **Repo path:** `docs/store_submissions/SUBMISSION_CHECKLIST.md`
> **Last updated:** Session 9

---

## Hard Gates (ALL must be ✅ before submitting to ANY store)

| # | Gate | Owner | Status | Notes |
|---|------|-------|--------|-------|
| G-01 | EV Code Signing cert procured ([BLOCKED T-003]) | DevOps | 🔲 | DigiCert/Sectigo. 2-5 day identity verification. |
| G-02 | Windows MSI signed with EV cert | DevOps | 🔲 | `installer_checklist.md` W-03/W-04 must PASS |
| G-03 | Apple ESF entitlement approved ([BLOCKED T-002]) | DevOps | 🔲 | developer.apple.com → System Extensions. 1-7 week wait. |
| G-04 | macOS PKG notarized | DevOps | 🔲 | `installer_checklist.md` M-01 must PASS |
| G-05 | Legal counsel review complete ([BLOCKED T-007]) | Legal | 🔲 | privacy_policy.md, tos.md, eula.md reviewed and approved |
| G-06 | Privacy policy URL live at public URL | Engineering | 🔲 | Required by CWS, AMO, and Google Play policies |
| G-07 | Android APK/AAB signed with release keystore | Engineering | 🔲 | Release keystore backed up securely |
| G-08 | All `installer_checklist.md` items marked ✅ PASS | QA | 🔲 | Windows (W-01–W-08), macOS (M-01–M-06), Linux (L-01–L-05) |
| G-09 | CI green on `main` branch (all jobs passing) | DevOps | 🔲 | daemon, extension, android, backend, security jobs |
| G-10 | Version bumped in all manifests | Engineering | 🔲 | Cargo.toml, package.json, tauri.conf.json, build.gradle.kts, manifest.json (v3+v2) |
| G-11 | CHANGELOG.md current version section complete | Engineering | 🔲 | All user-facing changes documented under `## [x.y.z]` heading |

---

## Platform-Specific Pre-Submission

### Chrome Web Store (T-056)

| # | Check | Status | Notes |
|---|-------|--------|-------|
| C-01 | Extension icon PNGs exported (16px, 48px, 128px) from SVG | 🔲 | `rsvg-convert` per icons/README.md |
| C-02 | Privacy policy URL set in `chrome_web_store.md` | 🔲 | Must match G-06 URL |
| C-03 | Extension `.zip` built from webpack production build | 🔲 | `npm run build` in extension/ |
| C-04 | Single purpose description reviewed and finalized | 🔲 | Max 132 chars for short description |
| C-05 | Developer account payment / identity verified | 🔲 | One-time $5 registration fee |
| C-06 | Screenshots (1280×800) prepared for listing | 🔲 | At least 1, recommended 5 |

### Firefox AMO (T-056)

| # | Check | Status | Notes |
|---|-------|--------|-------|
| F-01 | Privacy policy URL set in `firefox_amo.md` | 🔲 | Must match G-06 URL |
| F-02 | Source code `.zip` prepared for AMO review | 🔲 | AMO requires full source for review |
| F-03 | `.xpi` package built and verified | 🔲 | `web-ext build` in extension/ |
| F-04 | Add-on description and categories finalized | 🔲 | Match CWS listing language |

### Google Play (T-057)

| # | Check | Status | Notes |
|---|-------|--------|-------|
| P-01 | Privacy policy URL set in `google_play.md` | 🔲 | Must match G-06 URL |
| P-02 | AAB (Android App Bundle) built from release keystore | 🔲 | `./gradlew bundleRelease` |
| P-03 | AccessibilityService justification reviewed by Play Policy | 🔲 | Requires pre-approval due to sensitive permissions |
| P-04 | Data safety form matches actual app behavior | 🔲 | Review against PostHog analytics events in analytics.rs |
| P-05 | IARC content rating completed | 🔲 | Questionnaire in Play Console |
| P-06 | Minimum SDK version correctly set (API 26+) | 🔲 | Verify in build.gradle.kts |
| P-07 | App signing by Google Play configured (or upload key set) | 🔲 | Recommended: Google manages signing key |

---

## Pre-Submission QA Smoke Test

Run these on real hardware before ANY store submission:

| # | Test | Platform | Status |
|---|------|----------|--------|
| S-01 | Fresh install → plan wizard → block site → verify blocked | Windows | 🔲 |
| S-02 | Fresh install → plan wizard → block site → verify blocked | macOS | 🔲 |
| S-03 | Fresh install → plan wizard → block site → verify blocked | Linux | 🔲 |
| S-04 | Fresh install → plan wizard → block site → verify blocked | Android | 🔲 |
| S-05 | Extension popup shows correct state after plan activation | Chrome | 🔲 |
| S-06 | Extension popup shows correct state after plan activation | Firefox | 🔲 |
| S-07 | Forced mode prevents deactivation during lockout | All | 🔲 |
| S-08 | Cloud sync: create plan on Device A → verify on Device B | All | 🔲 |
| S-09 | Family invite flow: owner invites → member accepts → plan shared | Cloud | 🔲 |
| S-10 | Upgrade path: install v0.0.x → upgrade to release → data preserved | Windows | 🔲 |

---

## Sign-Off

> **RELEASE IS BLOCKED** until ALL hard gates (G-01–G-11) are ✅ **AND** all
> signatories below have approved.

| Role | Name | Date | All Hard Gates Pass? |
|------|------|------|----------------------|
| Product Manager | _________________ | ________ | ☐ Yes |
| DevOps / Release | _________________ | ________ | ☐ Yes |
| Security Lead | _________________ | ________ | ☐ Yes |
| Legal | _________________ | ________ | ☐ Yes |

---

## Submission Decision Matrix

| Store | Can Submit When | Estimate |
|-------|----------------|----------|
| Chrome Web Store | G-01 + G-05 + G-06 + G-09 + G-10 + G-11 + C-* | T-003 + T-007 resolved |
| Firefox AMO | G-05 + G-06 + G-09 + G-10 + G-11 + F-* | T-007 resolved |
| Google Play | G-05 + G-06 + G-07 + G-09 + G-10 + G-11 + P-* | T-007 resolved + APK signed |
| Windows (direct dist) | G-01 + G-02 + G-05 + G-06 + G-09 + G-10 + G-11 | T-003 + T-007 resolved |
| macOS (direct dist) | G-03 + G-04 + G-05 + G-06 + G-09 + G-10 + G-11 | T-002 + T-007 resolved |

*Firefox AMO does not require EV cert — it's the fastest store to submit to once legal review is done.*
