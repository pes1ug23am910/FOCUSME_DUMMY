// ============================================================
// FILE:        blocked.ts
// MODULE:      Layer 3 — Browser Extension > Blocked Page Script
// TASK:        T-031 (blocked page)
// PLATFORM:    chrome (MV3), firefox (MV2)
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Phase 2, Session 3
// ============================================================

document.addEventListener("DOMContentLoaded", () => {
  const params = new URLSearchParams(window.location.search);
  const domain = params.get("domain") || "Unknown site";
  const planId = params.get("plan");

  // Display blocked domain
  const domainEl = document.getElementById("blocked-domain");
  if (domainEl) {
    domainEl.textContent = domain;
  }

  // Display plan info if available
  const planInfo = document.getElementById("plan-info");
  if (planInfo && planId) {
    planInfo.textContent = `Blocked by plan: ${planId}`;
  }

  // Go back button
  const goBackBtn = document.getElementById("go-back-btn");
  if (goBackBtn) {
    goBackBtn.addEventListener("click", () => {
      if (window.history.length > 1) {
        window.history.back();
      } else {
        // Navigate to new tab page
        window.location.href = "about:blank";
      }
    });
  }
});

export {};
