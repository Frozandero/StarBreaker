#!/usr/bin/env python3
"""Contamination-guarded pixel measurer for UI parity adjudication
(plan P1.2, ledger items 19/21/23).

Measures glyph cap heights and colours inside an element box of a render or
reference capture, replacing the throwaway /tmp python of past arcs.

Usage:
    python3 scripts/ui_measure.py <image.png> --box x0,y0,x1,y1 [options]
    python3 scripts/ui_measure.py <image.png> --ir <ir.json> --node <id> [options]

Options:
    --delta N              bright threshold = median luminance + N (default 30)
    --anchor x0,y0,x1,y1   also measure an anchor region (haze calibration)
    --anchor-rgb r,g,b     the anchor's TRUE colour (e.g. the palette Base
                           slot) — enables haze-corrected ratios

Output: one JSON object on stdout.

  glyph_runs   Bright-row runs inside the box: rows holding at least one
               pixel above the threshold, grouped into consecutive runs
               {y0, y1, h} (y1 exclusive, image coordinates). The CALLER
               picks the glyph run — the tool only flags suspects: a run is
               "suspect_contamination": true when its bright pixels touch
               BOTH the box's left and right edge columns (a bar/rule
               crossing the box — the footer-bar-line trap, ledger 19) or
               the run abuts the box's top/bottom row (likely truncated by
               the box). Single-edge contact is reported in "touches"
               without raising the flag (a glyph nudging one box edge).
  cap_height   Height of the tallest NON-suspect run (null if none).
  colour       Mean RGB over above-threshold pixels + R-normalised ratios.

Additive-haze photometric model (the refined form of docs/ui-reference.md
§3's method): captures carry a per-capture colour cast plus bloom that adds
a roughly constant offset to the channel ratios in a local region, so

    measured_ratio ≈ true_ratio + haze_offset      (ratio = G/R or B/R)

Measuring a region whose true colour IS known (the anchor: e.g. footer text
= brand Base, pip slabs = Bright) gives
haze_offset = anchor_measured_ratio − anchor_true_ratio, and the element's
corrected ratio is corrected = element_measured_ratio − haze_offset.
Judge hue from corrected ratios, never raw channel values.
"""

import argparse
import json
import sys

from PIL import Image


def parse_box(text):
    parts = [int(p) for p in text.split(",")]
    if len(parts) != 4:
        raise ValueError(f"expected x0,y0,x1,y1 — got {text!r}")
    x0, y0, x1, y1 = parts
    if x1 <= x0 or y1 <= y0:
        raise ValueError(f"empty box {text!r}")
    return x0, y0, x1, y1


def box_from_ir(ir_path, node_id):
    with open(ir_path) as handle:
        doc = json.load(handle)
    for node in doc.get("nodes", []):
        if node.get("id") == node_id:
            rect = node.get("computed_rect") or {}
            return (
                int(rect.get("x", 0)),
                int(rect.get("y", 0)),
                int(rect.get("x", 0) + rect.get("w", 0)),
                int(rect.get("y", 0) + rect.get("h", 0)),
            )
    sys.exit(f"error: node id {node_id} not in {ir_path}")


def luminance(px):
    return (px[0] + px[1] + px[2]) / 3.0


def measure_region(img, box, delta):
    """Bright mask + row runs + colour stats for one box."""
    x0, y0, x1, y1 = box
    width, height = img.size
    x0, y0 = max(0, x0), max(0, y0)
    x1, y1 = min(width, x1), min(height, y1)
    pixels = img.load()

    lums = sorted(
        luminance(pixels[x, y]) for y in range(y0, y1) for x in range(x0, x1)
    )
    median = lums[len(lums) // 2]
    threshold = median + delta

    bright_rows = {}  # y -> (has_left_edge, has_right_edge)
    bright_px = []
    for y in range(y0, y1):
        edge_left = edge_right = False
        any_bright = False
        for x in range(x0, x1):
            px = pixels[x, y]
            if luminance(px) > threshold:
                any_bright = True
                bright_px.append(px)
                if x == x0:
                    edge_left = True
                if x == x1 - 1:
                    edge_right = True
        if any_bright:
            bright_rows[y] = (edge_left, edge_right)

    runs = []
    run_start = None
    prev = None
    for y in sorted(bright_rows):
        if run_start is None:
            run_start = y
        elif y != prev + 1:
            runs.append((run_start, prev + 1))
            run_start = y
        prev = y
    if run_start is not None:
        runs.append((run_start, prev + 1))

    glyph_runs = []
    for ry0, ry1 in runs:
        touches_left = any(bright_rows[y][0] for y in range(ry0, ry1))
        touches_right = any(bright_rows[y][1] for y in range(ry0, ry1))
        crossing_bar = touches_left and touches_right
        truncated = ry0 == y0 or ry1 == y1
        touches = [side for side, hit in (
            ("left", touches_left), ("right", touches_right),
            ("top", ry0 == y0), ("bottom", ry1 == y1),
        ) if hit]
        glyph_runs.append({
            "y0": ry0,
            "y1": ry1,
            "h": ry1 - ry0,
            "touches": touches,
            "suspect_contamination": crossing_bar or truncated,
        })

    colour = None
    if bright_px:
        n = len(bright_px)
        mean_r = sum(p[0] for p in bright_px) / n
        mean_g = sum(p[1] for p in bright_px) / n
        mean_b = sum(p[2] for p in bright_px) / n
        colour = {
            "mean_rgb": [round(mean_r, 2), round(mean_g, 2), round(mean_b, 2)],
            "ratios": ratios_of(mean_r, mean_g, mean_b),
            "pixels": n,
        }

    clean = [r["h"] for r in glyph_runs if not r["suspect_contamination"]]
    return {
        "box": [x0, y0, x1, y1],
        "median_luminance": round(median, 2),
        "threshold": round(threshold, 2),
        "glyph_runs": glyph_runs,
        "cap_height": max(clean) if clean else None,
        "colour": colour,
    }


def ratios_of(r, g, b):
    if r <= 0:
        return None
    return {"g_over_r": round(g / r, 4), "b_over_r": round(b / r, 4)}


def main():
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("image")
    parser.add_argument("--box", help="x0,y0,x1,y1 element box in image pixels")
    parser.add_argument("--ir", help="IR JSON (from ui render --dump-ir-dir)")
    parser.add_argument("--node", type=int, help="IR node id whose rect is the box")
    parser.add_argument("--delta", type=float, default=45.0,
                        help="bright threshold above the box's median luminance")
    parser.add_argument("--anchor", help="x0,y0,x1,y1 anchor region for haze calibration")
    parser.add_argument("--anchor-rgb", help="r,g,b TRUE colour of the anchor region")
    args = parser.parse_args()

    if args.box:
        box = parse_box(args.box)
    elif args.ir and args.node is not None:
        box = box_from_ir(args.ir, args.node)
    else:
        parser.error("need --box or (--ir and --node)")

    img = Image.open(args.image).convert("RGB")
    result = {"image": args.image}
    result.update(measure_region(img, box, args.delta))

    if args.anchor:
        anchor_box = parse_box(args.anchor)
        anchor = measure_region(img, anchor_box, args.delta)
        result["anchor"] = {
            "box": anchor["box"],
            "colour": anchor["colour"],
        }
        if args.anchor_rgb and anchor["colour"] and result["colour"]:
            r, g, b = (float(v) for v in args.anchor_rgb.split(","))
            true_ratios = ratios_of(r, g, b)
            measured = anchor["colour"]["ratios"]
            if true_ratios and measured:
                haze = {
                    "g_over_r": round(measured["g_over_r"] - true_ratios["g_over_r"], 4),
                    "b_over_r": round(measured["b_over_r"] - true_ratios["b_over_r"], 4),
                }
                element = result["colour"]["ratios"]
                result["anchor"]["true_rgb"] = [r, g, b]
                result["anchor"]["haze_offset"] = haze
                result["corrected_ratios"] = {
                    "g_over_r": round(element["g_over_r"] - haze["g_over_r"], 4),
                    "b_over_r": round(element["b_over_r"] - haze["b_over_r"], 4),
                }

    json.dump(result, sys.stdout, indent=2)
    print()


if __name__ == "__main__":
    main()
