#!/usr/bin/env python3
"""Region-by-region render-vs-reference comparison for UI parity review
(docs/ui-process-improvements.md item 2).

Usage:
    python3 scripts/ui_compare.py <render.png> <reference.png> \
        [--regions <preset>] [--out-dir <dir>]

The reference is auto-scaled to the render's width BEFORE cropping, so
mismatched capture resolutions (e.g. a 1959x1513 screenshot vs a 1600x1200
render) compare correctly. Region boxes are in the RENDER's pixel space.

Writes <out-dir>/cmp_full.png plus one cmp_<region>.png per preset region
(render on top / left, reference below / right). Review phases use this
script exclusively — no ad-hoc crop snippets.

Presets live in REGION_PRESETS below; add one row per screen as it is
worked. List available presets with --regions list.
"""

import argparse
import os
import sys

from PIL import Image

# Boxes are (left, top, right, bottom) in the render's pixel space
# (1600x1200 for the Clipper RTT screens).
REGION_PRESETS = {
    "power": {  # Screen_Left_Lower_RTT vs reference Screen_Left_Lower_RTT.png
        "emissions": (40, 0, 1560, 170),
        "columns": (430, 170, 1430, 1030),
        "scrollbar": (430, 1000, 1430, 1080),
        "output_card": (60, 170, 560, 620),
        "battery_card": (60, 600, 560, 1060),
        "footer": (0, 1060, 1600, 1200),
    },
    "target": {  # Screen_Right_Upper_RTT vs reference Screen_Right_Upper_RTT.png
        "status_band": (0, 0, 1600, 420),
        "chevrons": (150, 260, 1450, 700),
        "footer": (0, 950, 1600, 1200),
    },
}


def harness_error(msg):
    print(f"HARNESS ERROR: {msg}", file=sys.stderr)
    sys.exit(2)


def stack(a, b, gap=8, bg=(15, 15, 15)):
    """Side-by-side for tall crops, stacked for wide crops."""
    if a.width >= a.height:
        canvas = Image.new("RGB", (max(a.width, b.width), a.height + b.height + gap), bg)
        canvas.paste(a, (0, 0))
        canvas.paste(b, (0, a.height + gap))
    else:
        canvas = Image.new("RGB", (a.width + b.width + gap, max(a.height, b.height)), bg)
        canvas.paste(a, (0, 0))
        canvas.paste(b, (a.width + gap, 0))
    return canvas


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("render")
    ap.add_argument("reference")
    ap.add_argument("--regions", default=None, help="preset name (or 'list')")
    ap.add_argument("--out-dir", default="/tmp/ui_compare")
    args = ap.parse_args()

    if args.regions == "list":
        for name, regions in REGION_PRESETS.items():
            print(f"{name}: {', '.join(regions)}")
        return

    if not os.path.isfile(args.render):
        harness_error(f"render not found: {args.render}")
    if not os.path.isfile(args.reference):
        harness_error(f"reference not found: {args.reference}")

    render = Image.open(args.render).convert("RGB")
    ref = Image.open(args.reference).convert("RGB")
    # Scale the reference into the render's pixel space.
    ref = ref.resize((render.width, round(ref.height * render.width / ref.width)))

    os.makedirs(args.out_dir, exist_ok=True)
    written = []

    full = stack(
        render.resize((render.width // 2, render.height // 2)),
        ref.resize((ref.width // 2, ref.height // 2)),
    )
    path = os.path.join(args.out_dir, "cmp_full.png")
    full.save(path)
    written.append(path)

    if args.regions:
        preset = REGION_PRESETS.get(args.regions)
        if preset is None:
            harness_error(f"unknown preset '{args.regions}' (try --regions list)")
        for name, box in preset.items():
            l, t, r, b = box
            r = min(r, render.width)
            a = render.crop((l, t, r, min(b, render.height)))
            bcrop = ref.crop((l, t, r, min(b, ref.height)))
            path = os.path.join(args.out_dir, f"cmp_{name}.png")
            stack(a, bcrop).save(path)
            written.append(path)

    for p in written:
        print(p)


if __name__ == "__main__":
    main()
