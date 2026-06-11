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
`bash scripts/ui_check.sh --full` (482 ui lib tests passed, 2 ignored —
one is the next-step spec test, see below; all 5 frozen targets in the
live IR guard; snapshot + visual suites; both freeze validators; 3d lib;
font harness 26/26). Renders in this doc come from
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
| 1b | Emissions | emitted/ambient OVERLAP inside each group (one line, ambient under emitted) | **NEXT — spec test ready** (below) |
| 2 | Emissions | IR/EM/CS labels render `@LOC_PLACEHOLDER` | **OPEN — mechanism decoded** (below) |
| 3 | OUTPUT card | title at right of header row; ref has icon→dots→title left-aligned | OPEN — investigate flow/justification after 1b (same machinery family) |
| 4 | Battery card | OFFLINE text container 543px overflows card, indented right | OPEN — intrinsic measure uses effective font 100 for that text; revisit with 1b |
| 5 | All cards/pips/gauges | icons dark, separator dots invisible, gauge zone colours, pip brightness, backdrop bands, "2" white vs cream | **DEFERRED by design** — the parked defaultStyles/icon-tint cascade re-land, scheduled with the medical re-freeze (Steps 10–13); full diagnosis in memory file §"DIAGNOSED, NOT LANDED" |
| 6 | Footer/scrollbar | good parity; faint track + backdrop band remain (A7 class) | Defer with #5 |

## Remaining work, in order

### 1b. Emissions emitted/ambient stacking (spec test ready)

The ignored test
`column_zero_auto_text_children_stack_at_measured_heights`
(`bb_layout/engine_parts/engine_02.part`) is the spec: in a COLUMN flex, Auto
**value 0.0** (pure content hint) text-backed children must stack at
measured text heights. Implementation: in
`bb_layout/engine_parts/engine_01.part` (search `layout_flex_no_grow_children`), the auto_main chain currently has
`} else if is_row && let Some(intrinsic) = auto_text_intrinsic_main(...)`;
add a column branch gated on `Auto && value == 0.0` calling
`auto_text_intrinsic_main(child_id, scene, csy, false)`, set `h = intrinsic;
auto_main = false`. **Why scoped to 0.0:** the medical platinum pins the
column fill placement for non-zero Auto hints (a column-wide intrinsic
previously drifted the medical header h 78→18, y +27). Remove `#[ignore]`,
run the battery, adjudicate any guard trip via the structural-discriminator
method (`docs/ui-process-improvements.md` §8).

The containers (`base_CurrentEmittedContainer`, `card_AmbientEmission`,
0.5Perc×0.0Auto) and the cards/texts inside (0.0Auto²) all need this; after
the fix re-render and check the inner Row/Column nesting (drak's "Numbers
Container" brand entry now genuinely turns `base_NumericValues` into a
Column via the new FlexDirection modifier support).

### 2. Emissions IR/EM/CS labels (clone modifiers)

Authored: each `WidgetClone` (clone_IR/EM/CS in `gen_mc_s_emissions.json`)
carries `modifiers: [FieldModifierPair { target: ptr:5 (text_Abbreviation),
modifier: FieldModifierLocalization { field: "ParamInput0", value:
"@hud_Label_IR|EM|CS" } }]`. Our clone expansion
(`bb_scene/clone_expand.rs`) ignores `modifiers`.

Plan: during clone expansion, for each FieldModifierPair whose modifier is a
Localization: map `target` through `id_map` to the cloned widget; in the
CLONED ops, find the `BindingsLocalizedField { widget: <cloned target>,
field: <modifier.field> }` op, chase its `input` chain to the first
`BindingsLocalizedComponentParameter`, and append a `_SynthLocalizedParam_`
op sharing that (cloned) `_Pointer_` with `resolvedLocKey` = the modifier
value — the exact shadowing mechanism `inject_param_overrides`
(`bb_resolve/engine_parts/engine_01.part`, `inject_param_overrides`) already uses. Loc keys
`@hud_Label_IR/EM/CS` must exist in the localization map (verify; they are
standard HUD strings).

### 3. OUTPUT title position / 4. OFFLINE width

Diagnose after 1b lands (both are text-flow placements in the same card
machinery). For #4 note the OFFLINE text's effective font resolves 100 (no
inline style); its intrinsic row measure yields 543px. The reference shows
it fitting the 339px card — likely the correct font comes from a brand
standard that the parked tint/defaultStyles cascade work will apply, so
consider deferring #4 into item 5's re-land rather than fixing twice.

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
