# FocusMe — End User License Agreement (EULA)

> **FILE:** eula.md
> **TASK:** T-007 (template — requires legal counsel review)
> **AUTHOR:** FocusMe Co-Pilot (Claude Opus)
> **SESSION:** 5
> **STATUS:** `[LEGAL REVIEW REQUIRED]` — draft template for legal counsel to finalize
> **LAST UPDATED:** `[DATE]`

---

**`[LEGAL REVIEW REQUIRED]` — This entire document requires review and approval by qualified legal counsel before publication or inclusion in any installer.**

---

## FocusMe End User License Agreement

**Effective Date:** `[EFFECTIVE DATE — LEGAL REVIEW REQUIRED]`

**IMPORTANT — READ CAREFULLY BEFORE INSTALLING OR USING FOCUSME.**

By installing, copying, or otherwise using FocusMe ("the Software"), you agree to be bound by the terms of this End User License Agreement ("EULA"). If you do not agree to these terms, do not install or use the Software.

---

### 1. License Grant

Subject to the terms of this EULA, `[LEGAL ENTITY — LEGAL REVIEW REQUIRED]` ("Licensor") grants you a:

- **Personal**, non-exclusive, non-transferable, revocable license
- To install and use the Software on devices that you own or control
- For the purpose of personal productivity, self-imposed access restriction, and digital wellness

This license is:
- Limited to the number of devices specified by your subscription tier `[LEGAL REVIEW REQUIRED — define device limits]`
- Valid for the duration of your subscription or, for one-time purchases, perpetually for the purchased version

`[LEGAL REVIEW REQUIRED — License type (subscription vs perpetual), number of devices, transferability]`

---

### 2. Restrictions

You may NOT:

#### 2.1 Reverse Engineering
- Reverse engineer, decompile, disassemble, or attempt to derive the source code of the Software's enforcement mechanisms (blocking logic, forced mode implementation, tamper detection)
- This restriction exists because the effectiveness of FocusMe depends on users not being able to trivially bypass its enforcement

`[LEGAL REVIEW REQUIRED — Reverse engineering restrictions may conflict with EU Directive 2009/24/EC which allows decompilation for interoperability. Carve out interoperability exception.]`

#### 2.2 Redistribution
- Redistribute, sublicense, rent, lease, or lend the Software to third parties
- Share your license key or subscription credentials

#### 2.3 Modification
- Modify, adapt, or create derivative works of the Software
- Remove, alter, or obscure any proprietary notices or labels

#### 2.4 Misuse
- Use the Software to restrict another adult's device access without their informed consent
- Use the Software in any way that violates applicable law
- Use the Software to block access to emergency services or safety resources

---

### 3. Open Source Components

The Software includes open source components licensed under their respective terms. A complete Software Bill of Materials (SBOM) is available at:
- `docs/sbom.json` (CycloneDX format)
- Or within the application: Settings → About → Open Source Licenses

**Key open source dependencies and their licenses:**

| Component | License | Linking |
|-----------|---------|---------|
| Rust standard library | MIT/Apache-2.0 | Static |
| Tauri | MIT/Apache-2.0 | Dynamic |
| SQLCipher | BSD-3-Clause | Bundled |
| libbpf-rs | BSD-2-Clause | Dynamic |
| libbpf (C library) | LGPL-2.1 | Dynamic `[S-006]` |
| React | MIT | Bundled |
| webextension-polyfill | MPL-2.0 | Bundled |

`[LEGAL REVIEW REQUIRED — Verify all open source licenses are compatible with proprietary distribution. S-006 flags libbpf LGPL-2.1 — ensure dynamic linking or obtain legal opinion on static linking.]`

**Your rights under open source licenses are not restricted by this EULA.** The restrictions in Section 2 apply only to the proprietary portions of the Software.

---

### 4. Forced Mode / Lockout Acknowledgment

By enabling FocusMe's Forced Mode feature, you acknowledge and agree that:

1. **Voluntary restriction:** You are voluntarily choosing to restrict your own access to specified applications and websites for a defined period
2. **Intentional friction:** The emergency unlock procedure is intentionally difficult (requiring a pre-set password verified via Argon2id key derivation) to discourage impulsive disabling
3. **No liability:** The Licensor is not liable for any consequences arising from your inability to access blocked content during a Forced Mode session
4. **Your responsibility:** You are solely responsible for:
   - Choosing appropriate blocking durations
   - Remembering your emergency unlock password
   - Ensuring critical applications are not included in blocking plans
5. **Emergency access:** An emergency unlock procedure is always available through the application settings

`[LEGAL REVIEW REQUIRED — Duty of care, unconscionability, jurisdiction-specific consumer protection]`

---

### 5. Data Collection and Privacy

The Software's data practices are governed by the FocusMe Privacy Policy.

Key points relevant to this EULA:
- The Software stores blocking plans, usage statistics, and preferences in an encrypted local database
- By default, no data is transmitted to external servers
- Optional telemetry (if enabled) collects anonymous usage metrics only
- You retain ownership of all your data

---

### 6. Intellectual Property

The Software, including all code, documentation, user interface designs, icons, and trademarks, is the intellectual property of the Licensor.

- This EULA does not transfer any intellectual property rights to you
- "FocusMe" and the FocusMe logo are trademarks of `[LEGAL ENTITY]`
- All rights not expressly granted in this EULA are reserved

---

### 7. Warranty Disclaimer

`[LEGAL REVIEW REQUIRED — Consumer warranty laws vary significantly by jurisdiction]`

THE SOFTWARE IS PROVIDED "AS IS" WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO:

- WARRANTIES OF MERCHANTABILITY
- FITNESS FOR A PARTICULAR PURPOSE
- NON-INFRINGEMENT
- THAT BLOCKING WILL BE EFFECTIVE AGAINST ALL BYPASS METHODS
- THAT THE SOFTWARE WILL BE ERROR-FREE OR UNINTERRUPTED

Some jurisdictions do not allow the exclusion of implied warranties. In such jurisdictions, the above exclusions may not apply to you. You may have additional statutory rights.

---

### 8. Limitation of Liability

`[LEGAL REVIEW REQUIRED — CRITICAL — Must comply with jurisdiction-specific consumer protection]`

TO THE MAXIMUM EXTENT PERMITTED BY APPLICABLE LAW, IN NO EVENT SHALL THE LICENSOR BE LIABLE FOR:

- Any indirect, incidental, special, consequential, or punitive damages
- Loss of data, profits, revenue, or business opportunities
- Damages arising from Forced Mode lockouts
- Damages arising from bypass of blocking mechanisms
- Damages arising from interaction with any third-party software

THE LICENSOR'S TOTAL AGGREGATE LIABILITY SHALL NOT EXCEED THE AMOUNT PAID BY YOU FOR THE SOFTWARE IN THE TWELVE (12) MONTHS PRECEDING THE EVENT GIVING RISE TO LIABILITY.

---

### 9. Uninstall Rights

You may uninstall the Software at any time:

- **Desktop (Windows/macOS/Linux):** Standard OS uninstall procedure. If plan protection is active, the uninstaller will prompt for a challenge code.
- **Browser extension:** Remove from browser extension settings
- **Android:** Standard app uninstall from Settings or long-press

**Note:** During an active Forced Mode session, uninstallation may require the emergency unlock password. This is by design — the Software is functioning as you configured it.

`[LEGAL REVIEW REQUIRED — Uninstall rights may be regulated by consumer protection laws. Verify that challenge code requirement during uninstall is permissible.]`

---

### 10. Updates

The Software may automatically check for and install updates. Updates may:
- Add new features
- Fix bugs and security vulnerabilities
- Modify enforcement mechanisms
- Change subscription requirements

By accepting this EULA, you consent to automatic updates. You may disable automatic updates in Settings, but security updates are strongly recommended.

---

### 11. Termination

- **By you:** Uninstall the Software from all devices
- **By Licensor:** We may terminate this license if you breach any term of this EULA
- **Effect of termination:** Cease all use. Uninstall the Software. Local data remains on your device until you delete it.

---

### 12. Governing Law

This EULA shall be governed by the laws of `[JURISDICTION — LEGAL REVIEW REQUIRED]`, without regard to conflict of law principles.

`[LEGAL REVIEW REQUIRED — Governing law, mandatory consumer protection, EU directive compliance]`

---

### 13. Severability

If any provision of this EULA is held invalid or unenforceable, the remaining provisions continue in full force and effect.

---

### 14. Entire Agreement

This EULA, together with the Terms of Service and Privacy Policy, constitutes the entire agreement between you and the Licensor regarding the Software.

---

### 15. Contact

For questions about this EULA:
- **Email:** `[LEGAL EMAIL — LEGAL REVIEW REQUIRED]`
- **Mailing Address:** `[PHYSICAL ADDRESS — LEGAL REVIEW REQUIRED]`

---

**BY INSTALLING FOCUSME, YOU ACKNOWLEDGE THAT YOU HAVE READ THIS EULA, UNDERSTAND IT, AND AGREE TO BE BOUND BY ITS TERMS.**

---

*Template generated Session 5. ALL `[LEGAL REVIEW REQUIRED]` sections must be completed by qualified legal counsel. Do NOT include in any installer or publication without legal approval.*
