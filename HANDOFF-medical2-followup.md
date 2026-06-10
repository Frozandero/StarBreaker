# Hand-off: Clipper medical UI — follow-up (Medical1 item 2, Medical2 items 1–3)

Picks up from `HANDOFF-item2-medical2.md`. Working solely from **in-game references**
(the source of truth; if our model can't reproduce them, the model is what changes).
Companion memory: `medical2-item2-investigation.md` (+ `clipper-medical-bed-parity.md`,
`caps-reduction-removed.md`).

References (both 1920×1080, down-sized from ~4K + de-skewed → accurate **~1–2px**; treat
<2px as noise):
- Bed (Medical1): `reference/in-game/Clipper/screen_16x9_a-[medical1].png`
- Medical2:       `reference/in-game/Clipper/mesh_end_screen_plane-[medical2].png`

---

## RESOLVED this session

1. **Medical2 item 3 — title "DIGITAL MEDICAL ASSISTANT" colour → Base blue.** COMMITTED
   (`0a3e02874`). `semantic_text_colour_token_from_style_tags`
   (`ui_ir/engine_parts/part_08.part`) mapped a Title-style node carrying the colourless
   `Primary` state tag (`e6003a83`) → `Bright` (grey), overriding the brand Title3
   FillColor; now → `Base`. Rendered title went grey (188,192,208) → blue (110,189,243)
   vs ref (95,167,208). User-confirmed. Test renamed to `..._to_base_token`.

2. **Bed clipping regression** (self-inflicted): the item-2 stack experiment (below) was
   left compiled into the release binary; rebuilding + re-rendering restored baseline.
   The item-2 experiment is fully reverted. **Lesson: ALWAYS rebuild the release binary
   after a source change before re-rendering.**

3. **Medical2 item 2 — title SIZE is correct.** Font, size, weight, colour all right
   (user-confirmed font/weight). Single-glyph cap height is within 1px of the reference.
   It is **NOT** a brand issue (see "Brand bug" below). The earlier "×0.975" read was
   superseded — see the *width/kerning* residual under Unresolved.

---

## UNRESOLVED

### 1. Medical2 item 1 — MEDGELS block ~8px too low, ~2–3px too left  (PARTIAL FIX, uncommitted)

**Symptom (measured vs ref).** The whole MEDGELS block (label "MEDGELS" + value "200/200"
+ progress bar) renders too low. Baseline ~15–19px low; with the uncommitted change below,
**~8–11px low** (and the user independently confirmed the +19px overshoot of a different
attempt). Horizontal ~2–3px (the authored "right 2"). Band measurement after the partial
fix (uniform → it's a *container-position* problem, not internal spacing):
```
            label        bar
 OURS:    46–63        96–110
 REF :    35–52        85–100      → whole block ~11px too low, uniformly
```

**Root cause (confirmed, but its source is untraceable statically).** med2's `MedGel`
(`BuildingBlocks_ComponentLabelCaptionPair`) resolves `anchor=[1.0,0.0] pivot=[1.0,0.0]`
(right-anchored), whereas the **component standalone** (`cf8a3753`) and the **bed** both
give `[0.0,0.0]` (the authored value). Right-anchored flips `stacked_label_caption_pair_text_rects`
into an unclamped, anchor-derived `top_padding` (~39px) **plus** a `pair_offset_y` (~24px)
that float the block down. Additionally the container chain itself is mis-positioned: med2
`TextLayout` resolves to **y=−5.5** but the anchor math (`RightHeader.y −15.2 + 0.4×152.3`)
predicts **+45.7** — a ~51px discrepancy. Both the anchor flip and the ~51px shift are
introduced by the **med2 multi-canvas/slot composition** and are **untraceable by static
analysis** (see "Tried & failed (a)").

**Uncommitted partial fix** (working tree, `ir_compose/engine_parts/part_06.part`, in
`right_anchored_label_caption_pair_offset`): zero the vertical offset, keep the horizontal:
```rust
-    (-(line_box_delta + stroke_pair_span), line_box_delta)
+    (-(line_box_delta + stroke_pair_span), 0.0)
```
Effect: MEDGELS from ~19px → ~8px low; **no new test failures** (406 pass, same 2
pre-existing fail); only med2's MEDGELS is affected (the only right-anchored caption-pair
found). Decide: commit as a documented partial fix, continue, or revert.

**Approach (a) — find why med2 MedGel `anchor.x=1.0` — HIT A WALL.** Instrumented every
known anchor-mutation path; **none fire** for the MedGel:
- `AnchorX`/`AnchorY` brand modifier (`bb_brand_apply/modifiers.rs:229`) — never applied.
- clone-expansion copy (`bb_scene/clone_expand.rs:83`) — never fires for it.
- canvas merge scaling (`bb_resolve` `scale_node_from_child_canvas`) — doesn't touch
  anchor; also skipped here because the medical `Header` slot is `Percent`-sized (not
  `Fixed`) and the scale would be <0.25 (`child_canvas_scale_for_host`).
- component standalone (`cf8a3753`) resolves the MedGel to `[0.0,0.0]`.
- broad grep: those are the *only* anchor mutations in the crate.
So `anchor.x=1.0` appears in med2's resolved scene with no instrumented mutation firing →
it is produced by the med2 composition path I can't reach via the canvas-graph fetcher in
`query_ui_layout`. **Next session: instrument the live med2 merge end-to-end** (the actual
`render_ui_binding_png` / `resolve_canvas_graph_with_loc_and_bound_view` path for the
`_slot_standing_screen` binding, not the example), logging the MedGel anchor + position at
each stage, to find where the anchor flips to 1.0 and the ~51px shift is introduced.

**Approach (b) — make the right-anchored pair compact — OVERSHOT, then partial.**
- Clamping `top_padding` to `max_top_padding` (like the non-right-anchored bed): overshot
  to **~19px too high** (removed the full ~38px swing; user-confirmed). Reverted.
- Removing only `pair_offset_y` (the uncommitted change): **~8px too low** — best so far.
- The exact landing is the **midpoint**, which is calibration-sensitive (no clean
  structural value), *because the real cause is the container position, not the text
  offsets*. So (b) can get close but cannot reach exact — (a)/the composition root must be
  fixed for an exact, no-constant result.

### 2. Medical1 (bed) item 2 — "T3 MEDICAL ASSISTANT" header gap 5px too small

Reference-confirmed REAL: "Drake Clipper" cap-top 39 = ref 39 ✓; "MEDICAL ASSISTANT"
cap-top **66 vs ref 71** → gap **27 vs ref 32** (~5px too high on that line only).

**Tried & failed — the handoff's flex-Column "stack" fix OVERSHOOTS.** Implemented it
(ui_ir plumbs an em line-box onto the layout scene; bb_layout stacks `Auto`-height
flex-Column text fields at the line-box): rendered gap → **50px** (way past 32). Because
stacking drops `TierLevel` a full `LocationName` row and `MEDICAL ASSISTANT`
(`vAlign=Center`) sinks within its row. **Fully reverted to baseline.** The correct fix is
a *small ~5px DOWN nudge* of just the `TierLevel`/`MachineTypeNameText` line — structural
cause TBD (the prior handoff already ruled out topMargin, anchor.y, vAlign-centring as
data-faithful; do NOT redo the full stack). Parked.

### 3. Medical2 item 2 (residual) — title ~2.7% too wide (kerning)

Font size/weight/colour are correct. **Width**: ours 951 vs ref **926** (~25px, user
confirmed it's real and **>noise**). Single-glyph cap height within 1px (so it's not font
size). 2-line bbox height ~4px (line-spacing / vertical-alignment, near de-skew noise).

**Diagnosis:** the width is **SWF glyph-advance / kerning** — the global SWF text path
(`SWF_TEXT_WIDTH_CALIBRATION = 1.0` in `text/swf_draw.rs`, and the per-glyph advances).
NOT a brand issue. Untouched because it's a **global** path (affects every SWF-rendered
string — bed text, MFDs, etc.) and needs broad re-verification. Not yet attempted.

---

## Brand bug (DISCOVERED — do NOT "fix", per user)

`cli/src/ui.rs:112` derives `manufacturer_id` from the **ship** ("DRAK Clipper" → `drak`)
and forces it on the medical sub-canvases (`pipeline/mod.rs:185`: binding manufacturer
overrides the canvas's own brand). The medical UI's authored brand is **s_bioc**, but
**`drak` is what matches the references** — under s_bioc the title becomes
blenderpro-**medium** (bolder) + `line_spacing −55`, contradicting the thin reference.
**USER DECISION: keep drak; do not switch.** Recorded only in case it explains a future
discrepancy. (Both `s_bioc` and `s_drak` author Title3 = FontSize 150; no 146/147 variant
exists, which is why the "×0.975" theory was wrong.)

---

## Methods / tooling that worked

- **FAST LOOP (seconds, no 7-min render):**
  `cargo run -q -p starbreaker-ui --example query_ui_layout -- --canvas-guid <guid> --query <Node>`
  — prints layout rects, `primary/secondary_text_rect`, `*_text_drawn_bounds`, and
  `meter_draw_rect`. It **does** reproduce med2's header MedGel (id 2147483655) exactly.
  Iterate layout/positioning here; only render to confirm pixels. **Caveat:** it can't
  resolve the brand `TextFieldWidgetStandard` sizes (no fetcher) → it falls back to the
  raw authored font size, so measure *font sizes* via the real render, not this example.
- **RENDER LOOP:** rebuild release **then**
  `SB_UI_FONT_DUMP=1 ./target/release/starbreaker ui render --scene "../ships/Packages/DRAK Clipper_LOD0_TEX0/scene.json" --out-dir /tmp/fc 2>&1 | grep '^FONTDUMP'`
  — bed PNG `/tmp/fc/534bab84-299b-479a-a4af-4469df112ea7_TEX0.png`, Medical2 PNG
  `/tmp/fc/_slot_standing_screen_TEX0.png`. FONTDUMP gives per-node rendered px.
- **Pixel measurement:** `python3` with PIL+numpy (note: `uv` is NOT on PATH in this
  workspace; use `python3` directly for one-off measurement scripts).
- **Authored data = DataCore via MCP** (`datacore_record`, `search_records`, `p4k_read`);
  the MCP `ui_*` tools are derivative (StarBreaker's own output) — use only to measure
  drift. The deployed MCP binary does NOT reflect local source changes — don't use it to
  validate edits.

## Key authored records (DataCore)

- Header component `IC_Med_MedicalCommon_A_Header` = `cf8a3753-4d5a-4c33-84b5-3e14ff646023`
  — MedGel/RightHeader/TextLayout authored (`MedGel anchor (0,0)`).
- Brand `TextFieldWidgetStandard` = `ed45b6af-31ab-4726-893b-bea64fb91a49` — styles named
  T1–T6 (Title3 = "T3" = FontSize 150), FontSize/LineSpacing modifiers, `s_bioc`/`s_drak`
  brand sections.
- `LabelCaptionPairComponentStandard` = `08f7bde7-4860-4cbc-adbc-3a24cb24e450`.
- Bed canvas `534bab84-…`; Medical2 canvas `e9ad809d-…` (renders as `_slot_standing_screen`).

## Key code locations

- Title colour (FIXED): `ui_ir/engine_parts/part_08.part:231` `semantic_text_colour_token_from_style_tags`.
- Caption-pair text stacking: `ir_compose/engine_parts/part_06.part`
  (`stacked_label_caption_pair_text_rects`, `right_anchored_label_caption_pair_offset`).
- Progress-meter positioning: `ir_compose/engine_parts/part_04.part`
  (`resolved_linear_progress_meter_rect`, `debug_linear_progress_meter_rect`).
- Layout anchor/pivot resolution: `bb_layout/engine_parts/part_01.part` (`layout_node`,
  `effective_pivot_y`).
- Canvas merge/embed: `bb_resolve/engine_parts/part_05.part` (`merge_child_scene`,
  `child_canvas_scale_for_host`, `scale_node_from_child_canvas`).
- Brand selection (the brand bug): `cli/src/ui.rs:112`, `pipeline/mod.rs:185`.
- SWF text width/kerning: `text/swf_draw.rs` (`SWF_TEXT_WIDTH_CALIBRATION`, glyph advances).

## Pre-existing failing tests (NOT introduced this session — fail at HEAD `87d22f329`)

- `ir_compose::tests_d::label_caption_pair_stacks_secondary_immediately_below_primary_text_band`
  (asserts 69, gets 66).
- `ir_compose::tests_e::bottom_anchored_progress_meter_uses_label_caption_text_band_bottom`.
  Both are calibrated to the old caption-pair behaviour and need updating once the MEDGELS
  positioning is settled.

## Recommended next steps

1. **MEDGELS (item 1):** decide on the uncommitted partial fix (commit/continue/revert),
   then chase the real root via **(a)**: instrument the *live* med2 binding resolution
   (not the `query_ui_layout` example) to find where `MedGel.anchor.x` becomes 1.0 and the
   `TextLayout` ~51px shift is introduced — almost certainly in the multi-canvas/slot
   composition for the standing-screen binding. Fixing that should make MEDGELS exact with
   no constants. The fast loop + references make verification quick.
2. **Bed item 2:** a small ~5px down nudge of the `TierLevel`/`MachineTypeNameText` line —
   do NOT redo the full flex-Column stack (overshoots to 50px).
3. **Medical2 title kerning:** only if the ~2.7% width matters — investigate the SWF
   glyph-advance path globally (high blast radius; re-verify bed/MFD text widths).

## Tree state at hand-off

- Committed: `0a3e02874` (item 3 title colour). `87d22f329` is the prior HEAD.
- **Uncommitted** (working tree): `ir_compose/engine_parts/part_06.part` — the
  `pair_offset_y → 0.0` partial MEDGELS fix (diff above). Everything else reverted to
  baseline; `cargo build -p starbreaker-ui` is green.
- `HANDOFF-item2-medical2.md` (the prior handoff) and this file are untracked.
