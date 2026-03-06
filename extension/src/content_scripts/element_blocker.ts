// ============================================================
// FILE:        element_blocker.ts
// MODULE:      Layer 3 — Browser Extension > Content Script
// TASK:        T-033 (implementation — Session 4)
// PLATFORM:    chrome (MV3), firefox (MV2)
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Phase 2, Element-level blocking content script
// DEPENDENCIES: DOM MutationObserver API, chrome.storage.sync
// TEST COVERAGE: Test: YouTube Shorts button hidden when rule active,
//                Test: style element re-added after page removes it,
//                Test: SPA navigation re-applies remove rules
// KNOWN LIMITATIONS: Only runs on pages where content scripts are allowed.
//                    Cannot run on chrome:// or browser internal pages.
//                    MutationObserver adds minimal performance overhead (~1ms per mutation batch).
// ============================================================

// ============ Types ============

/** Element blocking rule from the daemon (stored in chrome.storage.sync) */
interface ElementBlockRule {
    domain: string;
    selectors: string[];    // CSS selectors targets
    action: "hide" | "blur" | "remove";
}

// ============ Constants ============

/** Style element ID — used to locate our injected <style> in the DOM */
const FOCUSME_STYLE_ID = "focusme-element-blocker";

/** Data attribute marking the style as FocusMe-protected */
const PROTECTED_ATTR = "data-focusme-protected";

/** Debounce interval for MutationObserver callbacks (ms) */
const MUTATION_DEBOUNCE_MS = 100;

/** Interval for periodic style element integrity check (ms) */
const STYLE_GUARD_INTERVAL_MS = 2000;

// ============ State ============

/** Rules that match the current page's domain */
let activeRules: ElementBlockRule[] = [];

/** DOM MutationObserver for element re-application */
let bodyObserver: MutationObserver | null = null;

/** MutationObserver that watches for our style being removed from <head> */
let styleGuardObserver: MutationObserver | null = null;

/** Our injected <style> element */
let styleElement: HTMLStyleElement | null = null;

/** Debounce timer handle */
let mutationDebounceTimer: ReturnType<typeof setTimeout> | null = null;

// ============ Initialisation ============

/**
 * Initialise the element blocker content script.
 *
 * 1. Load rules from chrome.storage.sync (persisted by background.ts)
 * 2. Filter to rules matching current domain
 * 3. Inject CSS for hide/blur rules
 * 4. Remove elements for remove rules
 * 5. Start MutationObserver for SPA re-renders
 * 6. Start style guard for anti-circumvention
 *
 * Called at document_idle (after DOM is ready).
 */
function initialize(): void {
    // Primary source: chrome.storage.sync (set by background.ts on ELEMENT_RULES_UPDATE)
    chrome.storage.sync.get("elementBlockRules", (result) => {
        const allRules: ElementBlockRule[] = result.elementBlockRules ?? [];
        activeRules = filterRulesForCurrentDomain(allRules);

        if (activeRules.length > 0) {
            applyBlockingRules();
            startBodyObserver();
            startStyleGuard();
        }
    });

    // Also request rules from background (fallback / real-time update path)
    chrome.runtime.sendMessage({ type: "GET_ELEMENT_RULES" }, (response) => {
        if (chrome.runtime.lastError) return; // No background page ready yet
        if (response?.rules) {
            const freshRules = filterRulesForCurrentDomain(response.rules);
            if (freshRules.length > activeRules.length) {
                activeRules = freshRules;
                applyBlockingRules();
                if (!bodyObserver) startBodyObserver();
                if (!styleGuardObserver) startStyleGuard();
            }
        }
    });

    // Listen for live rule updates pushed by background.ts
    chrome.runtime.onMessage.addListener((message) => {
        if (message.type === "ELEMENT_RULES_UPDATED") {
            activeRules = filterRulesForCurrentDomain(message.rules ?? []);
            applyBlockingRules();

            if (activeRules.length > 0) {
                if (!bodyObserver) startBodyObserver();
                if (!styleGuardObserver) startStyleGuard();
            } else {
                cleanup();
            }
        }
    });
}

// ============ Domain Matching ============

/**
 * Filter rules to those matching the current page's hostname.
 * Matches exact domain and parent domain (e.g., rule for "youtube.com"
 * matches "www.youtube.com" and "m.youtube.com").
 */
function filterRulesForCurrentDomain(rules: ElementBlockRule[]): ElementBlockRule[] {
    const currentDomain = window.location.hostname.toLowerCase();

    return rules.filter((rule) => {
        const ruleDomain = rule.domain.toLowerCase();
        return currentDomain === ruleDomain || currentDomain.endsWith(`.${ruleDomain}`);
    });
}

// ============ CSS Injection (hide / blur) ============

/**
 * Apply all blocking rules to the current page.
 *
 * - hide → CSS `visibility: hidden !important; height: 0 !important;`
 * - blur → CSS `filter: blur(10px) !important; pointer-events: none !important;`
 * - remove → DOM removal via querySelectorAll + Element.remove()
 */
function applyBlockingRules(): void {
    injectStyleRules();
    applyRemoveRules();
}

/**
 * Build and inject a <style> element for hide and blur rules.
 * If the element already exists, its content is replaced.
 * The element is marked with data-focusme-protected to enable tamper detection.
 */
function injectStyleRules(): void {
    const cssRules: string[] = [];

    for (const rule of activeRules) {
        for (const selector of rule.selectors) {
            // Validate selector to prevent injection attacks
            if (!isValidSelector(selector)) continue;

            switch (rule.action) {
                case "hide":
                    cssRules.push(
                        `${selector} { visibility: hidden !important; height: 0 !important; overflow: hidden !important; margin: 0 !important; padding: 0 !important; }`
                    );
                    break;
                case "blur":
                    cssRules.push(
                        `${selector} { filter: blur(10px) !important; pointer-events: none !important; user-select: none !important; }`
                    );
                    break;
                // "remove" is handled by applyRemoveRules() — no CSS needed
            }
        }
    }

    if (cssRules.length === 0) {
        // No CSS rules — remove existing style if present
        removeStyleElement();
        return;
    }

    const cssText = cssRules.join("\n");

    if (styleElement && document.contains(styleElement)) {
        // Update existing element in-place
        styleElement.textContent = cssText;
    } else {
        // Create new style element
        styleElement = document.createElement("style");
        styleElement.id = FOCUSME_STYLE_ID;
        styleElement.setAttribute(PROTECTED_ATTR, "true");
        styleElement.textContent = cssText;

        const target = document.head || document.documentElement;
        target.appendChild(styleElement);
    }
}

/**
 * Remove the FocusMe style element from the DOM.
 */
function removeStyleElement(): void {
    if (styleElement && document.contains(styleElement)) {
        styleElement.remove();
    }
    styleElement = null;
}

// ============ DOM Removal ============

/**
 * Remove elements matching "remove" action rules from the DOM.
 * Called on initial load and after each MutationObserver batch.
 */
function applyRemoveRules(): void {
    const removeRules = activeRules.filter((r) => r.action === "remove");

    for (const rule of removeRules) {
        for (const selector of rule.selectors) {
            if (!isValidSelector(selector)) continue;

            try {
                const elements = document.querySelectorAll(selector);
                elements.forEach((el) => el.remove());
            } catch {
                // Invalid selector at runtime — skip silently
            }
        }
    }
}

// ============ MutationObserver — SPA Re-render Handling ============

/**
 * Observe subtree mutations on document.body.
 * When new nodes are added (SPA navigation, dynamic content loading),
 * re-apply remove rules after a debounce period.
 */
function startBodyObserver(): void {
    if (bodyObserver) {
        bodyObserver.disconnect();
    }

    bodyObserver = new MutationObserver((mutations) => {
        let hasNewNodes = false;
        for (const mutation of mutations) {
            if (mutation.addedNodes.length > 0) {
                hasNewNodes = true;
                break;
            }
        }

        if (!hasNewNodes) return;

        // Debounce: batch rapid mutations into one re-apply cycle
        if (mutationDebounceTimer !== null) {
            clearTimeout(mutationDebounceTimer);
        }

        mutationDebounceTimer = setTimeout(() => {
            mutationDebounceTimer = null;
            applyRemoveRules();
        }, MUTATION_DEBOUNCE_MS);
    });

    // Observe the entire body subtree for added nodes
    const observeTarget = document.body || document.documentElement;
    bodyObserver.observe(observeTarget, {
        childList: true,
        subtree: true,
    });
}

// ============ Anti-Circumvention — Style Guard ============

/**
 * Protect the injected <style> element from being removed by page scripts.
 *
 * Strategy:
 * 1. MutationObserver watches <head> for childList removals — if our style
 *    disappears, re-inject it immediately.
 * 2. Periodic interval check (every 2s) as a fallback — some frameworks
 *    (e.g. YouTube's Polymer) rebuild <head> entirely without triggering
 *    a clean childList mutation.
 */
function startStyleGuard(): void {
    // --- MutationObserver on <head> ---
    if (styleGuardObserver) {
        styleGuardObserver.disconnect();
    }

    styleGuardObserver = new MutationObserver((mutations) => {
        for (const mutation of mutations) {
            for (let i = 0; i < mutation.removedNodes.length; i++) {
                const removed = mutation.removedNodes[i];
                if (
                    removed instanceof HTMLStyleElement &&
                    removed.id === FOCUSME_STYLE_ID
                ) {
                    // Our style was removed — re-inject
                    styleElement = null;
                    injectStyleRules();
                    return;
                }
            }
        }
    });

    const headTarget = document.head || document.documentElement;
    styleGuardObserver.observe(headTarget, {
        childList: true,
    });

    // --- Periodic fallback check ---
    setInterval(() => {
        if (activeRules.length > 0 && activeRules.some((r) => r.action !== "remove")) {
            if (!styleElement || !document.contains(styleElement)) {
                styleElement = null;
                injectStyleRules();
            }
        }
    }, STYLE_GUARD_INTERVAL_MS);
}

// ============ Cleanup ============

/**
 * Remove all FocusMe injections from the page (e.g., when rules are cleared).
 */
function cleanup(): void {
    removeStyleElement();

    if (bodyObserver) {
        bodyObserver.disconnect();
        bodyObserver = null;
    }

    if (styleGuardObserver) {
        styleGuardObserver.disconnect();
        styleGuardObserver = null;
    }

    if (mutationDebounceTimer !== null) {
        clearTimeout(mutationDebounceTimer);
        mutationDebounceTimer = null;
    }

    activeRules = [];
}

// ============ Utilities ============

/**
 * Validate a CSS selector string to prevent injection attacks.
 * Rejects selectors containing script-injection vectors.
 */
function isValidSelector(selector: string): boolean {
    if (!selector || selector.length === 0 || selector.length > 500) return false;

    // Reject anything that could be an injection vector
    if (/[<>"']/.test(selector)) return false;

    // Basic structural validation — try creating a static NodeList
    try {
        document.createDocumentFragment().querySelector(selector);
        return true;
    } catch {
        return false;
    }
}

// ============ Entry Point ============

// Initialise at document_idle (when DOM is ready)
initialize();

export {};
