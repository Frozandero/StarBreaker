#!/usr/bin/env python3
"""UI font-size regression check.

Compares an `SB_UI_FONT_DUMP` capture against the frozen baseline
(`crates/starbreaker-ui/tests/fixtures/font_size_baseline.tsv`), which holds the
committed rendered size of every text element on the four platinum/gold Clipper
screens and is within ~1-2% of the in-game references.

See `docs/ui-font-size-harness.md` for the full workflow.

Usage:
    # 1. capture a dump from any render (stderr -> file):
    SB_UI_FONT_DUMP=1 cargo run -q -p starbreaker -- ui render \
        --scene "<scene.json>" --out-dir /tmp/x 2>&1 \
        | grep '^FONTDUMP' > /tmp/font_dump.tsv

    # 2. check it:
    python3 scripts/font_size_check.py /tmp/font_dump.tsv [tolerance_pct] [baseline.tsv]

Each FONTDUMP line:
    FONTDUMP \\t canvas \\t node \\t font \\t size_px \\t visible_px \\t em \\t text
The held invariant is VISIBLE_px (final rendered text height), keyed by
(canvas, node, text). Reports every element that drifts beyond `tolerance_pct`
(default 2.5%) and exits non-zero on any drift, so it can gate CI.
"""
import os
import sys

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DEFAULT_BASELINE = os.path.join(
    REPO, "crates/starbreaker-ui/tests/fixtures/font_size_baseline.tsv"
)
# The four platinum/gold regression targets (see docs/ui-regression-policy.md).
TARGETS = (
    "I_Med_MedicalBed_A",          # platinum  (ui_target_a)
    "I_Med_MedicalEndOfBed_A",     # platinum  (ui_target_b)
    "I_Door_Small_DRAK",           # gold      (clipper_small_door)
    "H_Eng_Annunciator_Master_Left",  # gold   (eng_annunciator_master_left)
)


def load(path):
    """Return {(canvas, node, text): visible_px} for the target canvases."""
    out = {}
    for line in open(path):
        cells = [c for c in line.rstrip("\n").split("\t") if c != "FONTDUMP"]
        if len(cells) < 7:
            continue
        canvas, node, _font, _size, visible, _em, text = cells[:7]
        canvas = canvas.replace("BuildingBlocks_Canvas.", "")
        if canvas not in TARGETS:
            continue
        out[(canvas, node, text)] = float(visible)
    return out


def main():
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    new_path = sys.argv[1]
    tol = float(sys.argv[2]) if len(sys.argv) > 2 else 2.5
    baseline = sys.argv[3] if len(sys.argv) > 3 else DEFAULT_BASELINE

    base = load(baseline)
    new = load(new_path)
    bad = []
    for key, bv in sorted(base.items()):
        nv = new.get(key)
        if nv is None:
            bad.append((key, bv, None, None))
        else:
            pct = (nv - bv) / bv * 100.0
            if abs(pct) > tol:
                bad.append((key, bv, nv, pct))

    print(f"baseline elements: {len(base)}  matched within {tol}%: {len(base) - len(bad)}")
    for (canvas, node, text), bv, nv, pct in bad:
        if nv is None:
            print(f"  MISSING  {canvas:26} {node:20} base={bv:7.2f}  '{text[:24]}'")
        else:
            print(f"  {pct:+6.1f}%  {canvas:26} {node:20} base={bv:7.2f} new={nv:7.2f}  '{text[:24]}'")
    print("PASS" if not bad else f"FAIL ({len(bad)} drifted)")
    sys.exit(0 if not bad else 1)


if __name__ == "__main__":
    main()
