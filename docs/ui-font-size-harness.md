# UI font-size harness

> Satellite doc — the authoritative process is `docs/ui-workflow.md`; commands/tools/data live in `docs/ui-reference.md`.

A diagnostic tool for auditing the **final rendered size of every text element**
on the UI screens, and regression-checking it against a frozen baseline that is
within ~1–2% of the in-game reference screenshots.

Use it whenever you touch anything that can affect text size — the font-size
resolver (`ui_ir`), the SWF/TTF draw calibration (`ir_compose`, `text/`), font
records, brand styles, or layout scaling — to prove you did **not** regress the
platinum/gold screens.

## Why it exists

UI text size is produced by a chain of per-style/per-context factors
(`STANDARD_TEXTFIELD_NOMINAL_FONT_SCALE = 0.72`, `SWF_TEXT_RENDER_SIZE_CALIBRATION
= 0.84`, the rect-based fallback ladder, `units_per_em = ascent − descent`, …).
These constants are **empirically tuned** to match the engine's own Scaleform
text layout (which lives in engine code, not in the DataCore/P4K data) and land
the platinum/gold screens within 1–2% of the references. Attempts to replace
them with a single data-backed rule have been proven — with this harness — to
regress the auto-fit / raw-size / width-fit paths (see
[`ui-architecture-runbook.md`](ui-architecture-runbook.md) and the project
memory `font-sizing-constants-load-bearing`). So the harness is the guard: it
makes the *visible* effect of any font change measurable per element, instead of
relying on eyeballing screenshots.

## Components

| Piece | Location | Role |
|---|---|---|
| `SB_UI_FONT_DUMP` emitter | `crates/starbreaker-ui/src/text/swf_draw.rs` | When the env var is set, prints one `FONTDUMP` line per rendered text element. No effect otherwise. |
| dump context | `crates/starbreaker-ui/src/text/mod.rs` (`set_font_dump_ctx`) + `ir_compose/.../part_04.part` | Keys each dump line to its source canvas + node. |
| baseline | `crates/starbreaker-ui/tests/fixtures/font_size_baseline.tsv` | Committed (1–2%-accurate) rendered sizes for the 4 targets. |
| checker | `scripts/font_size_check.py` | Diffs a new dump against the baseline; per-element PASS/FAIL; non-zero exit on drift. |

The harness adds **no behavioural change** — it emits nothing and costs nothing
unless `SB_UI_FONT_DUMP` is set.

### Dump line format

```
FONTDUMP \t canvas \t node \t font \t size_px \t visible_px \t em \t text
```

- `size_px` — the size handed to the rasteriser (after IR + render-side scaling).
- `visible_px` — **the held invariant**: the actual rendered glyph cap height in
  image pixels (`glyph_bbox × size_px / units_per_em`). This folds in *every*
  render-side factor (calibration, em choice, glyph metrics), so matching it
  means the on-screen text is unchanged.
- `em` — the font's `units_per_em` as the renderer currently computes it.

## Regression targets

The baseline covers the four registered platinum/gold canvases
(`crates/starbreaker-ui/tests/fixtures/ui_regression_freeze.json`):

| Canvas | Tier | Screen |
|---|---|---|
| `I_Med_MedicalBed_A` | platinum | Medical bed (ui_target_a) |
| `I_Med_MedicalEndOfBed_A` | platinum | Medical end-of-bed (ui_target_b) |
| `I_Door_Small_DRAK` | gold | Small door (`CLOSED`) |
| `H_Eng_Annunciator_Master_Left` | gold | Engineering annunciator |

## Usage

```bash
cd StarBreaker
export SC_DATA_P4K="<path to Data.p4k>"

# 1. Capture a dump from a full Clipper render (any --scene with these bindings).
SB_UI_FONT_DUMP=1 cargo run -q -p starbreaker -- ui render \
    --scene "<export>/Packages/DRAK Clipper_LOD0_TEX0/scene.json" \
    --out-dir /tmp/fontcheck 2>&1 | grep '^FONTDUMP' > /tmp/font_dump.tsv

# 2. Check it against the frozen baseline (default tolerance 2.5%).
python3 scripts/font_size_check.py /tmp/font_dump.tsv
#   -> "baseline elements: 26  matched within 2.5%: 26 ... PASS"

# Optional: looser tolerance, or a different baseline file.
python3 scripts/font_size_check.py /tmp/font_dump.tsv 4
python3 scripts/font_size_check.py /tmp/font_dump.tsv 2.5 /path/to/other_baseline.tsv
```

A failing element prints its exact drift, e.g.:

```
  -28.6%  H_Eng_Annunciator_Master_Left Text Item 0   base=  71.74 new=  51.21  'PWR'
FAIL (15 drifted)
```

The script exits non-zero on any drift, so it can gate a change before commit.

## Re-baselining (after an intentional, approved size change)

Only do this when a size change is *deliberate* and verified against the in-game
references — never to silence a drift.

```bash
SB_UI_FONT_DUMP=1 cargo run -q -p starbreaker -- ui render \
    --scene "<export>/Packages/DRAK Clipper_LOD0_TEX0/scene.json" \
    --out-dir /tmp/fontcheck 2>&1 | grep '^FONTDUMP' \
  | grep -E 'I_Med_MedicalBed_A|I_Med_MedicalEndOfBed_A|I_Door_Small_DRAK|H_Eng_Annunciator_Master_Left' \
  | sed 's/BuildingBlocks_Canvas\.//' | sort \
  > crates/starbreaker-ui/tests/fixtures/font_size_baseline.tsv
```

This is the *visible-size* companion to the artifact/byte-hash freeze
(`docs/ui-workflow.md` §7): the hash freeze proves pixels are identical; this
proves text *sizes* are within tolerance even when other pixels legitimately
change.

## Notes / caveats

- `visible_px` is measured from the glyph bounding box, so a per-element key with
  unusually short text (one wide/narrow glyph) can read a percent or two off the
  style's nominal — keep the default tolerance at ≥2.5%.
- The dump only covers SWF-font text (`draw_swf_font`). TTF-fallback text is not
  emitted; on the Clipper every element resolves to an SWF font, so coverage is
  complete there.
- Keyed by `(canvas, node, text)`; if a binding renders the same node with
  several different strings (e.g. the three medical option cards) each string is
  a separate row.
