// ============================================================
// FILE:        rule_converter.ts
// MODULE:      Layer 3 — Browser Extension > Rule Converter
// TASK:        T-032 (implementation — Session 4)
// PLATFORM:    chrome (MV3), firefox (MV2)
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Phase 2, URL rule conversion
// DEPENDENCIES: Chrome declarativeNetRequest API (MV3), webRequest API (MV2)
// TEST COVERAGE: Test: domain pattern correctly converts to DNR filter,
//                Test: wildcard subdomain pattern generates *.domain filter,
//                Test: allow rules get action type 'allow',
//                Test: max 5000 rules enforced
// KNOWN LIMITATIONS: MV3 declarativeNetRequest has 30,000 total dynamic rule limit,
//                    but FocusMe caps at 5,000 to leave headroom for other extensions.
//                    MV2 webRequest.onBeforeRequest is synchronous blocking.
// ============================================================

// ============ Types ============

/** FocusMe URL rule from daemon */
export interface FocusMeUrlRule {
    id: string;
    domain: string;
    path_pattern?: string;      // Optional path regex/glob (e.g., "/r/*")
    action: "block" | "allow";
    plan_id: string;
    schedule_active: boolean;
}

/** Chrome MV3 declarativeNetRequest rule (mirrors chrome.declarativeNetRequest.Rule) */
export interface DNRRule {
    id: number;
    priority: number;
    action: {
        type: string;
        redirect?: { extensionPath: string };
    };
    condition: {
        urlFilter?: string;
        regexFilter?: string;
        resourceTypes: string[];
        domains?: string[];
        excludedDomains?: string[];
    };
}

/** Firefox MV2 webRequest blocking pattern */
export interface WebRequestPattern {
    urls: string[];
    domain: string;
    action: "block" | "allow";
    plan_id: string;
}

// ============ Constants ============

/** Blocked page path relative to extension root */
const BLOCKED_PAGE = "/blocked/blocked.html";

/** Maximum number of DNR rules FocusMe will generate (leave headroom for other extensions) */
const MAX_DNR_RULES = 5000;

// ============ MV3 Conversion — Primary API ============

/**
 * Convert FocusMe URL rules to Chrome MV3 declarativeNetRequest rules.
 *
 * urlFilter format (per Chrome docs):
 *   - `||domain|`          — domain anchor + separator: matches domain exactly + subdomains
 *   - `||domain/path`      — domain anchor with path: matches domain/path
 *   - `||*.domain|`        — explicit wildcard subdomain anchor
 *
 * Rule priorities:
 *   - Allow rules get priority 2 (higher = wins)
 *   - Block rules get priority 1
 *
 * Actions:
 *   - Block → redirect to /blocked/blocked.html with domain + plan query params
 *   - Allow → declarativeNetRequest allow action (exempts from lower-priority blocks)
 *
 * @param rules  FocusMe URL rules from daemon
 * @returns      Chrome declarativeNetRequest rules (max 5000, 1-indexed IDs)
 */
export function toDnrRules(rules: FocusMeUrlRule[]): DNRRule[] {
    // Only process active-schedule rules, cap at MAX_DNR_RULES
    const activeRules = rules
        .filter((r) => r.schedule_active)
        .slice(0, MAX_DNR_RULES);

    return activeRules.map((rule, index) => {
        const ruleId = index + 1; // 1-indexed IDs

        if (rule.action === "allow") {
            return buildAllowRule(ruleId, rule);
        }

        return buildBlockRule(ruleId, rule);
    });
}

/**
 * Build a declarativeNetRequest block rule (redirect to blocked page).
 */
function buildBlockRule(id: number, rule: FocusMeUrlRule): DNRRule {
    const urlFilter = buildUrlFilter(rule.domain, rule.path_pattern);

    return {
        id,
        priority: 1,
        action: {
            type: "redirect",
            redirect: {
                extensionPath: `${BLOCKED_PAGE}?domain=${encodeURIComponent(rule.domain)}&plan=${encodeURIComponent(rule.plan_id)}`,
            },
        },
        condition: {
            urlFilter,
            resourceTypes: ["main_frame", "sub_frame"],
        },
    };
}

/**
 * Build a declarativeNetRequest allow rule (exempts from lower-priority blocks).
 */
function buildAllowRule(id: number, rule: FocusMeUrlRule): DNRRule {
    const urlFilter = buildUrlFilter(rule.domain, rule.path_pattern);

    return {
        id,
        priority: 2, // Higher than block (1) — allow wins
        action: {
            type: "allow",
        },
        condition: {
            urlFilter,
            resourceTypes: ["main_frame", "sub_frame"],
        },
    };
}

/**
 * Build a declarativeNetRequest urlFilter from domain and optional path pattern.
 *
 * Conversion rules:
 *   - domain only: `||domain|`  (the trailing `|` is the separator anchor —
 *     prevents "reddit.com" from matching "reddit.community")
 *   - domain + path: `||domain/path/pattern`
 *   - wildcard subdomain: if domain starts with `*.`, uses `||*.rest|`
 *
 * @param domain       Domain string, e.g., "reddit.com" or "*.youtube.com"
 * @param pathPattern  Optional path, e.g., "/r/gaming" or "/shorts/*"
 * @returns            urlFilter string
 */
function buildUrlFilter(domain: string, pathPattern?: string): string {
    // || = domain anchor (matches domain at any position in URL)
    let filter = `||${domain}`;

    if (pathPattern) {
        // Normalise path: ensure it starts with /
        const normalizedPath = pathPattern.startsWith("/") ? pathPattern : `/${pathPattern}`;
        filter += normalizedPath;
        // No trailing separator when path is specified — allow wildcard matching
    } else {
        // Trailing | = separator anchor — ensures exact domain boundary
        // Without this, "reddit.com" could match "reddit.community.example.com"
        filter += "|";
    }

    return filter;
}

// ============ MV2 Conversion (Firefox) ============

/**
 * Convert FocusMe URL rules to Firefox MV2 webRequest blocking patterns.
 *
 * webRequest uses glob-style URL patterns:
 *   - `*://domain/*`      — matches http + https on domain
 *   - `*://*.domain/*`    — matches all subdomains
 *
 * @param rules  FocusMe URL rules from daemon
 * @returns      webRequest URL patterns for use with webRequest.onBeforeRequest
 */
export function toWebRequestPatterns(rules: FocusMeUrlRule[]): WebRequestPattern[] {
    return rules
        .filter((r) => r.schedule_active)
        .map((rule) => {
            const urls = buildWebRequestUrls(rule.domain, rule.path_pattern);
            return {
                urls,
                domain: rule.domain,
                action: rule.action,
                plan_id: rule.plan_id,
            };
        });
}

/**
 * Build webRequest URL match patterns from domain and optional path pattern.
 */
function buildWebRequestUrls(domain: string, pathPattern?: string): string[] {
    const path = pathPattern || "/*";
    const normalizedPath = path.startsWith("/") ? path : `/${path}`;

    // Handle wildcard domain (*.domain)
    if (domain.startsWith("*.")) {
        const baseDomain = domain.slice(2);
        return [
            `*://${baseDomain}${normalizedPath}`,
            `*://*.${baseDomain}${normalizedPath}`,
        ];
    }

    return [
        `*://${domain}${normalizedPath}`,
        `*://*.${domain}${normalizedPath}`, // Include subdomains
    ];
}

// ============ Utility Functions ============

/**
 * Deduplicate rules that target the same domain + path combination.
 * Block action takes precedence over allow when duplicates exist.
 *
 * @param rules  Raw FocusMe URL rules (may contain duplicates)
 * @returns      Deduplicated rules
 */
export function deduplicateRules(rules: FocusMeUrlRule[]): FocusMeUrlRule[] {
    const seen = new Map<string, FocusMeUrlRule>();

    for (const rule of rules) {
        const key = `${rule.domain}|${rule.path_pattern || ""}`;
        if (!seen.has(key)) {
            seen.set(key, rule);
        }
        // If duplicate, block takes precedence over allow
        else if (rule.action === "block") {
            seen.set(key, rule);
        }
    }

    return Array.from(seen.values());
}

/**
 * Validate a domain pattern.
 * Max 253 characters per RFC 1035 / JSON schema validation.
 *
 * @param domain  Domain string to validate
 * @returns       true if the domain is syntactically valid
 */
export function isValidDomain(domain: string): boolean {
    if (domain.length > 253 || domain.length === 0) return false;

    // Allow wildcard prefix *.
    const baseDomain = domain.startsWith("*.") ? domain.slice(2) : domain;

    if (baseDomain.length === 0) return false;

    // RFC 1035: labels separated by dots, each label 1-63 chars, alphanumeric + hyphen
    return /^[a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(\.[a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$/.test(baseDomain);
}
