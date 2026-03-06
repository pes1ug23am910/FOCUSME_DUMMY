# Deployment & Release Guide

This document describes how to build production-ready packages for all supported platforms and prepare FocusMe for distribution.

---

## Table of Contents

- [Prerequisites](#prerequisites)
- [Version Management](#version-management)
- [Windows (MSI Installer)](#windows-msi-installer)
- [macOS (PKG Installer)](#macos-pkg-installer)
- [Linux (DEB/RPM Packages)](#linux-debrpm-packages)
- [Android (APK/AAB)](#android-apkaab)
- [Browser Extensions](#browser-extensions)
- [Release Checklist](#release-checklist)

---

## Prerequisites

### Signing Certificates
- **Windows:** Code signing certificate (.pfx or Azure Key Vault)
- **macOS:** Apple Developer Certificate (Developer ID Application)
- **Android:** Keystore (.jks or .keystore)

### Build Tools
- **Windows:** WiX Toolset 4.x
- **macOS:** Xcode Command Line Tools, `pkgbuild`, `productbuild`
- **Linux:** `dpkg-deb`, `rpmbuild`

---

## Version Management

Update version numbers in all relevant files:

```bash
# daemon/Cargo.toml
version = "1.0.0"

# ui/src-tauri/Cargo.toml
version = "1.0.0"

# ui/package.json
"version": "1.0.0"

# extension/package.json
"version": "1.0.0"

# extension/manifest.v3.json
"version": "1.0.0"

# android/app/build.gradle
versionCode = 100
versionName = "1.0.0"
```

---

## Windows (MSI Installer)

### 1. Build Components

```powershell
# Build daemon
cd daemon
cargo build --release --target x86_64-pc-windows-msvc

# Build UI (Tauri)
cd ../ui
npm run tauri build
```

### 2. Create MSI with WiX

```powershell
cd ../packaging/windows

# WiX 4.x
wix build `
    -arch x64 `
    -out FocusMe-1.0.0-x64.msi `
    -ext WixToolset.UI.wixext `
    -ext WixToolset.Util.wixext `
    FocusMe.wxs
```

**MSI Features:**
- Installs daemon as Windows Service
- Registers browser Native Messaging Host
- Creates Start Menu shortcuts
- Sets up auto-start registry keys

### 3. Sign the MSI

```powershell
signtool sign /f CodeSigningCert.pfx `
    /p <password> `
    /t http://timestamp.digicert.com `
    /fd SHA256 `
    FocusMe-1.0.0-x64.msi
```

**Output:** `packaging/windows/FocusMe-1.0.0-x64.msi`

---

## macOS (PKG Installer)

### 1. Build Components

```bash
# Build daemon (universal binary Intel + Apple Silicon)
cd daemon
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin

# Create universal binary
lipo -create \
    target/x86_64-apple-darwin/release/focusme-daemon \
    target/aarch64-apple-darwin/release/focusme-daemon \
    -output target/release/focusme-daemon-universal

# Build UI
cd ../ui
npm run tauri build -- --target universal-apple-darwin
```

### 2. Build System Extension (Optional)

```bash
cd ../macos
xcodebuild -project FocusMeESF.xcodeproj \
    -scheme FocusMeESF \
    -configuration Release \
    -archivePath FocusMeESF.xcarchive \
    archive

xcodebuild -exportArchive \
    -archivePath FocusMeESF.xcarchive \
    -exportPath ./build \
    -exportOptionsPlist ExportOptions.plist
```

### 3. Create PKG

```bash
cd ../packaging/macos

# Build component packages
pkgbuild --root ./payload \
    --identifier com.focusme.daemon \
    --version 1.0.0 \
    --scripts ./scripts \
    --install-location /Library/Application\ Support/FocusMe \
    FocusMe-daemon.pkg

pkgbuild --component ../ui/src-tauri/target/release/bundle/macos/FocusMe.app \
    --identifier com.focusme.ui \
    --version 1.0.0 \
    --install-location /Applications \
    FocusMe-ui.pkg

# Combine into distribution package
productbuild --distribution Distribution.xml \
    --resources ./resources \
    --package-path . \
    FocusMe-1.0.0-universal.pkg
```

### 4. Sign and Notarize

```bash
# Sign
productsign --sign "Developer ID Installer: Your Name" \
    FocusMe-1.0.0-universal.pkg \
    FocusMe-1.0.0-universal-signed.pkg

# Notarize
xcrun notarytool submit FocusMe-1.0.0-universal-signed.pkg \
    --apple-id <your-email> \
    --password <app-specific-password> \
    --team-id <team-id> \
    --wait

# Staple
xcrun stapler staple FocusMe-1.0.0-universal-signed.pkg
```

**Output:** `packaging/macos/FocusMe-1.0.0-universal-signed.pkg`

---

## Linux (DEB/RPM Packages)

### 1. Build Components

```bash
# Build daemon
cd daemon
cargo build --release --target x86_64-unknown-linux-gnu

# Build UI
cd ../ui
npm run tauri build
```

### 2. Create DEB Package

```bash
cd ../packaging/linux/debian

# Copy binaries
mkdir -p focusme-1.0.0/usr/local/bin
cp ../../../daemon/target/release/focusme-daemon focusme-1.0.0/usr/local/bin/
cp ../../../ui/src-tauri/target/release/focusme-ui focusme-1.0.0/usr/local/bin/

# Copy service file
mkdir -p focusme-1.0.0/etc/systemd/system
cp ../../../linux/focusme.service focusme-1.0.0/etc/systemd/system/

# Set up DEBIAN control directory
mkdir -p focusme-1.0.0/DEBIAN
cat > focusme-1.0.0/DEBIAN/control << EOF
Package: focusme
Version: 1.0.0
Section: utils
Priority: optional
Architecture: amd64
Depends: libc6 (>= 2.31), libssl1.1, unbound
Maintainer: Yash Verma <pes1ug23am910@pesu.pes.edu>
Description: Cross-platform productivity enforcer
 System-level app and URL blocking with kernel-level enforcement.
EOF

# Build package
dpkg-deb --build --root-owner-group focusme-1.0.0
mv focusme-1.0.0.deb focusme_1.0.0_amd64.deb
```

### 3. Create RPM Package

```bash
cd ../rpm

# Create RPM build tree
mkdir -p ~/rpmbuild/{BUILD,RPMS,SOURCES,SPECS,SRPMS}

# Create spec file
cp focusme.spec ~/rpmbuild/SPECS/

# Create source tarball
tar czf ~/rpmbuild/SOURCES/focusme-1.0.0.tar.gz \
    -C ../../../ \
    daemon/target/release/focusme-daemon \
    ui/src-tauri/target/release/focusme-ui \
    linux/focusme.service

# Build RPM
rpmbuild -ba ~/rpmbuild/SPECS/focusme.spec
```

**Outputs:**
- `packaging/linux/debian/focusme_1.0.0_amd64.deb`
- `~/rpmbuild/RPMS/x86_64/focusme-1.0.0-1.x86_64.rpm`

---

## Android (APK/AAB)

### 1. Configure Signing

Create `android/keystore.properties`:
```properties
storeFile=/path/to/keystore.jks
storePassword=<keystore-password>
keyAlias=focusme
keyPassword=<key-password>
```

### 2. Build Signed APK

```bash
cd android

# Release APK
./gradlew assembleRelease

# Output: app/build/outputs/apk/release/app-release.apk
```

### 3. Build App Bundle (AAB) for Play Store

```bash
# Release AAB
./gradlew bundleRelease

# Output: app/build/outputs/bundle/release/app-release.aab
```

### 4. Verify Signing

```bash
# Check APK signature
apksigner verify --print-certs app/build/outputs/apk/release/app-release.apk

# Check AAB signature
jarsigner -verify -verbose -certs app/build/outputs/bundle/release/app-release.aab
```

---

## Browser Extensions

### 1. Chrome Web Store

```bash
cd extension
npm run build:mv3

# Create ZIP
cd dist-chrome
zip -r ../focusme-chrome-1.0.0.zip *
```

**Upload to Chrome Web Store:**
1. Go to https://chrome.google.com/webstore/devconsole
2. Upload `focusme-chrome-1.0.0.zip`
3. Fill in store listing details
4. Submit for review

### 2. Firefox Add-ons (AMO)

```bash
cd extension
npm run build:mv2

# Sign with web-ext
npx web-ext sign \
    --source-dir dist-firefox \
    --api-key <api-key> \
    --api-secret <api-secret>
```

**Output:** `extension/web-ext-artifacts/focusme-1.0.0.xpi`

### 3. Edge Add-ons

Use the same Chrome build:
```bash
# Edge uses MV3 like Chrome
cd dist-chrome
zip -r ../focusme-edge-1.0.0.zip *
```

Upload to https://partner.microsoft.com/dashboard/microsoftedge

---

## Release Checklist

Use this checklist before each release:

### Pre-Release
- [ ] All tests pass (`cargo test`, `npm test`, `./gradlew test`)
- [ ] Version numbers updated in all manifests
- [ ] CHANGELOG.md updated with release notes
- [ ] Documentation reviewed and updated
- [ ] Security review completed
- [ ] Performance benchmarks run
- [ ] Bypass tests completed (see `docs/bypass_tests.md`)

### Build
- [ ] Windows MSI built and signed
- [ ] macOS PKG built, signed, and notarized
- [ ] Linux DEB package built
- [ ] Linux RPM package built
- [ ] Android APK signed
- [ ] Android AAB signed (for Play Store)
- [ ] Chrome extension built and zipped
- [ ] Firefox extension built and signed
- [ ] Edge extension built and zipped

### Testing
- [ ] Fresh install tested on Windows 10, Windows 11
- [ ] Fresh install tested on macOS 13+
- [ ] Fresh install tested on Ubuntu 22.04, Fedora 38
- [ ] Android app tested on SDK 26, 33, 34
- [ ] Browser extensions tested on latest Chrome, Firefox, Edge
- [ ] Upgrade from previous version tested
- [ ] Uninstall tested (clean removal)

### Distribution
- [ ] GitHub Release created with binaries attached
- [ ] Release notes published
- [ ] Installers uploaded to download server
- [ ] Chrome Web Store submission
- [ ] Firefox AMO submission
- [ ] Edge Add-ons submission
- [ ] Google Play Store submission (if applicable)

### Post-Release
- [ ] Download links verified
- [ ] Auto-update mechanism tested
- [ ] User documentation updated
- [ ] Support channels notified

---

## Distribution Channels

### Direct Downloads
- **GitHub Releases:** https://github.com/pes1ug23am910/focusme/releases
- Include SHA-256 checksums for all binaries

### App Stores
- **Chrome Web Store:** https://chrome.google.com/webstore/category/extensions
- **Firefox Add-ons:** https://addons.mozilla.org/
- **Microsoft Edge Add-ons:** https://microsoftedge.microsoft.com/addons/
- **Google Play Store:** https://play.google.com/store/apps

### Package Managers
- **Arch Linux (AUR):** Create PKGBUILD
- **Homebrew (macOS):** Create formula
- **Winget (Windows):** Submit manifest
- **Snap/Flatpak (Linux):** Create packages

---

## Continuous Deployment

### GitHub Actions Workflow

```yaml
name: Release

on:
  push:
    tags:
      - 'v*.*.*'

jobs:
  build-windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v3
      - name: Build daemon
        run: cargo build --release
      - name: Build UI
        run: |
          cd ui
          npm install
          npm run tauri build
      - name: Upload artifacts
        uses: actions/upload-artifact@v3
        with:
          name: windows-installer
          path: packaging/windows/*.msi

  # Similar jobs for macOS, Linux, Android...
```

---

## Security Considerations

1. **Never commit signing keys** to version control
2. **Store secrets** in GitHub Actions Secrets or environment variables
3. **Use strong passwords** for keystores and certificates
4. **Rotate credentials** annually
5. **Enable two-factor auth** on all distribution accounts
6. **Verify signatures** before distribution
7. **Use HTTPS** for all download links
8. **Provide checksums** (SHA-256) for all downloads

---

## Support Matrix

| Platform | Package Format | Distribution | Auto-Update |
|----------|---------------|--------------|-------------|
| Windows 10/11 | MSI | GitHub Releases, Website | ✅ |
| macOS 13+ | PKG | GitHub Releases, Website | ✅ |
| Ubuntu 22.04+ | DEB | GitHub Releases, APT repo | ✅ |
| Fedora 38+ | RPM | GitHub Releases, DNF repo | ✅ |
| Android 8+ | APK/AAB | Play Store, GitHub Releases | ✅ |
| Chrome | CRX (ZIP) | Chrome Web Store | ✅ |
| Firefox | XPI | AMO | ✅ |
| Edge | CRX (ZIP) | Edge Add-ons | ✅ |

---

## Questions or Issues?

Contact: Yash Verma (PES1UG23AM910) - [@pes1ug23am910](https://github.com/pes1ug23am910)

---

**Last Updated:** March 2026
