# Clipper UI parity — handoff (2026-06-11)

> NOTE 2026-06-11: the `engine_parts/part_NN.part` files were consolidated
> into `engine_NN.part` chunks (cap 3000, target ≤2500 — `docs/ui-workflow.md`
> rule 5). Old part names are listed in each chunk's header comment, so
> `grep -rn "part_NN.part" crates/starbreaker-ui/src` locates the absorbing
> chunk for any stale reference (memory notes included).

State, remaining issues, and the ongoing plan for the Drake Clipper screen
parity arc (plan `~/.claude/plans/wondrous-sparking-sketch.md`, branch
`feature/ui`). Companion documents:

- `docs/ui-process-improvements.md` — process changes adopted mid-arc.
- Project memory `power-screen-parity-plan.md` (Claude session memory) — the
  full mechanism research log; this handoff supersedes its "remaining" lists.
- `docs/ui-workflow.md` + `docs/ui-reference.md` — the rules and the
  command/tool reference (TDD, no per-asset hacks, audited freezes only).
  Fresh sessions: instantiate
  `crates/starbreaker-ui/docs/ui-matching-agent-prompt.md` with
  `SCREEN=Screen_Left_Lower_RTT`, `HANDOFF=` this file.

## Where things stand

All work is committed on `feature/ui`; tree is clean and green via
`bash scripts/ui_check.sh --full` (484 ui lib tests passed, 1 ignored;
all 5 frozen targets in the live IR guard; snapshot + visual suites; both
freeze validators; 3d lib; font harness 26/26). Renders in this doc come from
`./target/debug/starbreaker ui render --scene
"/home/tom/projects/scorg_tools/ships/Packages/DRAK Clipper_LOD0_TEX0/scene.json"
--out-dir /tmp/... --helper Screen_Left_Lower_RTT` (debug build is fine;
release re-exports only matter for artifact freezes). Compare with
`python3 scripts/ui_compare.py <render> <reference> --regions power`.

Reference images: `/home/tom/projects/scorg_tools/reference/in-game/Clipper/`
(`Screen_Left_Lower_RTT.png` power, `Screen_Right_Upper_RTT.png` target,
`screen_16x9_a-[medical1].png`, `mesh_end_screen_plane-[medical2].png`).
NOTE: red/white pip outlines in the power reference are mouse-hover
artifacts — ignore (Tom's rule).

### Landed this arc (commit order, newest last)

| Commit | What |
|---|---|
| 49367056c | Step 0: `clipper_target_master` frozen GOLD (5 manifest targets now) |
| 55a70f9d6 | Step 1: `UiShipData` derivation (pools→pips, icons, temps) via `PipelineInputs::derived_values`; CLI replay derives from scene root entity |
| e224e992b | Step 2: `disabled` colour token → slot 8 (near-black unpowered pips) |
| 42c065370 | Step 3 (partial): pip icons from `ItemResourceNetworkGlobal.uiParams.typeData` (shield-in-dashed-circle = right asset); own-icon-tag contain fit |
| 53b00b171 | Step 5: `IntegerFromNumber` eval, `LocalizationCombine.withSpace`, param slot broadcast + unconditional relay tagging → OUTPUT "2 / 16" |
| 82aecd85a | Step 6 (row): `_ResolvedText_`/`_EffectiveFontPx_` annotations + intrinsic text measurement for all-Auto rows |
| fded64d87 | Step 6 (column): scoped flex shrink (`apply_flex_no_grow_shrink` — only flex-managed children shrink); battery counts derived from fitted Battery items → "0 / 0" |
| ffd587211 | Steps 4+12-fonts: node `inlineStyles` cascade (FINAL stage, `__InlineFontSize` outranks brand standard) → OUTPUT/BATTERY FontSize 30; mixed-row Auto text intrinsics (BATTERY right of icon); `font_size_check.py` parser fix |
| 1161e705a | Step 7: bound `AnchorX/AnchorY` applied in `resolve_geometry_fields_into_scene` (marker = `1 − current/max`); beyond-edge-anchored Auto-hint textfields size to text (°C below gauge); bb_layout part_15 split |
| 1b486f82e | Step 8 (values): clone `urlPostfix` namespaces cloned bindings; ABSOLUTE WidgetCanvas `urlPostfix` → child namespace; `LocalizedFromNumber` + `LocalizedSIUnitFromNumber` eval; signature derivation paths |
| e4a4bdef6, 9ba1c94b9, a3b16bc92 | `docs/ui-process-improvements.md` (retrospective → consolidation spec → phased plan) |
| b0d57a684 | Step 9R fixes: SizeX/SizeY modifiers preserve authored sizing behaviour (+ audited gold re-freeze, see below); boolean params take registry value by NAME (`iscast=false`); flex enum modifiers (FlexDirection etc.); clone roots inherit `layoutItemCommon` (IR, EM, CS order) |
| 837e6caff | this handoff doc |
| 2c6029f49 … 89d6a4d51 | process-plan implementation: `ui_check.sh`, `ui_compare.py`, harness self-check, `ui_stage_diff`; `docs/ui-workflow.md` + `docs/ui-reference.md` (old ui-matching docs DELETED); self-auditing freezes (delta embedded, no-op refused); registry `.notes.md`; docs reference guard; font TSV re-captured; artifact freeze refreshed |
| 3b3957562 … c256e3efc | engine-part consolidation: 83 `part_NN.part` → 12 `engine_NN.part` chunks (cap 3000); zero code changes |
| 6b958b4e5 | CI: P4K-backed MCP tests SKIP without game data; validator tests cfg(unix) |

### Gold re-freeze performed (audited)

`clipper_target_master` was re-frozen (approver tom): node
`40:widget_custom_shape` h 0.18 → 194.4. It is the TARGET STATUS faint
backdrop band (authored SizeY 0.18 **Percent**, alpha 0.1) that the old
Percent→Fixed modifier conversion collapsed to a sliver — movement toward
the reference; partially resolves open item A7. The freeze delta was audited
to exactly that one element. The artifact PNG freeze was completed
(89d6a4d51): the release re-export produced byte-identical target PNGs
(all 5 sha256 hashes unchanged — the alpha-0.1 band does not alter the
canvas-direct render's pixels; its visible-pixel side rides catalog #5/#6,
the A7 backdrop class). Freezes are now SELF-AUDITING: the tool prints and
embeds the per-identity delta and refuses no-op re-freezes.

## Step 9R diff catalog (power screen vs reference) — status

| # | Region | Difference | Status |
|---|---|---|---|
| 1 | Emissions | header collapsed to 2px | **FIXED** (SizeY behaviour + iscast) — values render "3.5K / 0.0 / 0.0" in IR-EM-CS order |
| 1b | Emissions | emitted/ambient OVERLAP inside each group (one line, ambient under emitted) | **FIXED 2026-06-11** — two rules: column zero-Auto text intrinsics + zero-Auto text children join the flex shrink set (below) |
| 2 | Emissions | IR/EM/CS labels render `@LOC_PLACEHOLDER` | **FIXED 2026-06-11** — clone expansion applies FieldModifierLocalization via `_SynthLocalizedWidget_` (below) |
| 3 | OUTPUT card | title at right of header row; ref has icon→dots→title left-aligned | **DIAGNOSED 2026-06-11, spec test ready** — intrinsic measure ≠ draw width (below) |
| 4 | Battery card | OFFLINE text container 543px overflows card, indented right | Same root cause as #3 (drawn width 352.5 vs measured 543); after the #3 fix the 352.5px box still slightly overflows the 339px card → the new zero-Auto shrink (1b rule 2) finishes the job |
| 5 | All cards/pips/gauges | icons dark, separator dots invisible, gauge zone colours, pip brightness, backdrop bands, "2" white vs cream | **DEFERRED by design** — the parked defaultStyles/icon-tint cascade re-land, scheduled with the medical re-freeze (Steps 10–13); full diagnosis in memory file §"DIAGNOSED, NOT LANDED" |
| 6 | Footer/scrollbar | good parity; faint track + backdrop band remain (A7 class) | Defer with #5 |

## Remaining work, in order

### 1b. Emissions emitted/ambient stacking — LANDED 2026-06-11

Two structural rules landed (TDD, both spec tests in
`bb_layout/engine_parts/engine_02.part` `flex_shrink_tests`):

1. `column_zero_auto_text_children_stack_at_measured_heights` — in a COLUMN
   flex, Auto **value 0.0** (pure content hint) text-backed children take
   their measured text heights (`layout_flex_no_grow_children` auto_main
   chain, `bb_layout/engine_parts/engine_01.part`). Scoped to 0.0 only: the
   medical platinum pins the fill placement for non-zero Auto hints.
2. `column_zero_auto_text_children_shrink_to_fit_container` — zero-Auto
   text-backed children are CONTENT-SIZED flex items and join the shrink
   set (`zero_auto_text_backed`, used by `apply_flex_no_grow_shrink`).
   Without this the emitted/ambient intrinsics (150px each at the
   nominal-100 measure) overflow the 141.5px band; with it they shrink to
   ~70.75 each and the fit-to-rect font model lands ~28px — matching the
   reference's adjacent two-line stack. Zero-Auto children WITHOUT text
   keep the zero-size rule and still veto flow-wide shrink (medical
   exemption semantics preserved; full battery + font harness green).

Verified on the replay render: emissions values now read 3.5K/294.1,
14.9K/0.0, 18.6K/0.0 in tight emitted-over-ambient stacks like the
reference. Note the pre-layout `_EffectiveFontPx_` for these texts is
still the authored 100 (no brand-standard FontSize applied pre-layout) —
the rendered size is correct only because the fit-to-rect model scales
into the shrunk box; the genuine effective-font question rides item 5's
cascade re-land (same family as catalog #4 OFFLINE font-100).

### 2. Emissions IR/EM/CS labels — LANDED 2026-06-11

NOTE: the original plan's premise was wrong — there is NO
`BindingsLocalizedField` op for text_Abbreviation (ptr:5) in
`gen_mc_s_emissions.json`; the clone's FieldModifierLocalization is the
ONLY text source for that label (authored `labelProperties.label =
@LOC_PLACEHOLDER`, suppressed as intentionally-empty).

Landed (test `widget_clone_localization_modifiers_apply_to_cloned_targets`,
`bb_scene/tests.rs`): `apply_clone_modifiers` in `bb_scene/clone_expand.rs`
maps each FieldModifierPair target through the clone `id_map` and
synthesizes a `_SynthLocalizedWidget_` op (the existing
`inject_param_overrides` vehicle feeding `widget_to_loc_key`), so the label
resolves `@hud_Label_IR/EM/CS` → "IR"/"EM"/"CS" (caseModifier Upper). The
library original keeps its placeholder. The clones' second modifier
(FieldModifierString SvgPath → per-type icon on shape_Icon) is logged but
NOT applied — shape_Icon is inactive at rest so there is no observable
effect to test; pick it up if/when an icon-active screen needs it.

### 3. OUTPUT title position / 4. OFFLINE width — DIAGNOSED, spec test ready

Root cause (measured 2026-06-11): the header row FLOWS correctly (icon
140.8 → dots 230.5 → title box 274.4) but the title box is 232.7px wide
while the drawn glyphs are 160.4px (`SB_UI_FONT_DUMP`: size_px 50 =
effective 30 × host-stage 1.667, drawn width 160.43). The authored
`textAlignment: Right` puts the 72px slack on the LEFT. In the engine the
Auto box hugs its glyphs (measure==draw), making alignment invisible — the
defect is OUR measure, not the alignment or the flow.

Why the measure overshoots ~1.45× on the MFD path:
`node_resolved_text_size` measures TTF Mono (advance ≈0.862em) at
effective × `LAYOUT_TEXT_MEASURE_CALIBRATION` (1.5), mirroring the
INTERIOR draw path (`TEXT_RENDER_SIZE_CALIBRATION` 1.5 + TTF). The MFD
path draws SWF Audimat Mono (advance ≈0.535em of its 21560-unit em) at
size_px = effective × design_text_scale (1.667; no 1.5). 0.862×1.5 /
(0.535×1.667×21560-em-normalised) ≈ 1.45. The advance mismatch dominates —
no scale constant can reconcile it; the measure must use draw metrics.

Fix spec (the `#[ignore]`d test
`auto_text_child_prefers_draw_metrics_annotations`,
`bb_layout/engine_parts/engine_02.part`): ui_ir's pre-layout annotation
pass — it already writes `_ResolvedText_`/`_EffectiveFontPx_` and knows
the resolved font record + `design_text_scale` (passed from
`pipeline/mod.rs`, which holds the `swf_library`) — additionally writes
`_DrawTextWidthPx_`/`_DrawTextHeightPx_` measured through the draw-side
SWF glyph machinery (`swf_glyph_advance_px` in `text/swf_draw.rs`; extract
a measure helper). `node_resolved_text_size` prefers the annotations over
its TTF estimate. Plumbing: a text-measure callback from pipeline into
`compile_ui_ir_*` (the SWF font lookup per resolved font record lives
there). Expect gold `clipper_target_master` geometry drift — adjudicate
each identity vs the reference (movement toward = re-freeze §7).

For #4: OFFLINE drawn width is 352.51 (vs 543 measured); with draw-metric
boxes it still slightly overflows the 339px card, then the zero-Auto
shrink (1b rule 2) fits it — verify both cards after landing.

### 5. Parked tint/defaultStyles cascade re-land (with medical Steps 10–13)

Full diagnosis in memory `power-screen-parity-plan.md` §"DIAGNOSED, NOT
LANDED (re-land with medical re-freeze)". Summary: power icon yellow =
defaultStyles entry "System Icon Color" (FillColor Accent2@1.0 on the `icon`
tag e5cd9d57); pipeline gaps are (a) `apply_canvas_style_cascade` never
applies `defaultStyles.entries`' plain modifiers, (b)
`FieldModifierColor` with a palette lacking `colorStyles` drops its token,
(c) `ir_compose`'s custom-shape fill_override path leaves asset-bearing
custom shapes untinted even when tokens flow. Expected medical platinum
token drift when re-landed: close-button X Base→Bright (this IS the wanted
white X of Step 10), fingerprint images Accent1→Base, one text
Foreground→Bright — adjudicate vs the medical references at the re-freeze.
DON'T retry the "enableColorOverlay+null → Base" overlay default (regressed
target-screen chevrons; entry-driven FillColor is the engine model).

### 6. Medical bed (plan Steps 10–13)

- White X: likely already delivered by item 5's Bright token shift — verify.
- 64→69px close-button: find the engine rule or register w/h known-outliers
  (`crates/starbreaker-ui/tests/fixtures/ui_ir/ui_known_outliers.json`,
  reference-anchored one-sided overrides; none registered yet).
- Position outliers: T3 + MEDICAL ASSISTANT −5px (lower), PATIENT NAME +
  "No patient in bed" +5px (higher), Bioticorp logo −12px (check
  `vertical_alpha_balance_offset` in `ir_compose/engine_parts/engine_01.part` (`vertical_alpha_balance_offset`)
  first — if fixed, also re-freeze `ui_target_b` end-of-bed).
- Re-freeze medical platinum: `scripts/freeze_ui_snapshot_ir.sh --approver
  tom --reason ...` + `scripts/validate_ui_snapshot_freeze.sh` + artifact
  freeze.

### 7. Power wrap (Step 9 finish)

After 1b/2 land: `bash scripts/ui_check.sh --full`; `cargo build --release
-p starbreaker`; re-export
(`./target/release/starbreaker entity export drak_clipper
/home/tom/projects/scorg_tools/ships --kind decomposed` — no SC_DATA_P4K
needed, auto-detected); final `ui_compare.py` pass vs the power reference;
then re-run the artifact freeze if any frozen target's PNG changed (the
2026-06-11 freeze 89d6a4d51 covers the current export).

### 8. Approval-gated items (ask Tom)

- ~~Font baseline TSV re-capture~~ DONE (5a5b51f71): re-captured from the
  LOD1 scene, checker 26/26; the 7 baselined drifts are quoted in the
  commit.
- **OUTPUT 2/16 + emissions derivation formulas**: both still
  reference-pinned (registry / `ship_values.rs` with TODOs). Emissions live
  formula = engine SignatureSystem; the canvas's `staticVariables`
  `Signatures_NA` array confirms the Signatures array shape.

## Key mechanisms quick reference

- **Derivation**: `crates/starbreaker-3d/src/ui_pipeline/ship_values.rs`
  (+ tests). Probe `SB_SHIP_VALUES_DUMP=1`. Replay derives from scene
  root_entity so `ui render --scene` == export.
- **Namespaces**: list slots (fully-qualified at materialisation), ABSOLUTE
  WidgetCanvas `urlPostfix` (leading `/`) → child namespace
  (`bb_resolve/engine_parts/engine_01.part`, Pass-2 child-namespace block), clone `urlPostfix` → cloned
  binding prefix (`bb_scene/clone_expand.rs`). RELATIVE canvas postfixes are
  deliberately NOT composed — medical authors pre-qualified bindings and the
  platinum registry keys pin that; composing them broke ui_target_a/b
  (don't retry without a registry key migration).
- **Style cascade order**: style-link < sharedStyles < brand < embedded <
  node `inlineStyles` (always applied last per pass + an empty-entry pass in
  `apply_canvas_style_cascade` guarantees them). Inline FontSize is marked
  `__InlineFontSize` and outranks the brand table in
  `resolve_effective_font_size` (`ui_ir/engine_parts/engine_02.part`, `resolve_effective_font_size`).
- **Flex**: order = `layoutItemCommon.order`; shrink only over flex-managed
  children (Fixed/Percent/Auto∈(0,1]); Auto hints >1 and method None are
  fill-fallback and never shrink; row intrinsic text measurement via
  `auto_text_intrinsic_main` (any row), columns pending 1b; scrollable
  (`scrollPolicy`) and wrap rows exempt from shrink. SpaceBetween axis
  justification is NOT implemented (falls back to Start) — minor open.
- **Probes**: full registry in `docs/ui-reference.md` §6
  (`BB_SHRINK_PROBE`, `SB_UI_GEOM_PROBE`, `BB_A3_STYLE_PROBE`,
  `BB_A3_TEXT_PROBE`, `SB_SHIP_VALUES_DUMP`, `SB_UI_FONT_DUMP`,
  `ui render --dump-ir-dir <dir>`).
- **Stage bisection**: `cargo run -p starbreaker-ui --example ui_stage_diff
  -- <canvas.json> [WxH] [--filter <substr>]` — parse-only vs full-resolve
  layout diff with a first-divergence report.
- **Freezes**: `scripts/freeze_ui_snapshot_ir.sh` + validate +
  `scripts/freeze_ui_regression_artifacts.sh`. The IR freeze tool now
  prints/embeds the per-identity delta and refuses no-op re-freezes; the
  reason and the commit message must account for every delta line.

## Task list mapping

Tasks #27/#30/#36 (emissions, tint re-land, review) remain in_progress;
#33–#35 (medical) pending; #15 is the umbrella. #17 (A7) is partially
resolved by the gold re-freeze; the rest of the A7 backdrop stack rides
item 5.
