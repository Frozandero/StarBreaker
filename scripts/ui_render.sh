#!/usr/bin/env bash
# Build-then-replay wrapper (docs/ui-process-improvements.md Part C).
#
# Twice in one session a render silently used a STALE binary because the
# preceding `cargo build` ran in the wrong working directory (background
# shells reset their cwd) — the renders looked identical and nearly
# mis-adjudicated a fix as ineffective. This wrapper resolves the repo root
# from its own location, always builds first, and prints the binary mtime so
# staleness is visible.
#
# Usage:
#   bash scripts/ui_render.sh --helper <name> [--scene <scene.json>] \
#       [--out <dir>] [--ir]
#
# Defaults: scene = the LOD1 Clipper scene (medical/door/annunciator/power
# bindings), out = /tmp/ui_render/<helper>.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

SCENE="$HOME/projects/scorg_tools/ships/Packages/DRAK Clipper_LOD1_TEX2/scene.json"
HELPER=""
OUT=""
DUMP_IR=0
while [[ $# -gt 0 ]]; do
    case "$1" in
        --helper) HELPER="$2"; shift 2 ;;
        --scene) SCENE="$2"; shift 2 ;;
        --out) OUT="$2"; shift 2 ;;
        --ir) DUMP_IR=1; shift ;;
        -h|--help) sed -n '2,16p' "$0"; exit 0 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done
if [[ -z "$HELPER" ]]; then
    echo "error: --helper is required (see the screen dossier in docs/ui-reference.md)" >&2
    exit 2
fi
OUT="${OUT:-/tmp/ui_render/$HELPER}"

echo "==> cargo build (debug)"
cargo build
echo "==> binary: $(stat -c '%y' target/debug/starbreaker | cut -d. -f1)"

rm -rf "$OUT"
ARGS=(ui render --scene "$SCENE" --out-dir "$OUT" --helper "$HELPER")
if [[ "$DUMP_IR" -eq 1 ]]; then
    ARGS+=(--dump-ir-dir "$OUT/ir")
fi
./target/debug/starbreaker "${ARGS[@]}"
ls -1 "$OUT"
