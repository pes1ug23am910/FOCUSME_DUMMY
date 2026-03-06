// ============================================================
// FILE:        popup.ts
// MODULE:      Layer 3 — Browser Extension > Popup UI
// TASK:        T-031 (popup)
// PLATFORM:    chrome (MV3), firefox (MV2)
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Phase 2, Session 3
// ============================================================

interface PopupStatus {
  connected: boolean;
  activeRules: number;
  forcedMode: boolean;
  lastSync: number;
}

document.addEventListener("DOMContentLoaded", () => {
  const daemonStatus = document.getElementById("daemon-status")!;
  const ruleCount = document.getElementById("rule-count")!;
  const forcedMode = document.getElementById("forced-mode")!;
  const lastSync = document.getElementById("last-sync")!;
  const syncBtn = document.getElementById("sync-btn")!;
  const openDashboard = document.getElementById("open-dashboard")!;

  // Fetch current status from background
  chrome.runtime.sendMessage({ type: "GET_STATUS" }, (response: PopupStatus) => {
    if (!response) return;

    // Daemon connection
    daemonStatus.textContent = response.connected ? "Connected" : "Disconnected";
    daemonStatus.className = `status-badge ${response.connected ? "status-connected" : "status-disconnected"}`;

    // Active rules
    ruleCount.textContent = String(response.activeRules);

    // Forced mode
    forcedMode.textContent = response.forcedMode ? "Active" : "Inactive";
    forcedMode.className = `status-badge ${response.forcedMode ? "status-active" : "status-inactive"}`;

    // Last sync
    if (response.lastSync > 0) {
      const elapsed = Date.now() - response.lastSync;
      if (elapsed < 60_000) {
        lastSync.textContent = "Just now";
      } else if (elapsed < 3_600_000) {
        lastSync.textContent = `${Math.floor(elapsed / 60_000)}m ago`;
      } else {
        lastSync.textContent = new Date(response.lastSync).toLocaleTimeString();
      }
    }
  });

  // Sync button
  syncBtn.addEventListener("click", () => {
    chrome.runtime.sendMessage({ type: "FORCE_SYNC" });
    syncBtn.textContent = "Syncing...";
    syncBtn.setAttribute("disabled", "true");
    setTimeout(() => {
      syncBtn.textContent = "Sync Now";
      syncBtn.removeAttribute("disabled");
      // Refresh status
      location.reload();
    }, 2000);
  });

  // Open dashboard
  openDashboard.addEventListener("click", (e) => {
    e.preventDefault();
    // Open the Tauri desktop app (or a web dashboard URL)
    chrome.tabs.create({ url: "focusme://dashboard" });
    window.close();
  });
});

export {};
