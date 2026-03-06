#!/usr/bin/env bash
# ============================================================
# FILE:        cargo-audit.sh
# MODULE:      Phase 5 — Cloud Backend > Security Audit Script
# TASK:        Session 9 A6 (dependency audit)
# PLATFORM:    linux / macOS (CI + local)
# AUTHOR:      FocusMe Co-Pilot (Claude Opus)
# GENERATED:   Session 9
# DEPENDENCIES: cargo-audit (cargo install cargo-audit)
# ============================================================
#
# Run from the backend/ directory:
#   chmod +x cargo-audit.sh
#   ./cargo-audit.sh
#
# This script:
# 1. Checks that cargo-audit is installed
# 2. Runs cargo audit with --deny warnings (fails on any advisory)
# 3. Saves full report to audit_report.txt
# 4. Exits with non-zero code if vulnerabilities found (CI-friendly)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "=== FocusMe Backend — Dependency Security Audit ==="
echo "Date: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
echo "Directory: $(pwd)"
echo ""

# Check cargo-audit is installed
if ! command -v cargo-audit &> /dev/null && ! cargo audit --version &> /dev/null 2>&1; then
    echo "ERROR: cargo-audit is not installed."
    echo "Install with: cargo install cargo-audit"
    exit 1
fi

# Run audit
echo "Running cargo audit..."
echo ""

if cargo audit --deny warnings 2>&1 | tee audit_report.txt; then
    echo ""
    echo "✅ No vulnerabilities found."
    echo "Report saved to: audit_report.txt"
    exit 0
else
    echo ""
    echo "❌ Vulnerabilities detected! Review audit_report.txt for details."
    echo ""
    echo "To fix:"
    echo "  1. cargo update -p <affected-crate>"
    echo "  2. If no fix available: cargo audit --ignore RUSTSEC-YYYY-NNNN"
    echo "  3. Document any ignored advisories in this script with justification"
    exit 1
fi
