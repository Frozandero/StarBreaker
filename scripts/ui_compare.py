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
    "annunciator": {  # Screen_Annunciator_L (1920x432) vs Screen_Annunciator_L.png
        "pwr": (15, 15, 370, 420),
        "wpn": (399, 15, 754, 420),
        "thr": (783, 15, 1138, 420),
        "shld": (1167, 15, 1522, 420),
        "cool": (1551, 15, 1906, 420),
    },
    "door": {  # i_door_small_drak (1920x1080) vs Door-closed.png
        "header": (0, 0, 1920, 330),
        "status": (0, 330, 1920, 860),
        "bottom": (0, 860, 1920, 1080),
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


def region_stats(name, render_crop, ref_crop):
    """Photometric comparison of a region: bright/dark pixel means and
    R-normalised ratios for both images.

    This is the method that diagnosed the linear-light compositing gap and
    identified the MissionObjectives icon slot: hue lives in the normalised
    ratios (capture casts attenuate G/B roughly uniformly per capture;
    bloom near bright elements lifts B). Judge HUE from ratios, never raw
    values, and estimate the capture's cast from a known anchor (footer
    text = Base, pip slabs = Bright) on the SAME reference before judging
    an unknown colour.
    """
    import numpy as np

    def stats(img):
        a = np.asarray(img, dtype=float)
        out = {}
        for label, mask in (
            ("bright", a.max(axis=2) > 110),
            ("dark", a.max(axis=2) < 60),
        ):
            px = a[mask]
            if len(px) == 0:
                out[label] = None
                continue
            mean = px.mean(axis=0)
            ratio = mean / mean[0] if mean[0] > 1e-6 else mean
            out[label] = (mean.round(0), ratio.round(2), len(px))
        return out

    r, f = stats(render_crop), stats(ref_crop)
    print(f"[{name}]")
    for label in ("bright", "dark"):
        for side, s in (("render", r[label]), ("ref   ", f[label])):
            if s is None:
                print(f"  {label:>6} {side}: (none)")
            else:
                mean, ratio, n = s
                print(
                    f"  {label:>6} {side}: mean=({mean[0]:.0f},{mean[1]:.0f},{mean[2]:.0f})"
                    f" ratio=(1,{ratio[1]:.2f},{ratio[2]:.2f}) n={n}"
                )


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("render")
    ap.add_argument("reference")
    ap.add_argument("--regions", default=None, help="preset name (or 'list')")
    ap.add_argument("--out-dir", default="/tmp/ui_compare")
    ap.add_argument(
        "--stats",
        action="store_true",
        help="print per-region bright/dark pixel means + R-normalised ratios "
        "for render and reference (the photometric review method)",
    )
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
            if args.stats:
                region_stats(name, a, bcrop)
    elif args.stats:
        region_stats("full", render, ref)

    for p in written:
        print(p)


if __name__ == "__main__":
    main()
