#!/usr/bin/env bash
# Standard starbreaker-ui check battery (docs/ui-process-improvements.md item 4).
#
# Default (TDD tier) — run after every red/green cycle:
#   example compile check + ui lib tests + manifest_live_ir_guard +
#   line_count_guard
#
# --full (workstream-boundary tier) — adds:
#   manifest_snapshot_regression + manifest_visual_regression suites,
#   freeze + artifact validators, starbreaker-3d lib tests, and the
#   font-size harness (docs/ui-font-size-harness.md) against UI_CHECK_SCENE.
#
# Environment:
#   UI_CHECK_SCENE   scene.json used for the --full font harness replay.
#                    Default: the Clipper LOD1 interior scene (it carries the
#                    medical/door/annunciator bindings the font baseline
#                    covers; the LOD0 cockpit scene does NOT).
set -euo pipefail
cd "$(dirname "$0")/.."

FULL=0
for arg in "$@"; do
  case "$arg" in
    --full) FULL=1 ;;
    -h|--help)
      sed -n '2,16p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *) echo "unknown argument: $arg (try --help)" >&2; exit 64 ;;
  esac
done

step() { echo; echo "==> $*"; }

# Examples are diagnostics tooling; a broken example otherwise stays unnoticed
# until someone reaches for it mid-investigation (ledger item 26).
step "starbreaker-ui examples compile"
cargo check -p starbreaker-ui --examples

step "starbreaker-ui lib tests"
cargo test -p starbreaker-ui --lib

step "manifest_live_ir_guard + line_count_guard"
cargo test -p starbreaker-ui --test manifest_live_ir_guard --test line_count_guard

if [[ "$FULL" == 1 ]]; then
  # Early staleness visibility (ledger item 30): the visual guard hard-fails
  # when the test binary is >30min newer than the export stamp — surface the
  # stamp age BEFORE minutes of suites run, so a needed re-export (~50s)
  # happens first. Warning only; the in-guard check stays authoritative.
  STAMP="$HOME/projects/scorg_tools/ships/Data/UI/Generated/.export_stamp.json"
  if [[ -f "$STAMP" ]]; then
    AGE_MIN=$(( ( $(date +%s) - $(python3 -c "import json;print(json.load(open('$STAMP'))['written_at_epoch_s'])") ) / 60 ))
    echo "export stamp age: ${AGE_MIN}min"
    if (( AGE_MIN > 30 )); then
      echo "WARNING: export stamp is ${AGE_MIN}min old — if the ui test binaries rebuild now, the staleness guard WILL fail; re-export first (~50s)." >&2
    fi
  else
    echo "WARNING: no export stamp at $STAMP — the visual guard will fail unless game data is absent (skip path)." >&2
  fi

  step "manifest_snapshot_regression + manifest_visual_regression"
  cargo test -p starbreaker-ui --test manifest_snapshot_regression --test manifest_visual_regression

  step "validate_ui_snapshot_freeze"
  bash scripts/validate_ui_snapshot_freeze.sh

  step "validate_ui_regression_artifacts --quick"
  bash scripts/validate_ui_regression_artifacts.sh --quick

  step "starbreaker-3d lib tests"
  cargo test -p starbreaker-3d --lib

  SCENE="${UI_CHECK_SCENE:-$HOME/projects/scorg_tools/ships/Packages/DRAK Clipper_LOD1_TEX2/scene.json}"
  if [[ -f "$SCENE" ]]; then
    step "font-size harness (scene: $SCENE)"
    DUMP="$(mktemp /tmp/ui_check_fontdump.XXXXXX.tsv)"
    SB_UI_FONT_DUMP=1 cargo run -q -p starbreaker -- ui render --scene "$SCENE" \
      --out-dir "$(mktemp -d /tmp/ui_check_render.XXXXXX)" 2>&1 \
      | grep '^FONTDUMP' > "$DUMP"
    python3 scripts/font_size_check.py "$DUMP"
  else
    echo "SKIP font harness: scene not found: $SCENE (set UI_CHECK_SCENE)" >&2
  fi
fi

echo
echo "ui_check: ALL GREEN"
