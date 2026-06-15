#!/usr/bin/env bash
# One-command artifact freeze cycle (crates/starbreaker-ui/docs/ui-process-improvements.md Part C).
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
    echo "==> full export (drak_clipper, --lod 0; PNGs land near the END of the run)"
    # --lod 0 is REQUIRED: LOD1 CULLS the small cockpit HUD screens (g-force,
    # velocity ball/num, countermeasures, …), so a re-freeze of any of them off a
    # plain (LOD1) export would freeze a stale/missing PNG. Matches the canonical
    # guard-export and generate_ui_regression_artifacts.sh (reference §1/§5, ledger
    # 47/62).
    ./target/release/starbreaker entity export drak_clipper "$EXPORT_ROOT" \
        --kind decomposed --lod 0 --mip 0 --materials all
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

# Guard (ledger 44): this cycle freezes IMAGE artifacts only; the IR-snapshot
# freeze is a SEPARATE script. A manifest target with no IR-snapshot baseline
# makes the validator below fail opaquely ("snapshot freeze ids do not match
# manifest ids"). Detect the drift here and point at the fix.
MISSING_SNAPSHOT=$(python3 - \
    crates/starbreaker-ui/tests/fixtures/ui_ir/ui_snapshot_manifest.json \
    crates/starbreaker-ui/tests/fixtures/ui_ir/ui_snapshot_freeze.json <<'PY'
import json, sys
def ids(path):
    t = json.load(open(path)).get("targets", [])
    return set(t.keys()) if isinstance(t, dict) else {x.get("id") for x in t if isinstance(x, dict)}
print(" ".join(sorted(ids(sys.argv[1]) - ids(sys.argv[2]))))
PY
)
if [[ -n "${MISSING_SNAPSHOT// }" ]]; then
    echo "error: manifest target(s) with no IR-snapshot baseline: ${MISSING_SNAPSHOT}" >&2
    echo "  run first:  bash scripts/freeze_ui_snapshot_ir.sh --approver ${APPROVER} --reason \"…\"" >&2
    echo "  (and bump the count assert in tests/manifest_visual_regression.rs for a new target)" >&2
    exit 3
fi

echo "==> validating"
bash scripts/validate_ui_snapshot_freeze.sh
bash scripts/validate_ui_regression_artifacts.sh

echo "==> full battery"
bash scripts/ui_check.sh --full

echo "ui_freeze_cycle: COMPLETE (remember: the commit message must account for the freeze delta/hash scope)"
