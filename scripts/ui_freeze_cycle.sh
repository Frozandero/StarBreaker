#!/usr/bin/env bash
# One-command artifact freeze cycle (docs/ui-process-improvements.md Part C).
#
# Runs the full documented flow that was previously seven hand-typed steps
# (and was run five times in one session): release build -> full export ->
# stale-comparison cleanup -> artifact freeze -> both validators ->
# ui_check.sh --full. All paths are resolved from the repo root, so it is
# immune to the wrong-cwd/stale-binary trap.
#
# Usage:
#   bash scripts/ui_freeze_cycle.sh --approver <name> --reason "<text>" \
#       [--skip-export]   # only when the export is known-current
#
# The IR snapshot freeze (freeze_ui_snapshot_ir.sh) is intentionally NOT
# included: it is only needed when IR semantics changed, and it prints a
# delta that must be read and accounted for before continuing.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

APPROVER=""
REASON=""
SKIP_EXPORT=0
while [[ $# -gt 0 ]]; do
    case "$1" in
        --approver) APPROVER="$2"; shift 2 ;;
        --reason) REASON="$2"; shift 2 ;;
        --skip-export) SKIP_EXPORT=1; shift ;;
        -h|--help) sed -n '2,16p' "$0"; exit 0 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done
if [[ -z "$APPROVER" || -z "$REASON" ]]; then
    echo "error: --approver and --reason are required" >&2
    exit 2
fi

EXPORT_ROOT="$HOME/projects/scorg_tools/ships"

echo "==> cargo build --release"
cargo build --release

if [[ "$SKIP_EXPORT" -eq 0 ]]; then
    echo "==> full export (drak_clipper; PNGs land near the END of the run)"
    ./target/release/starbreaker entity export drak_clipper "$EXPORT_ROOT" --kind decomposed
else
    echo "==> export SKIPPED (--skip-export)"
fi

# Stale '*-current.png' comparison outputs from earlier validation
# mismatches make the validator fail with 'undeclared artifact produced in
# freeze scope'; they are debris, not products of this freeze.
echo "==> cleaning stale comparison outputs"
rm -f test-artifacts/ui/*-current.png

echo "==> freezing artifacts"
bash scripts/freeze_ui_regression_artifacts.sh --approver "$APPROVER" --reason "$REASON"

echo "==> validating"
bash scripts/validate_ui_snapshot_freeze.sh
bash scripts/validate_ui_regression_artifacts.sh

echo "==> full battery"
bash scripts/ui_check.sh --full

echo "ui_freeze_cycle: COMPLETE (remember: the commit message must account for the freeze delta/hash scope)"
