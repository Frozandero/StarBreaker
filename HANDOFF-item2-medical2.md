# Hand-off: Clipper medical-bed item 2 (header gap) + Medical2 issues

Temporary working doc. Picks up the DRAK Clipper medical UI parity work.
Companion to project memory `clipper-medical-bed-parity.md` /
`caps-reduction-removed.md`.

---

## Context / what's already done (medical bed, `I_Med_MedicalBed_A`)

Canvas GUID `534bab84-299b-479a-a4af-4469df112ea7`, brand **s_bioc**, render is
1920×1080 so **IR units = screen pixels 1:1**.

**7 fixes committed** on branch `feature/ui`:
- `52e4d3e37` — caps fudge removed (full brand nominal sizing), header colour
  (Drake Clipper + T3 = Base light-blue), welcome band 0.375→0.381 (→40px),
  MedGel label→value 31→28 + meter pinned to value baseline (→22).
- `87d22f329` — procedural close-button X ~2px smaller + thinner stroke.

**Still pending:** item 2 (below), then **re-freeze** the medical bed +
`font_size_baseline.tsv` + the 4 frozen visual standards (medical bed/end-of-bed
platinum, door/annunciator gold). All 4 will fail until re-frozen — this is
intentional drift from the caps removal + colour change; **do not re-baseline
until the user approves the visuals**, then follow the onboarding/freeze steps in
`crates/starbreaker-ui/docs/ui-matching-workflow.md` (§Target Onboarding /
Re-baselining). The font-size harness re-baseline command is in
`docs/ui-font-size-harness.md`.

### Fast iterate / measure loop (no full entity export needed)
```bash
cd StarBreaker
cargo build --release -p starbreaker
SB_UI_FONT_DUMP=1 ./target/release/starbreaker ui render \
  --scene "../ships/Packages/DRAK Clipper_LOD0_TEX0/scene.json" --out-dir /tmp/fc \
  2>&1 | grep '^FONTDUMP'                       # exact rendered px per element
# medical bed PNG: /tmp/fc/534bab84-299b-479a-a4af-4469df112ea7_TEX0.png
# medical end-of-bed (Medical2) PNG: /tmp/fc/_slot_standing_screen_TEX0.png
```
Layout rects without a render (reflects local build):
```bash
cargo run -p starbreaker-ui --example query_ui_layout -- \
  --canvas-guid 534bab84-299b-479a-a4af-4469df112ea7 --query <NodeName>
```
No `SC_DATA_P4K` needed (auto-detected).

---

## ITEM 2 — header gap (+3-4px between "Drake Clipper" and "T3 MEDICAL ASSISTANT")

Currently top-to-top **27px**, want **~31** (+4). "Drake Clipper" cap-top ≈39,
"MEDICAL ASSISTANT" cap-top ≈66 in the latest render.

### Dead ends (already ruled out — do NOT re-investigate)
1. `centered_intrinsic_text_column_adjustment` compression heuristic — exits
   early for Heading1/Heading3 (`textfield_auto_intrinsic_override` only matches
   Title3/Heading2 + specific tags).
2. `topMarginModifier` — **0** for blenderpro-thin/bold.
3. `anchor.y` box resolution — correct + linear (`bb_layout` part_01:334 /
   part_04:351: `parent.y + parent.h*anchor.y`; verified LocationName
   anchor.y=-0.05→y30.8, TierLevel anchor.y=-0.12→y25.3 against parent h=78).
4. `vAlign=Center` glyph-bbox centring + SWF `top_overscan` (Drake Clipper sits
   8.2px below its box top = the font's authored edit-text top inset) — all
   data-faithful.
5. The reference screenshot is **unusable** as an arbiter: its whole UI is
   vertically offset ~11-16px (RTT framing) and naive measure gives a *smaller*
   gap (22). Trust the user's +3-4px target, not the reference pixels.

### ROOT CAUSE (confirmed)
`HeaderTitleBase` (the container holding the two header rows) has:
```
layoutPolicy: BuildingBlocks_FlexContainer {
  direction: Column, wrap: NoWrap, axisJustification: Start,
  crossAxisJustification: Start, itemAlignment: Start,
  rowSpacing: 0, columnSpacing: 0 }
```
(The flex lives in **`layoutPolicy`**, not a `flex` field — that's why an early
look "found flex=None".) Children **LocationName** (Drake Clipper, Heading3) and
**TierLevel** (T3 + nested MachineTypeNameText "MEDICAL ASSISTANT", Heading1)
both have **`affectsLayout: true`**. So the engine **stacks them as a Start
column** — Drake Clipper row, then the tier/title row below it — and the gap =
the stacked height of the LocationName row.

`bb_layout` DOES detect the flex (`part_02.part:46` reads
`node.raw["layoutPolicy"]` containing "FlexContainer" → `layout_flex_children`
in `part_03.part`). **The bug:** LocationName/TierLevel are `height: Auto`, so in
`part_04.part` they hit the `auto_main` branch (~lines 339-395) which calls
`layout_node(id, container)` — this **anchor-overlays** them inside the container
(boxes land at the anchor.y formula → 30.8 / 25.3, overlapping) instead of
placing them at the stacked `cursor`. AND their `Auto` height resolves to the
**parent height (78)** via `resolve_value` (`part_06.part:148`: `"Auto" =>
primary_dim`) rather than the text line-box.

Net: the two rows overlap and the 27px gap is an accident of anchor offsets +
text vertical alignment, not the intended column stack.

### INTENDED FIX (data-derived, no fudge)
In a flex **Column**, an auto-height text field must be placed at the stacked
`cursor` with height = its **intrinsic line-box** (≈ resolved font size +
leading), not anchor-overlaid at parent height. Expected: LocationName row height
≈31 (size 30) → TierLevel starts +31 → header gap ≈31. ✓

Two coupled changes:
1. **Intrinsic auto-height for text fields.** `resolve_value` returns parent
   height for `Auto`; instead an `Auto`-height `WidgetTextField` should size to
   its line-box. The blocker: **`bb_layout` runs before `ui_ir` resolves font
   sizes**, so it has no text renderer / resolved size — that's the whole reason
   `textfield_auto_intrinsic_override` (`part_06.part:265`) exists with hard-coded
   180/270/60 per style+tag. Clean approach: make the resolved font size
   available at layout time. The size resolution already exists in
   `ui_ir/part_09 standard_textfield_font_size_from_styles` → `part_10
   brand_style_font_size` (reads `textfieldwidgetstandard.json` defaultStyles +
   the brand-matched brandStyles, keyed by `selected_style_source`). Either plumb
   that size onto the `BbNode` during `bb_scene` build, or compute the brand
   FontSize for `labelProperties.style` inside `textfield_auto_intrinsic_override`
   and return line-box ≈ `FontSize * (em+leading)/em` (≈ FontSize). That also lets
   the magic 180/270/60 be replaced by derived values (a win — those are exactly
   the fudges to remove).
2. **Stack instead of overlay for Column auto text.** In `part_04.part`
   `auto_main` branch, when `!is_row` (Column), place the node at
   `Rect{ x: container.x (+ cross-axis just), y: cursor, w, h: intrinsic_h }` and
   advance `cursor += intrinsic_h + item_spacing`, rather than
   `layout_node(id, container)`. Keep the overlay behaviour only where it's truly
   needed (non-flex parents / `affectsLayout:false`). Mind `crossAxisJustification`
   / `itemAlignment` = Start (left) and `anchor.x`.

### Verify (this is a BROAD layout change)
- Re-render medical bed; header gap → ~31 (measure Drake-Clipper-cap-top vs
  heading-cap-top).
- Run the FULL UI regression path (`ui-matching-workflow.md` §Validation Loop):
  `manifest_snapshot_regression`, `manifest_live_ir_guard`,
  `manifest_visual_regression`, `validate_ui_snapshot_freeze.sh`,
  `validate_ui_regression_artifacts.sh --quick`, plus
  `cargo test -p starbreaker-ui`.
- Re-render + eyeball **medical end-of-bed, door (CLOSED), annunciator** — any
  flex Column with auto text could shift. The harness `SB_UI_FONT_DUMP` +
  `scripts/font_size_check.py` guards text sizes.

---

## MEDICAL2 — `I_Med_MedicalEndOfBed_A` (next screen, platinum / ui_target_b)

aka `_slot_standing_screen_TEX0.png` aka
`buildingblocks_canvas_i_med_medicalendofbed_a.png`. NOTE: the caps removal +
header-colour commits already change this screen too (it shares the header +
MedGel components) — re-measure current state before applying offsets.

User-reported issues:
1. **The entire MEDGELS container** (MEDGELS text + 200/200 value + progress bar)
   must move **UP 19px and RIGHT 2px**. This is the RightHeader MedGel block (same
   `ComponentLabelCaptionPair` + `WidgetLinearProgressMeter` as the medical bed,
   in `ic_med_medicalcommon_a_header.json`). Find what positions the whole MedGel
   container (its anchor/position in RightHeader) and move it as a unit — prefer a
   data-derived placement fix over an offset (check this screen's RightHeader
   anchors vs the medical bed's; the 19px/2px may be a layout difference this
   screen authors that we're not honouring).
2. **"DIGITAL MEDICAL ASSISTANT"** text → **97.5%** of current size. This is the
   MachineTypeNameText/title on this screen ("DIGITAL MEDICAL ASSISTANT" vs the
   bed's "MEDICAL ASSISTANT"). After the caps removal it renders full nominal
   (H1=57). Measure current px via `SB_UI_FONT_DUMP`, target = ×0.975. Investigate
   whether this screen authors a different brand/scale (e.g. labelProperties.scale
   or a different style) rather than applying a raw 0.975.
3. **"DIGITAL MEDICAL ASSISTANT"** text → **light blue** (Base). Same mechanism as
   the medical-bed item 1: a header text field with **no own style tag** inherits
   its brand H-level FillColor (s_bioc H1 = Base) via the `ui_ir/part_04`
   colour_token chain change already committed. Check why this screen's title
   isn't already blue — likely it HAS an own style tag (overriding to white) that
   the bed's TierLevel lacks, or it's a different node. Compare its `styleTags` to
   the bed's LocationName/TierLevel (which have none).

### Medical2 measure loop
PNG `/tmp/fc/_slot_standing_screen_TEX0.png`; canvas record
`I_Med_MedicalEndOfBed_A` (find GUID via `search_records`/`datacore_record`);
`query_ui_layout --canvas-guid <guid> --query <Name>` for positions; reference
image is the user-provided in-game Medical2 screenshot.

---

## Working rules (from the repo + this session)
- No hard-coding / name-matching / per-screen branches / magic offsets in
  production. Fix the structural cause (data-derived). Registered fallbacks only
  when proven not in data (`docs/ui-fallback-register.md`).
- The reference in-game RTT captures are ~2% small and vertically offset — trust
  the user's measured targets, not naive reference pixels.
- Verify visually with the user (VFL) before re-freezing; don't re-baseline to
  silence intended drift.
