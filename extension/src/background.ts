// ============================================================
// FILE:        background.ts
// MODULE:      Layer 3 — Browser Extension > Service Worker
// TASK:        T-031 (implementation — Session 4)
// PLATFORM:    chrome (MV3), firefox (MV2)
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Phase 2, Browser extension background/service worker
// DEPENDENCIES: Chrome declarativeNetRequest API, Native Messaging API,
//               Alarms API, Storage API (session + local)
// TEST COVERAGE: Test: URL blocked via declarativeNetRequest rule,
//                Test: exponential backoff reconnect after disconnect,
//                Test: BLOCK message redirects active tab
// KNOWN LIMITATIONS: MV3 service worker can terminate after 30s idle (Chrome).
//                    Must persist state via chrome.storage.session (ephemeral)
//                    and chrome.storage.local (durable).
//                    MV2 Firefox uses persistent background page (no termination issue).
// ============================================================

import {
    toDnrRules,
    type FocusMeUrlRule,
} from "./rule_converter";

// ============ Types ============

/** URL rule received from daemon via native messaging */
interface UrlRule {
    id: string;
    domain: string;
    path_pattern?: string;
    action: "block" | "allow";
    plan_id: string;
    schedule_active?: boolean;
}

/** Message envelope for daemon communication via native messaging */
interface DaemonMessage {
    type: string;
    version: number;
    request_id: string;
    payload: unknown;
}

/** Internal extension state persisted to chrome.storage.session */
interface SessionState {
    connected: boolean;
    lastSync: number;
    retryCount: number;
    forcedModeActive: boolean;
}

/** Durable state persisted to chrome.storage.local */
interface DurableState {
    activeRules: UrlRule[];
    elementBlockRules: ElementBlockRule[];
}

/** Element blocking rule forwarded to content scripts */
interface ElementBlockRule {
    domain: string;
    selectors: string[];
    action: "hide" | "blur" | "remove";
}

// ============ Constants ============

/** NMH name — must match NMH manifest JSON "name" field */
const NATIVE_HOST_NAME = "com.focusme.nmh";

/** Alarm name for periodic plan sync */
const SYNC_ALARM_NAME = "focusme-plan-sync";

/** Sync interval: 30 seconds expressed as minutes (Chrome minimum is 0.5 min) */
const SYNC_PERIOD_MINUTES = 0.5;

/** Blocked page path relative to extension root */
const BLOCKED_PAGE_PATH = "blocked/blocked.html";

/** Exponential backoff schedule (ms): 2s, 4s, 8s, 16s, 32s */
const BACKOFF_DELAYS_MS: readonly number[] = [2000, 4000, 8000, 16000, 32000];

/** Maximum reconnect attempts before giving up until next alarm tick */
const MAX_RETRIES = 5;

/** Message types — centralised to avoid hardcoded strings (fixes S-010) */
const MSG = {
    // Outbound (extension → daemon via NMH)
    PLAN_LIST:       "PLAN_LIST",
    PING:            "PING",
    URL_CHECK:       "URL_CHECK",
    // Inbound (daemon → extension via NMH)
    RULES_UPDATE:    "RULES_UPDATE",
    BLOCK:           "BLOCK",
    FORCED_MODE:     "FORCED_MODE_STATUS",
    PONG:            "PONG",
    ELEMENT_RULES:   "ELEMENT_RULES_UPDATE",
    // Internal (popup / content script ↔ background)
    GET_STATUS:      "GET_STATUS",
    GET_ELEMENT_RULES: "GET_ELEMENT_RULES",
    ELEMENT_RULES_UPDATED: "ELEMENT_RULES_UPDATED",
} as const;

// ============ State ============

let nativePort: chrome.runtime.Port | null = null;

let sessionState: SessionState = {
    connected: false,
    lastSync: 0,
    retryCount: 0,
    forcedModeActive: false,
};

let durableState: DurableState = {
    activeRules: [],
    elementBlockRules: [],
};

/** Handle returned by setTimeout for pending reconnect */
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;

// ============ Native Messaging — Exponential Backoff ============

/**
 * Connect to the FocusMe native messaging host.
 *
 * Uses exponential backoff on disconnect: 2 s → 4 s → 8 s → 16 s → 32 s,
 * then stops retrying until the next alarm-based sync tick reconnects.
 */
function connectToNativeHost(): void {
    // Clear any pending reconnect timer
    if (reconnectTimer !== null) {
        clearTimeout(reconnectTimer);
        reconnectTimer = null;
    }

    // Guard: already connected
    if (nativePort !== null) return;

    try {
        nativePort = chrome.runtime.connectNative(NATIVE_HOST_NAME);

        nativePort.onMessage.addListener((message: DaemonMessage) => {
            handleDaemonMessage(message);
        });

        nativePort.onDisconnect.addListener(() => {
            const err = chrome.runtime.lastError?.message ?? "unknown";
            console.warn(`[FocusMe] Native host disconnected: ${err}`);
            nativePort = null;
            sessionState.connected = false;
            saveSessionState();

            scheduleReconnect();
        });

        // Connection succeeded — reset retry counter
        sessionState.connected = true;
        sessionState.retryCount = 0;
        saveSessionState();
        console.log("[FocusMe] Connected to native messaging host");

        // Request full rule sync immediately after connect
        requestPlanSync();
    } catch (error) {
        console.error("[FocusMe] Failed to connect to native host:", error);
        nativePort = null;
        sessionState.connected = false;
        saveSessionState();

        scheduleReconnect();
    }
}

/**
 * Schedule a reconnect attempt using exponential backoff.
 * After MAX_RETRIES (5), stops retrying — the next alarm tick will try again.
 */
function scheduleReconnect(): void {
    if (sessionState.retryCount >= MAX_RETRIES) {
        console.warn(
            `[FocusMe] Max retries (${MAX_RETRIES}) reached — waiting for next sync alarm`
        );
        return;
    }

    const delay = BACKOFF_DELAYS_MS[sessionState.retryCount] ?? BACKOFF_DELAYS_MS[BACKOFF_DELAYS_MS.length - 1];
    sessionState.retryCount++;
    saveSessionState();

    console.log(
        `[FocusMe] Reconnect attempt ${sessionState.retryCount}/${MAX_RETRIES} in ${delay}ms`
    );

    reconnectTimer = setTimeout(() => {
        reconnectTimer = null;
        connectToNativeHost();
    }, delay);
}

/**
 * Send a message to the native messaging host.
 * Silently drops if not connected (caller should check sessionState.connected).
 */
function sendToNativeHost(message: DaemonMessage): void {
    if (nativePort) {
        nativePort.postMessage(message);
    } else {
        console.warn("[FocusMe] Cannot send — native host not connected");
    }
}

// ============ Message Handling ============

/**
 * Central dispatcher for all messages arriving from the daemon via NMH.
 */
function handleDaemonMessage(message: DaemonMessage): void {
    console.log("[FocusMe] Received from daemon:", message.type);

    switch (message.type) {
        case MSG.RULES_UPDATE:
            handleRulesUpdate(message.payload as UrlRule[]);
            break;

        case MSG.BLOCK:
            handleBlockCommand(message.payload as { domain: string; plan_id?: string });
            break;

        case MSG.FORCED_MODE:
            handleForcedModeStatus(message.payload as { active: boolean });
            break;

        case MSG.ELEMENT_RULES:
            handleElementRulesUpdate(message.payload as ElementBlockRule[]);
            break;

        case MSG.PONG:
            // Health check response — daemon is alive
            sessionState.lastSync = Date.now();
            saveSessionState();
            break;

        default:
            console.warn("[FocusMe] Unknown daemon message type:", message.type);
    }
}

/**
 * RULES_UPDATE — daemon pushed a new set of URL rules.
 * Convert to declarativeNetRequest dynamic rules and persist.
 */
async function handleRulesUpdate(rules: UrlRule[]): Promise<void> {
    durableState.activeRules = rules;
    sessionState.lastSync = Date.now();

    await updateDeclarativeNetRequestRules(rules);
    saveDurableState();
    saveSessionState();

    console.log(`[FocusMe] Rules updated: ${rules.length} active rules`);
}

/**
 * BLOCK — daemon instructs the extension to redirect the current tab
 * to the blocked page immediately (real-time enforcement).
 */
async function handleBlockCommand(payload: { domain: string; plan_id?: string }): Promise<void> {
    const domain = payload.domain ?? "unknown";
    const planId = payload.plan_id ?? "";
    const blockedUrl = chrome.runtime.getURL(
        `${BLOCKED_PAGE_PATH}?domain=${encodeURIComponent(domain)}&plan=${encodeURIComponent(planId)}`
    );

    try {
        // Find active tab(s) matching the blocked domain
        const tabs = await chrome.tabs.query({ active: true, currentWindow: true });
        for (const tab of tabs) {
            if (tab.id !== undefined) {
                const tabUrl = tab.url ?? "";
                // Only redirect if the tab is actually on the blocked domain
                if (tabUrl.includes(domain) || domain === "unknown") {
                    await chrome.tabs.update(tab.id, { url: blockedUrl });
                    console.log(`[FocusMe] Redirected tab ${tab.id} to blocked page for domain: ${domain}`);
                }
            }
        }
    } catch (err) {
        console.error("[FocusMe] Failed to redirect tab:", err);
    }
}

/**
 * FORCED_MODE_STATUS — toggle forced mode flag in session state.
 */
function handleForcedModeStatus(status: { active: boolean }): void {
    sessionState.forcedModeActive = status.active;
    saveSessionState();

    if (status.active) {
        console.log("[FocusMe] Forced mode ACTIVE — extension cannot be disabled");
    }
}

/**
 * ELEMENT_RULES_UPDATE — daemon pushed element-level blocking rules.
 * Persist and broadcast to all content scripts.
 */
async function handleElementRulesUpdate(rules: ElementBlockRule[]): Promise<void> {
    durableState.elementBlockRules = rules;
    saveDurableState();

    // Also store in chrome.storage.sync so content scripts can read on injection
    await chrome.storage.sync.set({ elementBlockRules: rules });

    // Broadcast to all tabs
    const tabs = await chrome.tabs.query({});
    for (const tab of tabs) {
        if (tab.id !== undefined) {
            chrome.tabs.sendMessage(tab.id, {
                type: MSG.ELEMENT_RULES_UPDATED,
                rules,
            }).catch(() => {
                // Tab may not have content script — ignore
            });
        }
    }
}

// ============ declarativeNetRequest (MV3) ============

/**
 * Convert URL rules to declarativeNetRequest dynamic rules and apply.
 * Delegates conversion to rule_converter.ts (T-032).
 */
async function updateDeclarativeNetRequestRules(rules: UrlRule[]): Promise<void> {
    // Remove all existing FocusMe dynamic rules
    const existingRules = await chrome.declarativeNetRequest.getDynamicRules();
    const removeRuleIds = existingRules.map((r) => r.id);

    // Convert via rule_converter (T-032)
    const focusMeRules: FocusMeUrlRule[] = rules.map((r) => ({
        id: r.id,
        domain: r.domain,
        path_pattern: r.path_pattern,
        action: r.action,
        plan_id: r.plan_id,
        schedule_active: r.schedule_active ?? true,
    }));

    const dnrRules = toDnrRules(focusMeRules);

    // Apply atomically
    await chrome.declarativeNetRequest.updateDynamicRules({
        removeRuleIds,
        addRules: dnrRules as chrome.declarativeNetRequest.Rule[],
    });

    console.log(`[FocusMe] Applied ${dnrRules.length} declarativeNetRequest rules`);
}

// ============ Periodic Sync (30s alarm) ============

/**
 * Set up periodic plan sync via the Alarms API.
 * Chrome's minimum alarm period is 0.5 minutes (30 seconds).
 */
function setupPeriodicSync(): void {
    chrome.alarms.create(SYNC_ALARM_NAME, {
        delayInMinutes: 0.5,          // first fire after 30s
        periodInMinutes: SYNC_PERIOD_MINUTES,
    });

    chrome.alarms.onAlarm.addListener((alarm) => {
        if (alarm.name === SYNC_ALARM_NAME) {
            onSyncAlarmFired();
        }
    });
}

/**
 * Called every 30 seconds by the alarm.
 * If not connected: attempt reconnect (resets retry counter).
 * If connected: request plan list sync from daemon.
 */
function onSyncAlarmFired(): void {
    if (!nativePort) {
        // Reset retry counter so backoff restarts fresh each alarm period
        sessionState.retryCount = 0;
        saveSessionState();
        connectToNativeHost();
    } else {
        requestPlanSync();
    }
}

/**
 * Request the daemon to send the full PLAN_LIST.
 * The daemon will respond with a RULES_UPDATE message.
 */
function requestPlanSync(): void {
    sendToNativeHost({
        type: MSG.PLAN_LIST,
        version: 1,
        request_id: generateRequestId(),
        payload: { action: "sync" },
    });

    // Also send a PING for liveness tracking
    sendToNativeHost({
        type: MSG.PING,
        version: 1,
        request_id: generateRequestId(),
        payload: null,
    });
}

// ============ State Persistence ============

/**
 * Persist ephemeral connection state to chrome.storage.session.
 * Session storage survives service worker restarts but not browser restarts.
 */
function saveSessionState(): void {
    chrome.storage.session.set({ focusmeSession: sessionState }).catch(() => {
        // Fallback for browsers that don't support storage.session (Firefox MV2)
        chrome.storage.local.set({ focusmeSession: sessionState });
    });
}

/**
 * Persist durable rule state to chrome.storage.local.
 * Survives browser restarts.
 */
function saveDurableState(): void {
    chrome.storage.local.set({ focusmeDurable: durableState });
}

/**
 * Load both session and durable state from storage.
 */
async function loadState(): Promise<void> {
    // Try session storage first, fall back to local
    try {
        const sessionResult = await chrome.storage.session.get("focusmeSession");
        if (sessionResult.focusmeSession) {
            sessionState = { ...sessionState, ...sessionResult.focusmeSession };
        }
    } catch {
        const localResult = await chrome.storage.local.get("focusmeSession");
        if (localResult.focusmeSession) {
            sessionState = { ...sessionState, ...localResult.focusmeSession };
        }
    }

    const durableResult = await chrome.storage.local.get("focusmeDurable");
    if (durableResult.focusmeDurable) {
        durableState = { ...durableState, ...durableResult.focusmeDurable };
    }
}

// ============ Utilities ============

/** Generate a unique request ID for daemon messages */
function generateRequestId(): string {
    return `ext-${Date.now()}-${Math.random().toString(36).substring(2, 8)}`;
}

// ============ Extension Lifecycle ============

/**
 * onInstalled — extension first install or update.
 * Initialise state, connect to NMH, start sync alarm.
 */
chrome.runtime.onInstalled.addListener(async () => {
    console.log("[FocusMe] Extension installed/updated");
    await loadState();
    connectToNativeHost();
    setupPeriodicSync();
});

/**
 * onStartup — browser launch (service worker may have been terminated).
 * Restore state, reconnect, restart alarms.
 */
chrome.runtime.onStartup.addListener(async () => {
    console.log("[FocusMe] Extension started");
    await loadState();

    // Re-apply any persisted rules so blocking is active immediately
    if (durableState.activeRules.length > 0) {
        await updateDeclarativeNetRequestRules(durableState.activeRules);
    }

    connectToNativeHost();
    setupPeriodicSync();
});

/**
 * onMessage — handle messages from popup.ts and content scripts.
 * Centralises all internal message types to avoid hardcoded strings (S-010).
 */
chrome.runtime.onMessage.addListener(
    (
        message: { type: string; [key: string]: unknown },
        _sender: chrome.runtime.MessageSender,
        sendResponse: (response: unknown) => void,
    ) => {
        switch (message.type) {
            case MSG.GET_STATUS:
                sendResponse({
                    connected: sessionState.connected,
                    activeRules: durableState.activeRules.length,
                    forcedMode: sessionState.forcedModeActive,
                    lastSync: sessionState.lastSync,
                });
                break;

            case MSG.GET_ELEMENT_RULES:
                sendResponse({
                    rules: durableState.elementBlockRules,
                });
                break;

            default:
                sendResponse({ error: "unknown_message_type" });
        }

        return true; // Keep sendResponse channel open for async
    }
);

export {};
