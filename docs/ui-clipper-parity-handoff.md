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
`bash scripts/ui_check.sh --full` (487 ui lib tests passed, 1 ignored;
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
| 3 | OUTPUT card | title at right of header row; ref has icon→dots→title left-aligned | **FIXED 2026-06-11** — draw-metrics intrinsic measurement (below); title flows after the dots |
| 4 | Battery card | OFFLINE text container 543px overflows card, indented right | **FIXED 2026-06-11** — same fix; OFFLINE fits the card (draw-width box + 1b shrink) |
| 5 | All cards/pips/gauges | icons dark, separator dots invisible, gauge zone colours, pip brightness, backdrop bands, "2" white vs cream | **PARTIALLY LANDED 2026-06-11** — the defaultStyles cascade re-land (below) tints the system/card icons (Accent2/Base) and delivered the medical white X + annunciator rounded chiclets; residuals: "2" white vs cream (DIAGNOSED: our `UI_Generic_Flag_03 → Foreground` directive in `node_colour_directive_token` is right for the medical card headers — genuinely white in-game, platinum-pinned — but wrong for the power "2", cream/Base in-game; the discriminator needs authored-data archaeology on the medical ChoiceButton headers; power isn't a frozen target so no outlier vehicle — fix before the power freeze), separator dots, pip brightness, backdrop bands (A7 class) |
| 6 | Footer/scrollbar | good parity; faint track + backdrop band remain (A7 class) | Open — A7 backdrop class residual |

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

### 3. OUTPUT title position / 4. OFFLINE width — LANDED 2026-06-11

Root cause: the intrinsic text measure (TTF Mono at effective × 1.5,
mirroring the TTF fallback draw) overshot ~1.45× on screens whose text
draws through imported SWF fonts — the SWF path renders at the IR font
size with NO TTF calibration (`SB_UI_FONT_DUMP` for OUTPUT: size_px 50 =
effective 30 × host-stage 1.667, drawn width 160.43 vs measured box
232.7). The authored Right alignment put the slack on the left; in the
engine measure==draw so the box hugs the glyphs and alignment is moot.

Landed (specs `auto_text_child_prefers_draw_metrics_annotations` in
bb_layout + `compile_ir_uses_draw_text_measure_for_intrinsic_boxes` in
ui_ir, the latter with a no-measure control proving discrimination):

- `ui_ir::DrawTextMeasure` trait; the pre-layout annotation pass writes
  `_DrawTextWidthPx_`/`_DrawTextHeightPx_` (draw font size mirrored:
  styled brand sizes verbatim, plain + imageSizePercent boost);
  `node_resolved_text_size` prefers the annotations.
- `pipeline/text_measure.rs` implements it over the SAME
  `SwfAssetLibrary` + font selection as the compose draw (selection core
  extracted to assets-level `select_imported_ui_font_from_assets`,
  shared by both — measure == draw by construction). Width = the draw's
  advance primitive; height = its em line box. No SWF source → no
  annotation → the TTF ×1.5 estimate stays (that path's draw IS TTF).

Verified: OUTPUT flows icon→dots→title; OFFLINE fits its card; emissions
unchanged. `ui_check.sh --full` ALL GREEN with NO baseline drift — the
anticipated gold-target adjudication was not needed (its pinned elements
aren't in Auto-intrinsic flows beyond thresholds).

### 5. Tint/defaultStyles cascade re-land — LANDED 2026-06-11 (79c87e8ea)

Three TDD'd rules: (a) `apply_canvas_style_cascade` applies the canvas's
own `defaultStyles.entries` as cascade BASE (style-link < defaultStyles <
shared < brand < embedded < inline); (b) `FieldModifierColor` with an
entries-only palette (no `colorStyles`) keeps its TOKEN for the
render-time resolver; (c) the token-only path CLEARS a stale RGBA an
earlier pass wrote for the field (the per-pass WidgetIcon overlay default
had written Base RGBA that shadowed the bioc `Bright` token on the medical
X). NOTE: the originally-suspected ir_compose custom-shape fill gap did
NOT materialise — once tokens flow correctly the existing fill_override
path tints; no compose change was needed.

Audited re-freezes (IR + artifacts, approver tom) with per-target pixel
decomposition — pre-cascade renders were byte-identical to the previous
frozen hashes: ui_target_a +196px (white X, toward reference);
eng_annunciator +27.7k px (rounded chiclet corners + border insets, toward
reference); clipper_target_master +320px (subtle footer title-card border
band; chevrons byte-untouched); ui_target_b/small_door byte-identical.
DON'T retry the "enableColorOverlay+null → Base" overlay default (regressed
target-screen chevrons; entry-driven FillColor is the engine model).

### 6. Medical bed (plan Steps 10–13) — status 2026-06-11 evening

- ~~White X~~ DONE (item 5's re-land + stale-RGBA clear; verified vs
  medical1 reference, 196px delta, platinum re-frozen).
- ~~Bioticorp logo −12px~~ DONE (4f8532f4e): `draw_manufacturer_logo_ir`
  now honours the authored asset layout (Contain at containPosition 0,0 —
  the square 1024-viewBox SVG width-fits its 120×140 box top-anchored) and
  the `vertical_alpha_balance_offset` recentring heuristic is DELETED. Bed
  logo measures pixel-exact vs the reference (rows 44–83); end-of-bed
  matches within its capture offset. Artifact re-freeze done (ui_target_a/b
  ±2510px in the logo box only).
- ~~Position outliers~~ REGISTERED as reference-anchored known-outliers
  (`ui_known_outliers.json`, 4 entries on ui_target_a): T3 + MEDICAL
  ASSISTANT primary_text_top 77→82 (~5px high), PATIENT NAME pair primary
  1004→999 / secondary 1032→1029 (~5px/3px low). True fix = the
  caption/heading line-box baseline model (A5 residual class, opposite
  signs in two pair archetypes — no single offset rule).
- 64→69px close-button — ENGINE RULE DIAGNOSED, implementation deferred:
  the frame node (`ComponentRoot`, authored 64×64, border 3px, radius 6)
  draws its border OUTSET in-game (content-box: 64 + 2×3 = 70 visible;
  reference measures 68–70 at the aligned right edge). Our border draws
  inset (64 visible). NOTE: a w/h known-outlier is the WRONG vehicle —
  outset borders are paint-only, the snapshot rect stays 64 and would
  never graduate. Implementation = outset border drawing scoped to
  expanded-standard component roots (own `canvas-proxy-root` tag);
  blast radius includes the GOLD target master's two 212×105.6 side
  buttons (3.33px borders) — measure those frames on the target reference
  before landing, then artifact-adjudicate every target.

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

### 9. Annunciator rounds 2-3 + MFD body backplate — LANDED 2026-06-12 (81c93109e, f436b446f)

Annunciator (eng_annunciator_master_left, gold):
- defaultStyles.entries = EDITOR-TIME defaults, never applied at runtime
  (square chiclet frames, white power system icons prove it).
- Pending-state entry deferral + Ancestor breaks-everywhere containment;
  the 16-function annunciator heuristic cluster in ui_ir is GONE — chiclet
  borders/fills/text colours are entry-driven (square 3px Base border,
  WPN Moderate amber + dark text, COOL grey 143 Off-Text).
- Styled PascalCase `ImagePath` overrides the authored image source in
  `collect_node_asset_refs` (empty styled path clears it — ARGO flat-fill
  variant): image_BG draws the near-black DRAK_Background_anunciators.tif
  brand swap (authored alpha 1.0; the texture itself is near-black warm).
  Reference background away from bloom is neutral ~(5-9) — the remaining
  brightness delta vs our (33,20,8) plate body is the in-game CRT/capture
  side (Tom handles the CRT effect in the Blender shader).

MFD body backplate (power/target/radar/all MFD content screens):
- M_Eng_MFDContent authors background_Main (WidgetBodyBackground,
  backgroundType Texture) skinned by the modularkit
  BodyBackgroundWidgetStandard: the s_drak_hud container authors
  ImagePath DRAK_GroundVehicle_Dashboard_background_2.tif, scaling Fill,
  **Alpha 0.2** over the Background-token fill — all verbatim DataCore.
- background_Main.Instantiated binds host boolean `backgroundenabled`:
  bb_state_filter now consults the default-value registry for boolean
  variables with no authored/inherited static value; registry pins
  backgroundenabled=true (provenance: all 4 Clipper screen references).
- The standard's brand container is matched by the OWNING canvas's
  selected brand identifier (s_drak_hud), applied only at the canvas that
  authors the body widget — the manufacturer-prefix scan hit the shared
  standard's s_drak_env (door-panel) container first.
- Artifact freeze: clipper_target_master hash changed (gains the plate,
  matches Screen_Right_Upper_RTT.png); other targets byte-identical.

### 10. Annunciator round 4 — near-black bg + tinted glow — LANDED 2026-06-12 (16d595dbb)

- Registry `EnableBackground` -> FALSE (single consumer in the whole UI
  tree: `H_Eng_Annunciator.image_BG.IsActive`). In-game the strips are
  near pure black (cockpit screenshot beside backplated MFDs); round-3's
  full-brightness backplate is overturned. MFD body plates (item 9) are
  the separate BodyBackground mechanism and stay.
- White alpha-mask textures (shape entirely in the alpha channel:
  Annunciator_On.tif, F_Common_Gradient_128px.tif) with
  `svgFill.enableColorOverlay` take the brand `Base` overlay at the image
  blit (`image_tint_for_blit` + `image_is_white_alpha_mask`;
  `UiIrNode.colour_overlay_enabled` render-only hint). MRAI authors the
  same mechanism as explicit FillColor entries on its white masks.
  Coloured textures stay untinted (medical photos / MFD plates).
  Door artifact changed with the same rule (warm bottom haze, matches
  the door reference's warm body).

**OPEN — GATED ON TOM: linear-light compositing.** The glow renders
darker than the reference because the engine composites in LINEAR light
and our renderer blends in sRGB space. Numbers: predicted linear-light
(39,20,3)/(68,38,8) vs reference (45,25,7)/(71,48,15) at the chiclet
top/side edges; our sRGB-space blend reproduces the rendered
(6,4,1)/(13,8,3) exactly (texture alpha 52/146 of 255 x widget 0.1 x
Base). Switching the compositor is renderer-wide (every alpha blend,
text AA included) => full gold/platinum re-freeze + re-adjudication of
all targets. Do not partially apply (image-only carve-outs are not
engine-faithful).

### 11. Annunciator round 5 — WPN glow off + linear mask glow — LANDED 2026-06-12 (f5040aeab)

- `state_driven_image_activation` DELETED from ui_ir (tag-NAME-keyed
  force-activation of inactive State*/Flashing images): WPN's glow is
  correctly entry-gated off (NotTag StateModerate); IsActive is
  entry-driven only.
- The white-mask overlay glow composites in LINEAR LIGHT
  (`blit_white_mask_overlay_linear` in ir_compose): chiclet edge
  (64,35,6) vs reference (71,48,15), top (46,24,3) vs (45,25,7). This
  CLOSES the glow-brightness half of the linear question for the mask
  category; the renderer-wide linear migration (text AA, fills) is
  still the gated item 10.
- Gold re-frozen (user-directed) for eng_annunciator_master_left; the
  door artifact moved with it (same white-mask category). Medical
  platinum + target master byte-identical; no IR drift (draw-only).

### 12. Power MFD review round 2 — catalog (2026-06-12, render /tmp/power_v3 vs Screen_Left_Lower_RTT.png)

Review phase per docs/ui-workflow.md §4 (after the body backplate landed —
overall warmth now matches; crops read with vision at full scale):

| # | region | difference | sev | root-cause hypothesis | decision |
|---|---|---|---|---|---|
| P1 | output_card | "2" renders WHITE, reference shows cream/Base like "/ 16" | M | our invented `UI_Generic_Flag_03 -> Foreground` directive in `node_colour_directive_token`; the power card has NO DRAK entry on that flag (only a GRIN geometric entry); medical archaeology: menuoptioncard's flag entries are hover/disabled BG fills, TierLevel carries the flag with NO colour entry — hypothesis: the flag is NOT a colour signal at all; medical white comes from the bioc brand TEXT STYLE, power cream from the drak caption-pair style | FIX FIRST: delete the directive arm (TDD); platinum guards adjudicate the medical side |
| P2 | output+battery cards | card icons (battery glyphs) dark vs bright orange | M | icon tint token on card icons — check IR tokens (cascade re-land was supposed to tint Accent2/Base) | FIX (investigate with P5) |
| P3 | both cards | dotted separator (vertical dot column) between icon and title missing | M | separator element not rendered (documented "separator dots") | FIX after P2 |
| P4 | columns | pip slabs saturated orange vs pale/washed in ref | L-M | pip fill colour role (Bright vs Base) + capture bloom | INVESTIGATE after P1-P3 |
| P5 | columns bottom | `>>` chevron glyph dark vs bright | M | glyph tint — likely same family as P2 | FIX with P2 |
| P6 | columns | red pip outlines + white mid-column slab in ref | — | documented mouse-hover capture artifacts (ui-workflow §4.3) | EXCLUDE (explicit) |
| P7 | below columns | orange scrollbar underline position/length differs | L | scrollbar geometry | DEFER |
| P8 | footer | letter pitch ~6% (long-standing) | L | global SWF LetterSpacing model | DEFER (documented) |

Order: P1 -> P2+P5 -> P3 -> P4; P6 excluded; P7/P8 deferred.

P1 LANDED 2026-06-12: UI_Generic_Flag_03 directive deleted (flag is not
a colour signal); brand text-style FillColor fallback un-gated (a style
tag only overrides when an entry/directive maps it to a colour). "2" now
cream like "/ 16". Medical re-frozen: headers Foreground->Bright (the
authored s_bioc H2 entry), H1 Base + H6 Bright tokens added to the
previously token-less fields; visually adjudicated vs medical1.
P2+P5 LANDED 2026-06-12 (next commit): entry-less colour-overlay
WidgetCustomShapes (renderShape + svg source + no resolved tint/fill)
take icon token `MissionObjectives` (BB_ColorStyle enum 16; drak slot 16
(243,220,110)); compose maps the token to slot 16. Evidence: all five
power icons (gun/thrusters/shield system icons + OUTPUT/BATTERY card
glyphs) share the slot-16 hue on the reference capture, distinct from
Base (footer) and Bright (slabs) on the SAME capture; misc/orig author
explicit at-rest "System Icon Color" entries while DRAK authors only the
hover state; HUD records author FillColor=MissionObjectives for generic
Icon Styles. The thrusters >> chevron (black SVG) is visible for the
first time. No frozen-target drift.

### P3 separator dots — PARKED 2026-06-12 (full diagnosis, ~30min to land)

The dotted icon/title separators are `BuildingBlocks_WidgetSeparator`
widgets (direction Vertical, style Tertiary on the power cards) — a
modularkit standard family: 6 records
`modularkit/standard/widgets/{vertical,horizontal}separator{primary,secondary,tertiary}widgetstandard.json`,
each a single ComponentRoot WidgetCustomShape whose PER-BRAND container
authors the visual (drak env: `DRAK_S42_seperator_vertical_2.svg`,
EnableColorOverlay=false, nine-slice). A working implementation was
built and REVERTED for two blockers (the diff survives in this
session's transcript; all pieces below were verified):

1. bb_scene: add `BbNodeType::WidgetSeparator` (+3 exhaustive matches:
   bb_layout type_name_str, ui_ir node_type_name "widget_separator",
   compose draw_node no-op host arm) and convert the 3 existing
   `Other("BuildingBlocks_WidgetSeparator")` matches in ui_ir
   engine_02.part; add `"Separator"` to node_type_matches.
2. Expansion (bb_resolve engine_04 expand_widget_standards): include
   WidgetSeparator hosts, template by direction/style, no params.
   **BLOCKER A**: instances consume the shared 0xF000_0000 band counter
   and SHIFT the frozen platinum instance ids (medical close-button X
   4026531855 became a separator). Fix: a second band (e.g.
   0xF800_0000) for separator merges in merge_child_scene
   (engine_01.part EXPANSION_ID_BASE), or per-host-type band lanes.
3. Brand application (apply_separator_standard_styles, engine_01.part,
   subtree-scoped via apply_scene_style_entries_in_subtree with brand
   fills + Style-record chrome): **BLOCKER B**: brand selection. Exact
   canvas-selected identifier + hud<->env sibling works for power
   (s_drak_hud -> s_drak_env) and medical bed (s_bioc container exists
   in the standards), but the medical FOOTER component selects
   s_aegs_env via the IC_* single-container rule and exactly matches
   the standard's s_aegs_env => AEGS divider leaks into platinum. Needs
   the typography-table model (collect_standard_text_styles
   selected_style_name: canvas:<style-link> else s_<mfr>_{hud|env} by
   canvas family) instead of resolve_brand_style on the component
   record.
4. The overlay-icon default (P2) must respect a styled
   `EnableColorOverlay=false` (PascalCase raw override) so the
   separator SVGs are not MissionObjectives-tinted.
5. Test fixtures need the tag database served (expansion bails without
   it).

NEXT after P3: P4 pip slab brightness (Bright vs washed reference —
bloom caveat), P7 scrollbar geometry, P8 letterspacing.

### 13. Power review round 3 — session 2026-06-12 (user symptom list)

**Committed:** `a30761f20` — P9: per-axis Overflow clip. `clip_rect_for_node`
hard-clips only axes whose `fade<Axis>` flag is FALSE (the power Scrollview
authors Clip + fadeXAxis=true/fadeYAxis=false; the in-game capture shows the
col-3 temp gauge complete, unfaded, past the viewport edge). Spec test
`compile_ir_clip_rect_skips_fade_axes`. Diff confined to the gauge box.

**UNCOMMITTED working tree** (lib tests 512 green; live IR guard trips — see
below): the tag-conditioned text-format mechanism, discovered via the power
font/colour symptoms and verified numerically against the reference captures:

- **Mechanism (landed in tree):** a style entry whose conditions are
  `Parent(...)`-wrapped selects a `WidgetTextField`'s IMPLICIT TEXT-FORMAT
  CHILD — i.e. it styles the text of fields whose OWN tags satisfy the
  unwrapped conditions. `entry_matches_text_format` +
  `apply_entry_text_format_modifiers` (bb_brand_apply): only text-format
  modifiers apply (FontSize, AutoFontSize, Fill/StrokeColor, Letter/Line
  spacing, font record); `Type(...)` never matches (footer counterexample).
  Route gated to MANUFACTURER BRAND containers (`s_*` identifiers):
  embedded = state sheets ('Bright Elements', 'Textfield_BrightColor_Override'
  — at-rest refs show neither), shared = generic sheets (mfd_g_emissions
  'Header Text' Accent1 does NOT show on emitted values in-game).
- **Size table sources (all verified vs ref caps):** M_Eng_MFDContent drak
  brand container: Size_1→45 / Size_2→45 / Size_3→40 / Tertiary→40
  (+AutoFontSize=false); gen_mc_s_poweroutputinfo drak: Parent(Bright)→70
  ("2"), Parent(Text_Header)→40 ("/16"); M_Eng drak 'Bright Orange Objects'
  Parent(Text_Body ∧ ¬EmissionAbbreviation) → FillColor Base — THE user's
  "lighter orange" for 294.1/0.0 ambient AND battery OFFLINE/0/0 (IR/EM/CS
  excluded by the NotTag ⇒ they keep the H1 deep orange = "the orange 3.5K
  has"). `__EntryFontSize` marker (modifiers_number.rs) ranks entry sizes
  ABOVE the named-style table (engine: instance entries override standard
  styling; drak H1 standard = 60 buried the entry 40 otherwise).
- **Entry/inline sizes are VERBATIM (styled), not boosted**: medical mainmenu
  banner entry FontSize 40 renders cap 25px ≈ ref 27px; the power texts'
  apparent ×4/3 is the MFD CONTENT canvas text scale (see open items).
- **Accent2 = BB enum index 5** in compose `resolve_colour_token` (was
  hand-mapped to 1/2): NO TARGET authors `Base/Bright Elements`
  FillColor=Accent2 and renders s_drak_hud slot 5 (222,88,3) — pixel-identical
  to the old frozen rgba, token-only drift. Old pin updated
  (`color_style_tokens_resolve_to_palette_slots`).
- **Deferred late-state passes** now carry their ORIGIN container identifier
  (brand `s_*` vs "embeddedStyles") so state-deferred entries keep
  container-class semantics (collect_late_state_style_entries signature).
- Probe upgraded: `BB_A3_STYLE_PROBE=1` now prints `text_format=[...]`.

**Guard adjudications (NOT yet re-frozen):**
- `ui_target_a` 2147483698 banner tint Base→**Bright**: CORRECT — photometric
  vs medical1 (banner R-norm (1,1.12,1.26) ≈ bioc Bright(197,200,216) under
  the capture's cast; bioc Base(115,198,254) would read (1,~1.9,~2.7) like the
  lower-left text anchor). Re-freeze when tree settles.
- `clipper_target_master` 31 NO TARGET token Base→**Accent2** + rgba
  Some→None: pixel-identical (222,88,3); authored-entry-driven token.
  Re-freeze.
- `ui_target_a` 2147483697 **T3 tier label font 90→40 = REGRESSION, OPEN**:
  ref shows T3 large (≈ frozen). Suspect = mainmenu bioc 'New Style'
  (Parent(Tag 0964d22f…) → FontSize 40 + FillColor Bright) reaching T3 via
  the route — need to identify the tag (0964d22f-3a22-4052-92e4-eaf77f975423)
  and the discriminator separating the banner (takes 40 ✓) from T3 (must
  not). DO NOT freeze until resolved.

**Power render state** (/tmp/power_v8 was the boosted variant; with verbatim
sizes expect OUTPUT cap 40 ✓, but emissions/"2"//16 at ~40/93/53×(3/4) until
the content-scale item lands):

| sym | status |
|---|---|
| 1 gauge clip | FIXED (committed) |
| 2 header sizes | mechanism landed; final size needs content text scale ×4/3 |
| 3 ambient lighter | FIXED in tree ('Bright Orange Objects') |
| 4 IR/EM/CS colour | OPEN — `Icon Color` (shared mfd_g_emissions, normal match) recolours the abbreviation to slot-0 Base; in-game keeps H1 deep orange. Evidence (incl. footer `SelectedName` comment in conditions.rs) suggests shared-record colour entries don't restyle textfield text — needs its own scoped rule + battery adjudication |
| 5 side bars red | NOT STARTED (likely Accent1=(243,80,77) enum idx 4) |
| 6 titles 130% | FIXED in tree (inline 30 verbatim ... ×content scale → 40) — verify after content-scale |
| 7a/b "2"/"16" | entry sizes land; exact match rides content scale |
| 7c separator dots | PARKED (P3 diagnosis §12) |
| 8 battery texts | colours FIXED in tree; sizes ride content scale; OFFLINE cap ref 43 vs predicted 53 unexplained (band-fit?) |

**OPEN: MFD content text scale ×4/3.** Independent evidence: the ºC glyph
(non-entry, authored 26) renders h=27 vs ref 37 (×1.37). Frame texts (footer)
are correct at ×1.667, so the CONTENT canvas needs ×2.222 ≈ 1.667×4/3.
Candidate structural sources (UNVERIFIED): 1200/540 (target_h over
0.9-scaled content stage height — canvas_LandscapeMFDView authors h=0.9), or
the root-SWF imageSizePercent applying once at the host-stage conversion.
`design_text_scale` is computed in `pipeline/mod.rs:330`.

**P7 scrollbar slider** (~393 vs ref ~432) and **P9-residual** (in-game
viewport may genuinely differ; the slider-width math suggested
0.7×padding-box, but the card-width/scroll-position evidence is
capture-skew-limited) — both documented in this session's analysis; defer.

### 15. Route LANDED + hard-coding remediation — DONE 2026-06-12

Commits 5bf1d7f84 (RgbaColor guard/fixture/neutral-fallback), b09c2d98a
(Phase 1 colour maps → BB enum), 46b81a25a (register refresh + phases 2–5
docs), 07c821a83 (**the text-format route LANDED** — T3 resolved: a
LITERAL widget match does not outrank the named-style table, only
TEXT-FORMAT-routed FontSizes do; `__EntryFontSize` set only by the route;
audited IR re-freeze approver tom, 6-line delta: banner Base→Bright +
40.005→40.0, NO TARGET Base→Accent2 token-only), 4803d3c48
(FIXED_BAND_HEADING_FILL RETIRED — the authored mainmenu entry sizes the
banner; harness 26/26 zero drift).

§13's catalog state updates: the route is now COMMITTED, both
adjudications frozen. STILL OPEN for the next power session: the MFD
content text scale ×4/3 (ºC discriminator), IR/EM/CS colour (shared
'Icon Color' restyling question), P13 side bars, P3 separator dots, P7
slider width, the §12 P4 pips. The remediation backlog lives in
`docs/ui-hardcoding-remediation-plan.md` (Phase 3 directive-arm audit now
UNBLOCKED; TTF calibrations registered for their own arc).

### 14. USER PRIORITY ITEMS 2026-06-12 (tasks #8–#11) — DONE (see §15)

Tom found hard-coded `RgbaColor { r:.., g:.., b:.. }` literals (e.g.
`ir_compose/engine_parts/engine_02.part:725` — `r:115,g:198,b:254` bioc Base
in a test fixture; this session added more such TEST fixtures incl.
`r:222,g:88,b:3`). Ordered work, before continuing the power arc:
1. Update the docs so the anti-hard-coding rule is SELF-CORRECTING (an agent
   finding existing hard-coded values must replace/flag them, never extend
   the pattern; if the ban's scope wasn't obvious, make it so).
2. Crate-wide guard test detecting literal `RgbaColor{..}` construction in
   starbreaker-ui production code (alongside existing anti-hardcoding tests
   / `check_ui_hardcoding.sh`); tests exempt per Tom's item 4 — but his cited
   example IS in a test fixture, so clarify scope with the doc update (test
   fixtures that encode REAL palette values should derive them from game
   data too, or be clearly synthetic).
3. Replace all production hard-coded colours with game-data lookups
   (DataCore `colorStyles` etc.).
4. Scan the crate for remaining hard-coding (tests exempt) → phased plan doc
   with actionable todos.

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
