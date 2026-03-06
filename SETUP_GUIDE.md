# FocusMe - Quick Start Guide

This guide provides step-by-step instructions for getting FocusMe up and running on your development machine.

---

## Prerequisites Checklist

Before you begin, ensure you have the following installed:

### All Platforms
- [ ] **Rust 1.75+** - [Install from rustup.rs](https://rustup.rs/)
- [ ] **Node.js 18+** - [Download from nodejs.org](https://nodejs.org/)
- [ ] **Git** - For cloning the repository

### Platform-Specific

#### Windows
- [ ] **Visual Studio Build Tools 2022**
  - Install with "Desktop development with C++" workload
  - Download: https://visualstudio.microsoft.com/downloads/
- [ ] **Windows SDK 10.0.22000+**

#### macOS
- [ ] **Xcode 15+** with Command Line Tools
  ```bash  
  xcode-select --install
  ```

#### Linux (Ubuntu/Debian)
```bash
sudo apt update
sudo apt install -y \
    build-essential \
    clang \
    libbpf-dev \
    pkg-config \
    libssl-dev \
    unbound
```

#### Android Development
- [ ] **Android Studio** (latest stable)
- [ ] **Android SDK 26+**
- [ ] **Android NDK 25+**

---

## Step-by-Step Setup

### 1. Clone the Repository

```bash
git clone https://github.com/pes1ug23am910/focusme.git
cd focusme
```

### 2. Build and Install the Daemon

The daemon is the core service that enforces blocking rules.

```bash
cd daemon
cargo build --release
```

**Install as a system service:**

<details>
<summary><b>Windows (PowerShell as Administrator)</b></summary>

```powershell
# Copy binary to Program Files
New-Item -ItemType Directory -Force -Path "C:\Program Files\FocusMe"
Copy-Item "target\release\focusme-daemon.exe" -Destination "C:\Program Files\FocusMe\"

# Create and start service
sc.exe create FocusMeDaemon `
    binPath= "C:\Program Files\FocusMe\focusme-daemon.exe" `
    start= auto `
    DisplayName= "FocusMe Enforcement Daemon"

sc.exe start FocusMeDaemon

# Verify service is running
sc.exe query FocusMeDaemon
```
</details>

<details>
<summary><b>Linux (Ubuntu/Debian)</b></summary>

```bash
# Copy binary and service file
sudo cp target/release/focusme-daemon /usr/local/bin/
sudo chmod +x /usr/local/bin/focusme-daemon
sudo cp ../linux/focusme.service /etc/systemd/system/

# Enable and start service
sudo systemctl daemon-reload
sudo systemctl enable focusme.service
sudo systemctl start focusme.service

# Verify service is running
sudo systemctl status focusme.service
```
</details>

<details>
<summary><b>macOS</b></summary>

```bash
# Copy binary and LaunchDaemon plist
sudo cp target/release/focusme-daemon /usr/local/bin/
sudo chmod +x /usr/local/bin/focusme-daemon
sudo cp ../macos/com.focusme.daemon.plist /Library/LaunchDaemons/

# Load and start daemon
sudo launchctl bootstrap system /Library/LaunchDaemons/com.focusme.daemon.plist

# Verify daemon is running
sudo launchctl list | grep focusme
```

**Note:** macOS requires additional permissions:
- System Settings → Privacy & Security → Full Disk Access → Add focusme-daemon
- May require System Extension approval on first run
</details>

### 3. Build the Desktop UI

```bash
cd ../ui
npm install

# Development mode (with hot-reload)
npm run tauri dev

# Or build for production
npm run tauri build
```

**Production builds will be located at:**
- Windows: `src-tauri/target/release/FocusMe.exe`
- macOS: `src-tauri/target/release/bundle/macos/FocusMe.app`
- Linux: `src-tauri/target/release/focusme-ui`

### 4. Build the Browser Extension

```bash
cd ../extension
npm install

# For Chrome/Edge (Manifest V3)
npm run build:mv3

# For Firefox (Manifest V2)  
npm run build:mv2
```

**Load the extension:**
- **Chrome/Edge:**
  1. Open `chrome://extensions/`
  2. Enable "Developer mode" (top right)
  3. Click "Load unpacked"
  4. Select the `dist-chrome/` directory
  
- **Firefox:**
  1. Open `about:debugging#/runtime/this-firefox`
  2. Click "Load Temporary Add-on..."
  3. Select `dist-firefox/manifest.json`

### 5. Build Android App (Optional)

```bash
cd ../android

# Debug build
./gradlew assembleDebug

# Signed release build (requires keystore)
./gradlew assembleRelease
```

**Install on device:**
```bash
adb install app/build/outputs/apk/debug/app-debug.apk
```

### 6. Build Cloud Backend (Optional)

The cloud backend is optional and provides plan syncing and family dashboard features.

```bash
cd ../backend

# Start PostgreSQL via Docker
docker compose up -d postgres

# Create .env file
cp .env.example .env
# Edit .env and set:
#   DATABASE_URL=postgres://focusme:password@localhost:5432/focusme
#   JWT_SECRET=<generate with: openssl rand -base64 64>

# Install sqlx-cli
cargo install sqlx-cli --features postgres --no-default-features

# Run migrations
cargo sqlx migrate run

# Start server
cargo run --release
```

Server will be available at `http://localhost:8080`

---

## Verification

### Test the Daemon

```bash
# Check if daemon is running
# Windows (PowerShell)
sc.exe query FocusMeDaemon

# Linux
sudo systemctl status focusme.service

# macOS
sudo launchctl list | grep focusme
```

### Test the UI

1. Launch the FocusMe UI application
2. You should see the dashboard with no plans configured
3. Try creating a test plan:
   - Click "Create Plan"
   - Add a simple URL rule (e.g., block `twitter.com`)
   - Set schedule to "Always active"
   - Save plan

### Test Browser Extension

1. Ensure the extension is loaded and enabled
2. Create a plan in the UI that blocks a specific website
3. Try navigating to that website
4. You should see a block page

---

## Common Issues

### Daemon won't start

**Windows:**
- Ensure you're running PowerShell as Administrator
- Check Windows Event Viewer for error messages
- Verify antivirus isn't blocking the daemon

**Linux:**
```bash
# Check logs
sudo journalctl -u focusme.service -n 50
```

**macOS:**
```bash
# Check console logs
sudo log show --predicate 'processImagePath contains "focusme"' --last 5m
```

### Extension not connecting to daemon

1. Check that the daemon is running (see "Test the Daemon" above)
2. Verify the Native Messaging Host is installed:
   - Chrome/Edge: Check `~/.config/google-chrome/NativeMessagingHosts/` (Linux/macOS) or `%LOCALAPPDATA%\Google\Chrome\User Data\NativeMessagingHosts\` (Windows)
   - Firefox: Check manifest location in browser console

3. Check browser console for errors:
   - Right-click extension icon → "Inspect popup"
   - Look for connection errors

### UI won't build

```bash
# Clear node_modules and reinstall
cd ui
rm -rf node_modules package-lock.json
npm install
npm run tauri dev
```

### Rust compilation fails

```bash
# Update Rust toolchain
rustup update stable

# Clear build cache
cd daemon
cargo clean
cargo build --release
```

---

## Next Steps

1. **Read the Architecture:** See [docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md) for system internals
2. **Explore Features:** Create different types of plans (schedule-based, quota-based)
3. **Test Enforcement:** Try the bypass tests in [docs/bypass_tests.md](../docs/bypass_tests.md)
4. **Contribute:** See [CONTRIBUTING.md](../CONTRIBUTING.md) for development guidelines

---

## Getting Help

- **Documentation:** [docs/](../docs/)
- **Issues:** Check existing GitHub issues or create a new one
- **Architecture Questions:** Read [docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md)

---

**Need more help?** Contact: Yash Verma (PES1UG23AM910) - [@pes1ug23am910](https://github.com/pes1ug23am910)
