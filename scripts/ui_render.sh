#!/usr/bin/env bash
# Build-then-replay wrapper (crates/starbreaker-ui/docs/ui-process-improvements.md Part C).
#
# Twice in one session a render silently used a STALE binary because the
# preceding `cargo build` ran in the wrong working directory (background
# shells reset their cwd) — the renders looked identical and nearly
# mis-adjudicated a fix as ineffective. This wrapper resolves the repo root
# from its own location, always builds first, and prints the binary mtime so
# staleness is visible.
#
# Usage:
#   bash scripts/ui_render.sh --helper <name> [--lod 0|1] [--scene <scene.json>] \
#       [--out <dir>] [--ir]
#
# Scene: --scene wins; else --lod picks the LOD0/LOD1 Clipper scene; else the
# LOD is derived from the helper — cockpit MFD render-to-texture screens
# (`*_RTT`, e.g. Screen_Left_Lower_RTT) use LOD0, interior usables (medical/
# door/annunciator) use LOD1 (dossier §3). Out defaults to /tmp/ui_render/<helper>.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

SCENE_LOD0="$HOME/projects/scorg_tools/ships/Packages/DRAK Clipper_LOD0_TEX0/scene.json"
SCENE_LOD1="$HOME/projects/scorg_tools/ships/Packages/DRAK Clipper_LOD1_TEX2/scene.json"
SCENE=""      # --scene overrides LOD selection
LOD=""        # --lod 0|1; else derived from the helper
HELPER=""
OUT=""
DUMP_IR=0
while [[ $# -gt 0 ]]; do
    case "$1" in
        --helper) HELPER="$2"; shift 2 ;;
        --scene) SCENE="$2"; shift 2 ;;
        --lod) LOD="$2"; shift 2 ;;
        --out) OUT="$2"; shift 2 ;;
        --ir) DUMP_IR=1; shift ;;
        -h|--help) sed -n '2,18p' "$0"; exit 0 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done
if [[ -z "$HELPER" ]]; then
    echo "error: --helper is required (see the screen dossier in crates/starbreaker-ui/docs/ui-reference.md)" >&2
    exit 2
fi
# Scene: explicit --scene wins; else pick the LOD0/LOD1 Clipper scene. The LOD
# defaults from the helper — `*_RTT` cockpit MFDs are in LOD0, everything else
# (interior usables) in LOD1 (dossier §3). Override with --lod 0|1.
if [[ -z "$SCENE" ]]; then
    if [[ -z "$LOD" ]]; then
        if [[ "$HELPER" == *_RTT ]]; then LOD=0; else LOD=1; fi
    fi
    case "$LOD" in
        0) SCENE="$SCENE_LOD0" ;;
        1) SCENE="$SCENE_LOD1" ;;
        *) echo "error: --lod must be 0 or 1 (got '$LOD')" >&2; exit 2 ;;
    esac
fi
echo "==> scene (LOD${LOD:-?}): $SCENE"
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
