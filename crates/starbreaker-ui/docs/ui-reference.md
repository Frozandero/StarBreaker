# UI parity reference — commands, tools, data

The lookup half of the UI workflow (`crates/starbreaker-ui/docs/ui-workflow.md` is the process).
Every command here was executed during writing (2026-06-11). Paths marked
(ws) are workspace-specific (this machine); the repo-relative ones are
universal.

## 1. Build & test

```bash
cargo build                      # debug — fast, fine for ALL iteration incl. renders
cargo build --release -p starbreaker   # only for deploy / canonical exports

bash scripts/ui_check.sh         # TDD tier: examples compile + the WHOLE ui suite
                                 #   (lib + every integration target; the two
                                 #   export-coupled visual guards skipped)
bash scripts/ui_check.sh --full  # boundary tier: + those export-coupled visual
                                 #   guards, freeze+artifact validators, 3d lib,
                                 #   font harness
cargo test --workspace           # everything (slow)
```
**`--full` does NOT re-export.** The export-coupled visual guards compare
`ships/Data/UI/Generated/*.png`, which only refresh on a full export — run one
FIRST or `--full` fails the staleness guard (`STALE EXPORT: Generated PNGs
predate the current build`), which looks like breakage but is the P0.2 guard
working. Re-export with the canonical guard command (§2) before `--full`:
```bash
cargo build --release -p starbreaker && \
./target/release/starbreaker entity export drak_clipper "$HOME/projects/scorg_tools/ships" \
  --kind decomposed --lod 0 --mip 0 --materials all
```
`--lod 0` is REQUIRED (LOD1 culls the cockpit HUD screens, §5).

Individual suites for targeted debugging:
`cargo test -p starbreaker-ui --lib [filter]`, `--test manifest_live_ir_guard`,
`--test line_count_guard`, `--test manifest_snapshot_regression`,
`--test manifest_visual_regression`. Hardcoding guard:
`bash scripts/check_ui_hardcoding.sh`.

## 2. Render & export

**Replay (iteration, ~1 min, debug binary is fine).** Prefer the wrapper —
it always rebuilds first and prints the binary mtime (a background shell's
cwd reset twice caused renders from a STALE binary that looked like a fix
had no effect):
```bash
bash scripts/ui_render.sh --helper Screen_Annunciator_L [--ir] \
  [--lod 0|1] [--scene <scene.json>] [--out <dir>]
```
The scene is picked automatically: `--scene` wins; else `--lod`; else derived
from the helper — the cockpit dashboard screens use LOD0 (`*_RTT`, the HUD
gauges `Screen_Small_Radar*` / `Screen_Central_Compass` / `Countermeasures_Screen`
/ `screen_flight_hud*`, and `Screen_Annunciator_*` — all on the LOD0 CGA; the
small HUD screens are CULLED in LOD1, ledger 47), the interior usables
(medical, door) LOD1. So the power screen no longer needs the long `--scene`
path: `ui_render.sh --helper Screen_Left_Lower_RTT --ir`.
Raw form (when the wrapper's defaults don't fit):
```bash
./target/debug/starbreaker ui render \
  --scene "$HOME/projects/scorg_tools/ships/Packages/DRAK Clipper_LOD0_TEX0/scene.json" \
  --out-dir /tmp/ui_replay \
  [--helper Screen_Left_Lower_RTT]   # filter to one screen
  [--dump-ir-dir /tmp/ui_replay/ir]  # write composed IR JSON per helper
  [--mip 2]                          # smaller output
```
Replay renders every `ui_bindings` entry through live DataCore/P4K fetchers
and derives ship values from the scene's root entity — replay == export.
P4K location is auto-detected; do NOT set `SC_DATA_P4K` unless pointing at
non-default data, and never set `RAYON_NUM_THREADS=1` except when
benchmarking.

**Full export (canonical PNGs under `ships/Data/UI/Generated/...`, ~50s
as of 2026-06-12 — cheap enough to re-export before ANY artifact
comparison; the `Generated` PNGs are only as fresh as the last export and
a stale comparison mis-adjudicated a real regression as "zero drift" on
2026-06-12):**
```bash
./target/release/starbreaker entity export drak_clipper \
  ~/projects/scorg_tools/ships --kind decomposed
```
Required before artifact freezes. The generated PNGs are written near the
END of the run — never diff/freeze until the process exits.

**Freeze tooling** (flows in `crates/starbreaker-ui/docs/ui-workflow.md` §7). The whole artifact
cycle (release build → export → stale-comparison cleanup → freeze → both
validators → full battery) is one command:
```bash
bash scripts/ui_freeze_cycle.sh --approver <name> --reason "..." [--skip-export]
```
Individual steps:
```bash
bash scripts/add_ui_regression_target.sh --id <id> --tier <gold|platinum> \
  --source-generated-png ships/Data/UI/Generated/ship/<mfr>/<Ship>/<file>.png
bash scripts/freeze_ui_snapshot_ir.sh --approver <name> --reason "..."
bash scripts/validate_ui_snapshot_freeze.sh
bash scripts/generate_ui_regression_artifacts.sh
bash scripts/freeze_ui_regression_artifacts.sh --approver <name> --reason "..."
bash scripts/validate_ui_regression_artifacts.sh --quick
bash scripts/validate_ui_regression_repo_only.sh   # hosted CI (no game data)
```

**Registering a NEW gold/platinum target** (ledger 44 — `clipper_power_master`
was the first new gold in a while and the ordering bit). `ui_freeze_cycle.sh`
alone is INSUFFICIENT for a new id: it freezes the image artifact but omits the
IR snapshot freeze, so its validator then hard-fails `snapshot freeze ids do not
match manifest ids`. Do, in order:
```bash
bash scripts/add_ui_regression_target.sh --id <id> --tier <gold|platinum> \
  --source-generated-png ships/Data/UI/Generated/ship/drak/Clipper/<canvas>.png
# bump the hard-coded count in tests/manifest_visual_regression.rs
#   (manifest_contains_expected_visual_targets: targets.len() == N) and add an
#   `any(id == "<id>")` assert.
bash scripts/freeze_ui_snapshot_ir.sh --approver owner --reason "…"   # reads a delta
bash scripts/ui_freeze_cycle.sh --approver owner --reason "…"          # image artifacts
bash scripts/validate_ui_snapshot_freeze.sh                            # expect "N target(s)"
```
The generated source PNG is `buildingblocks_canvas_<canvasname>.png` (e.g.
`…_mc_s_power_master.png`), written by the FULL `entity export`, not `ui render`.

## 3. Comparison & screen dossier

```bash
python3 scripts/ui_compare.py <render.png> <reference.png> \
  --regions <preset> --out-dir /tmp/ui_compare [--stats]
python3 scripts/ui_compare.py --regions list   # available presets
python3 scripts/ui_compare.py <render> <ref> --box x0,y0,x1,y1 [--box …]  # ad-hoc region(s), no preset
```
`--stats` prints per-region bright/dark pixel means + R-normalised ratios
for both images — the photometric review method. Judge HUE from the ratios,
never raw values: every reference capture has its own cast (G/B attenuate)
and bloom lifts B near bright elements. Before judging an unknown colour,
measure a known anchor on the SAME capture (footer text = Base, pip slabs =
Bright); a colour matches a palette slot when the RATIOS line up under that
cast. This method identified the linear-light compositing gap and the
MissionObjectives icon slot. The refined additive-haze form (measured_ratio
≈ true_ratio + haze_offset, solved from the anchor) is implemented in
`scripts/ui_measure.py` (`--anchor` / `--anchor-rgb`; model documented in
its docstring), which also measures glyph cap heights with
edge-contamination flagging — prefer it over ad-hoc pixel maths.
The reference is auto-scaled to the render width before cropping; READ the
emitted `cmp_*.png` files with vision. Reference screenshots are imperfect
(skew/bloom/capture artifacts; power-screen pip outlines are mouse-hover
artifacts) — compare structurally. Skew is correctable: store the capture's
four screen-corner pixel coordinates as `<reference>.corners.json`
(`{"tl":[x,y],"tr":..,"br":..,"bl":..}` — in GIMP, hover each screen bezel
corner and read the pointer coordinates off the status bar) and
`ui_compare.py` homography-rectifies the capture onto the render rectangle
automatically, printing "rectified via <file>" (or pass `--rectify
<corners.json>` explicitly). **Thin-feature COLOUR caveat (ledger 35):** the
warp interpolates a few-px-wide feature (a bar/stroke/dotted separator) with
whatever sits behind it, smearing its hue toward the background — a 2px Accent1
header bar measured G/R 0.64 ≈ Base on the *rectified* power reference and a real
colour bug was nearly closed as "faithful". Rectify for POSITION; judge a thin
feature's COLOUR on the CRISP ORIGINAL (`ui_measure.py` warns when
`feature_width ≤ 4`).

**Screen dossier** — one row per known screen (extend as screens are
worked). References live in
`~/projects/scorg_tools/reference/in-game/Clipper/` (ws); generated
PNGs in `ships/Data/UI/Generated/ship/drak/Clipper/` named
`buildingblocks_canvas_<canvas>.png`; scenes in
`~/projects/scorg_tools/ships/Packages/` (ws). **Reference variant:** when a
screen has several captures, prefer the straight-on one that carries a
`<name>.corners.json` sidecar (`ui_compare` auto-rectifies it) over the
dossier's legacy name — e.g. the power screen uses `Screen_Left_Lower_RTT_dark.png`
(rectifiable), NOT the un-cornered `Screen_Left_Lower_RTT.png`. (Then heed the
thin-feature colour caveat above: rectify for position, measure thin colour on
the original.)

| Screen | Helper / scene | Canvas | Reference image | Preset | Tier / target id | Open issues |
|---|---|---|---|---|---|---|
| Power MFD | `Screen_Left_Lower_RTT` / LOD0 scene | `MC_S_Power_Master` (via `M_MFD_Screen` frame) | `Screen_Left_Lower_RTT.png` | `power` | **GOLD `clipper_power_master`** (frozen 2026-06-14) | card width + battery icon DONE 2026-06-14 (data-driven AspectRatioToTag→"Content Canvas Scaling", `pipeline/aspect_tag.rs`; cards ~437, icon ~67px); open: P3 separator dots, P4 pip brightness, P13 header side bars, P7 slider width (plan P2.2b), P8 letter pitch (`crates/starbreaker-ui/docs/ui-clipper-parity-handoff.md`) |
| Target MFD | `Screen_Right_Upper_RTT` / LOD0 scene | `MC_S_Target_Master` | `Screen_Right_Upper_RTT.png` | `target` | GOLD `clipper_target_master` | A7 backdrop stack remainder (handoff) |
| Medical bed | usable screen / LOD1 scene | `I_Med_MedicalBed_A` | `screen_16x9_a-[medical1].png` | — (add) | PLATINUM `ui_target_a` | handoff steps 10–13 |
| Medical end-of-bed | usable screen / LOD1 scene | `I_Med_MedicalEndOfBed_A` | `mesh_end_screen_plane-[medical2].png` | — (add) | PLATINUM `ui_target_b` | logo −12px check (handoff) |
| Small door | usable screen / LOD1 scene | `I_Door_Small_DRAK` | `Door-closed.png` | `door` | GOLD `clipper_small_door` (re-frozen 2026-06-14, 1920×1132) | render aspect 1.70 from the `rtt_screen` mesh (was the 16:9 canvas) |
| Annunciator L | `Screen_Annunciator_L` / LOD0 cockpit | `H_Eng_Annunciator_Master_Left` | `Screen_Annunciator_L.png` (resized to match) | `annunciator` | **PLATINUM `eng_annunciator_master_left`** (re-frozen 2026-06-14, 1920×344) | aspect 5.58 from the screen quad (was 4.44 SWF content-crop) |
| Annunciator R | `Screen_Annunciator_R` / LOD0 cockpit | `H_Eng_Annunciator_Master_Right` | (mirror of L — no separate capture) | `annunciator` | **PLATINUM `eng_annunciator_master_right`** (onboarded + frozen 2026-06-14, 1920×344) | aspect 5.58 (mirror of L) |
| G-force ball | `Screen_Small_Radar2` / LOD0 cockpit | `HC_HUD_Ship_G_Force_Ball_Master` | `g_force_ball_master.png` | `target` | **PLATINUM `clipper_g_force_ball_master`** (frozen 2026-06-15) | PARITY PASS 2026-06-15 (owner-approved platinum): diagram centred (cover-fit recentre clamp relaxed + scoped to overflowing axes only, so aspectOverrides door/annunciator are untouched), cross SQUARE (flex `shrinkProportion=0` now honoured → `card_BallArea` stays square, readouts overflow/crop), centre dot now a true CIRCLE (`rounded_rect_path` cubic arcs for full ellipses), cardinal+diagonal markers symmetric at the axis ends, at-rest LIMIT "0" hidden (`AccelerationLimiterRatio=1.0` registry pin → `base_LIMIT Text` IsActive false), right-edge readout cropped off-screen. Earlier history: aspect 1.0 square; blank-at-rest FIXED 2026-06-14 (`bb_state_filter` sole-root `Instantiated` exemption — class-wide, also unblanked velocity ball/countermeasures). Correct content = DRAK ADV ball (G1a generic-vs-drak angle FALSIFIED+reverted). C1/G1b square-gauge centring LANDED 2026-06-14 (owner-approved): `cover_fit` (uniform max-scale) + `cover_fit_recentre` (clamp-centre on the largest LOCALISED node = the ball-area) in `bb_layout`, gated to the `screen_aspect_w_over_h` `useRaw` branch — g-force (ball-left) + velocity (ball-right, mirrored) both centre & fill, readouts overflow/crop; ui_check GREEN, frozen screens untouched. C2 = bezel geometry (square RT correct, no change). C3 spoke colour cyan→orange LANDED 2026-06-15 (`bb_svg::parse_uniform_colorstyle` + `ir_compose::svg_colorstyle_fill_override`: resolve the `colorstyle:<role>`/`opacity:` SVG path-id convention → brand surface palette × opacity, uniform-role only; the literal `#6CB8C7` is an Illustrator placeholder). Generic across HUD glyph SVGs; 0.0000% diff on all frozen targets. C4 solid markers LANDED 2026-06-15: WidgetCircle `doFill`+`fillColor` was unhandled (`draw_widget_circle` only stroked); now `ui_ir` captures `circle_fill_colour_token` (doFill-gated) and `ir_compose` fills via the brand surface palette — the 2 solid diagonal dots render (frozen targets 0.0000% diff). The 4 cardinal markers are circular RINGS (correct; owner clarified the ref markers are circles). **C6 white centre dot LANDED 2026-06-15** (`ui_ir::is_untinted_overlay_render_shape`→white overlay-identity fill + `ir_compose::fill_rounded_rect_ts_with_mode`; shared with velocity V2b). **Cardinal-marker rotation+positioning LANDED 2026-06-15** via the velocity V4 fix (these `base_Cap*` markers are the same display-widget+SVG structure with z=90 rotation + flipV; `apply_node_rotation` + `flip_adjusted_contain_position` make them symmetric at the diagram edges — top 768/bottom 767, left 717/right 717). Background centred via V5. Residual: g-force diagram is ~63px left of screen centre (cover-fit ball-area centring) and 1436 vs 1536 wide (slightly squashed cross); cardinal markers are circles vs the reference's rounded squares (owner-confirmed circles) (see [[gforce-hud-blank-fix]]) |
| Velocity ball | `Screen_Small_Radar1` / LOD0 cockpit | `HC_HUD_Ship_Velocity_Ball_Master` | `velocity_ball_master.png` | `target` | **PLATINUM `clipper_velocity_ball_master`** (frozen 2026-06-15) | PARITY PASS 2026-06-15 (owner-approved platinum): inherits the g-force structural fixes (centring overflow-gate, `shrinkProportion=0` square ball, full-ellipse cubic circle dot) — centred square cross, circular centre dot, four chevron caps symmetric at the axis ends, matches the reference. Earlier history: aspect 1.0 (square) fixed 2026-06-14. V1 spokes (`base_Diagram` cross_line `Accent1` op 85/50) + chevrons (`base_Cap*` cross_cap `Critical` op 100/70) cyan→**orange** LANDED 2026-06-15: these HUD glyph SVGs are non-uniform colorstyle (mixed opacity / two roles), which `parse_uniform_colorstyle` rejects → `bb_svg::recolour_colorstyle_svg` recolours each path to its OWN id-encoded role colour at its OWN opacity (generic per-path; uniform SVGs stay on the single-overlay path so the frozen g-force bytes are untouched). Both DRAK roles resolve to the same orange (ref `(213,80,60)` bloomed ≈ render `(193,52,43)` crisp). V2 at-rest cream "L" vector (`base_CurrentLine*` Bright / `base_InputLine*` Base, `SizeY=|flightcontroller/linearvelocity/ratio/z|/2`) COLLAPSE LANDED 2026-06-15: `resolve_geometry_fields_into_scene`'s `value<=0 → keep authored` guard discarded the genuine at-rest 0; now `field_value_source_is_engine_variable` (path-sensitive: follows the taken boolean branch, treats div-by-zero as data-absent) lets an engine-`Variable`-grounded 0 collapse the bar, while the power-pip `1/MaxPipList` + widget-standard unwired `ParamInput` sizes stay placeholders. V4 cap-arrow rotation+positioning LANDED 2026-06-15: node `rotation_deg` (orientation.z+orientationOffset.z) captured in `ui_ir`, applied in `ir_compose::apply_node_rotation` (rotate asset around pivot, alpha-weighted bilinear) — left/right caps author z=90; AND `flip_adjusted_contain_position` pre-inverts contain pos on flipped axes so the whole-box flip lands content at its authored end (bottom/left caps were ~200px short). All 4 caps symmetric at the diagram edges. V5 green background fit+centre LANDED 2026-06-15: `bb_layout::cover_fit_full_bleed_to_viewport` snaps the full-canvas `WidgetImage` background to the visible viewport (was stretched to the 16:9 canvas + bright centre off-screen). V2b/C6 white centre dot LANDED 2026-06-15 (NOT deferred — owner directed): `is_untinted_overlay_render_shape` (background.enable+null AND svgFill.renderShape+overlay+null+empty-path) on a LEAF → white `background_fill_colour` (colour-overlay identity; zero frozen-target matches) + `fill_rounded_rect_ts_with_mode` renders corner_radius fills rounded (dot is a circle). All five fixes pass ui_check + visual guard. Velocity ball now matches the reference structurally |
| Countermeasures | `Countermeasures_Screen` / LOD0 cockpit | `HC_HUD_Ship_Countermeasures_Master` | `countermeasures_master.png` | — | — | aspect 1.0 (square) fixed 2026-06-14 |
| Compass | `Screen_Central_Compass` / LOD0 cockpit | `HC_HUD_Ship_Compass_Master` | `compass_master.png` | — | — | aspect 3.27 fixed 2026-06-14 |
| Radar | `Screen_Radar_RTT` / LOD0 cockpit | `MapDisplayMaster` | `Screen_Radar_RTT.png` (+ `mapdisplaymaster.png`) | — | — | aspect 1.23 fixed 2026-06-14 (was 1024² square); curved-chord ~5% under ref 1.29 |
| Self MFD | `Screen_Left_Upper_RTT` / LOD0 cockpit | `MC_S_Self_Master` | `self_master.png` | — | — | mfd 4:3 frame path (unchanged) |
| LR-indicator | `Screen_Left_Upper_RTT_Small` / LOD0 cockpit | `HC_HUD_Ship_LRInd_Master` | `lrind_master.png` | — | — | aspect 1.56 fixed 2026-06-14 |
| Velocity num | `screen_flight_hud_left_upper` / LOD0 cockpit | `HC_HUD_Ship_Velocity_Num_Master` | `ship_velocity_num_master.png` | — | **GOLD `clipper_velocity_num_master`** (frozen 2026-06-15) | PARITY PASS 2026-06-15 (DRAK cutlass variant, two readouts stacked centred: "0m/s" / "0.0 G"). Was 100% BLANK at rest. LANDED: (1) at-rest content — both engine `NumberVariable`s (`flightController.forwardVelocity`, `flightcontroller.totalgforceamount`, +`ForwardBackSpeedGoal`) absent in static replay → placeholders ('-', 'G'); pinned =0 in the default-value registry (`1f25845d2`, AccelerationLimiterRatio precedent) → velocity "0", g-force "0.0 G" ✓. (2) layout — `base_CurrentVelocityNumber` is a flex Column(axisJustification/cross/item=Center) whose two `WidgetCard`s author non-zero Auto (64>1.0) on both axes; the flex sizer had no content-fit path for value>1.0 text-backed column children → they filled the canvas, overlapped, dumped glyphs top-left. Added main+cross-axis content-fit via `auto_text_intrinsic_main`, gated `axisJustification==Center` (no frozen screen has it — medical pins fill for non-zero column Auto, workflow §10) → cards shrink-wrap + Center stacks/centres them (`0cdf6d526`, visual+snapshot guards GREEN after `--lod 0` export). Colour Bright≈white faithful. (3) **velocity "m/s" unit LANDED** (`131c2f8b8`): `LocalizedSIUnitFromNumber` now renders `unitSuffix` — the SI unit symbol is the systematic localization key `text_ui_SIUnit_<suffix>` (`Speed`→"m/s", from global.ini at runtime, NO hard-coded string), and `forcedSIPrefix="Unit"` pins the base unit (no K/M/G); `unitSuffix="None"` adds nothing (g-force "G" comes from the later combine). Render now "0m/s" / "0.0 G". Frozen target master ALSO uses SIUnit Speed/Distance but its bindings are absent at rest (no target) → None → unchanged (visual+snapshot GREEN). **FONT ~9× LANDED 2026-06-15** (`6c1343abf`): the prior "52px Heading2, undecoded per-screen scale" diagnosis was WRONG. The DRAK velocity SCREEN variant (`drak_hc_hud_cutlass_velocity_num`, the `(Screen)` tag-79f1bb24 variant the Clipper instantiates) authors FontSize **500** ("0m/s") / **420** ("0.0 G") in BOTH its `defaultStyles` (FontSize + white StrokeColor) AND a duplicate `s_grey_hud` brandStyles block (FillColor Base). A drak ship matches NO brandStyles entry (only `s_grey_hud` declared → grey-HUD ships), so per the engine "no brand match → defaultStyles used at every level" rule the sizing comes from **defaultStyles**, applied at the BRAND tier as a no-brand-match fallback (`apply_canvas_style_cascade`) so the textfield text-format route reaches the readouts. The route fix: a WidgetTextField's implicit text-format CHILD is a `Text` node, so `Type(Text)` matches it INSIDE a `Parent(...)`-wrapped entry (velocity's `Type(Text)+Parent[(Not)Tag(fontnumber)]`); a bare/`Ancestor`-wrapped `Type(Text)` stays widget-route (MFD footer screen-name SAFE — it's `Ancestor`-wrapped). Render 20.9%/16.2% cap-h vs ref 20.4%/16.2% (size_ratio 1.01), centred. **COLOUR faithful**: text keeps its natural `Bright` (244,244,218 ≈ ref 232,236,218) — applying the `s_grey_hud` BRAND instead (the FALSIFIED first approach) forced FillColor Base → drak orange; defaultStyles has no FillColor so the natural white survives. **BOXES fixed** (`c7931fb4b`): defaultStyles' white StrokeColor landed on the node `stroke_colour` and `ir_compose` drew it as a box around each field — a WidgetTextField's StrokeColor is a GLYPH outline, not a box (`node_draws_rect_stroke` now excludes `widget_text_field`). **(c) background = PROVEN capture characteristic**: render faithfully reproduces the `DRAK_Background_indicators` dark-centre/bright-edge vignette; the ref's brighter uniform rounded panel + dark bezel is the physical emissive screen + rounded screen-mesh shape (same bound the g-force/velocity-ball platinum passes accepted); no brand-driven brightening exists for the drak no-brand-match path. **TRAP STILL VALID: broadening `ir_compose` `center_anchored_heading` (Heading1→any label_style) REGRESSES medical-bed title/desc — centre via card cross-fit, NOT the text-draw rule.** ui_check --full GREEN, no frozen-screen drift. ONBOARDED **GOLD `clipper_velocity_num_master`** 2026-06-15 (owner-approved, `a91cec377`; manifest 9→10; artifact from `--lod 0`). aspect 1.24 fixed 2026-06-14 |
| Master-mode display | `screen_flight_hud_right_upper` / LOD0 cockpit | `HC_HUD_Ship_Master_Mode_Display_Master` | `master_mode_display_master.png` | — | — | aspect 1.24 fixed 2026-06-14 |
| Velocity / afterburner bars | `screen_flight_hud_left` / `_right` / LOD0 cockpit | `HC_HUD_Ship_Velocity_Bar_Master` / `…_Afterburner_Bar_Master` | (no straight-on capture yet) | — | — | aspect 2.99 fixed 2026-06-14 |

Per-screen render aspect (commit `cc67d79e2`, 2026-06-14): physical/radar
screens are sized to their cockpit screen-mesh aspect (the `RTT_Screen`
faces), so the square gauges, compass and annunciator no longer render
squashed. This needs the **LOD0** cockpit geometry (the small HUD screens
are culled in LOD1); the freeze scripts export `--lod 0`.

Scene split: **LOD0** (`DRAK Clipper_LOD0_TEX0/scene.json`) carries the whole
cockpit dashboard — the MFDs AND the HUD gauges + annunciator L/R (the small HUD
screens exist ONLY here; LOD1 culls them); **LOD1**
(`DRAK Clipper_LOD1_TEX2/scene.json`) carries the interior usables (medical,
door) — the font baseline and the medical/door frozen targets render at LOD1,
but the regression FREEZE exports `--lod 0` for every target (it reaches the
cockpit screens), so the dossier "scene" column is the *replay* LOD, not the
freeze source.

## 4. MCP tools (server `starbreakerMcp`)

Policy: MCP-first for data archaeology; CLI for renders/exports; LOCAL
PNGs/JSON are read directly with the Read tool (vision for images) — never
via MCP. Confirm data is loaded with `p4k_data_status`. **`canvas` arg = the
record GUID or name, NOT the dcb_canvas mirror file path** (a path returns
`canvas_not_found`); get the GUID from the mirror file's top-of-file
`_RecordId_` / `_RecordName_` (e.g. `BuildingBlocks_Canvas.GEN_MC_S_Emissions`).

**Style/IR investigation order** (run BEFORE editing style logic; if a
change has no effect in these, revert it):
1. `ui_canvas_style_inventory` — authored containers (embeddedStyles,
   defaultStyles, brandStyles[], inlineStyles) with condition/modifier
   summaries.
2. `ui_scene_style_probe` — resolved scene nodes with style tags, raw
   colour/tint fields, `__AppliedStyleEntries` (which entries actually
   matched).
3. `ui_ir_query` — compiled IR: computed/draw rects, text payloads/bounds,
   resolved tokens and colours (what the renderer consumes).

**Data tools:** `search_entities` (EntityClassDefinition by name),
`search_records` (all record types), `datacore_record` (full JSON by
GUID/name), `datacore_query` (property path, e.g.
`Components[SEntityComponentDefaultLoadoutParams]`), `entity_loadout`
(resolved tree), `p4k_list` / `p4k_read` (CryXML auto-decode) /
`p4k_search`, `image_preview` (vision on P4K DDS/PNG), `chunk_list` /
`chunk_read`, `ui_regression_registry` / `ui_regression_validate`.
Gotcha: canvas JSON says `.tif` → the P4K entry is `.dds`.

MCP server redeploy after changing it:
`pkill -f starbreaker-mcp || true && cargo build --release -p starbreaker-mcp
&& cp target/release/starbreaker-mcp mcp/starbreaker-mcp`, then restart the
client.

## 5. Data locations

| What | Where |
|---|---|
| Decompiled record mirror (grep-able authored canvases/styles/tags — the workhorse) | `~/projects/scorg_tools/ships/dcb_canvas/libs/foundry/records/` (ws); UI under `ui/buildingblocks/...` |
| Decomposed export | `~/projects/scorg_tools/ships/Packages/<Ship>_LODn_TEXm/scene.json` (ws) |
| Generated screen PNGs | `ships/Data/UI/Generated/ship/<mfr>/<Ship>/buildingblocks_canvas_*.png` (ws, refreshed by full export only) |
| Reference screenshots | `~/projects/scorg_tools/reference/in-game/Clipper/` (ws) |
| Regression manifest | `crates/starbreaker-ui/tests/fixtures/ui_ir/ui_snapshot_manifest.json` |
| IR freeze (baselines) | `crates/starbreaker-ui/tests/fixtures/ui_ir/ui_snapshot_freeze.json` (schema: `crates/starbreaker-ui/docs/ir-freeze-schema.md`) |
| Known outliers | `crates/starbreaker-ui/tests/fixtures/ui_ir/ui_known_outliers.json` |
| Measurement bank (settled reference numbers — consult FIRST, workflow §4) | `crates/starbreaker-ui/tests/fixtures/ui_ir/reference_measurements_v1.json` + `.notes.md` |
| Font baseline | `crates/starbreaker-ui/tests/fixtures/font_size_baseline.tsv` (`crates/starbreaker-ui/docs/ui-font-size-harness.md`) |
| Default-value registry + provenance | `crates/starbreaker-ui/data/default_value_registry_v1.json` + `.notes.md` |
| Ship-value derivation | `crates/starbreaker-3d/src/ui_pipeline/ship_values.rs` (+ `ship_values/tests.rs`) |
| Pipeline stages | see `crates/starbreaker-ui/docs/ui-workflow.md` §2 table; engine code in `crates/starbreaker-ui/src/<stage>/engine_parts/*.part` |
| Fallback register | `crates/starbreaker-ui/docs/ui-fallback-register.md` |

**Screen-mesh → render aspect** (the physical/radar screen-aspect mechanism,
`crates/starbreaker-3d/src/ui_pipeline/screen_aspect.rs`; landed `cc67d79e2`).
A render-to-texture UI screen is displayed on a mesh quad; its proportions ARE
the engine's render-target aspect, so the export derives each screen's aspect
from the geometry and sizes the `physical`/`radar` target to it (`mfd` keeps the
frame-canvas 4:3 path). Hard-won facts a fresh agent needs:

- **LOD0 is required.** Plain `entity export <ship>` defaults to **LOD1**, which
  CULLS the small cockpit HUD screens (g-force, velocity ball, countermeasures,
  …) — their aspect then resolves to `None` and they render 16:9. The cockpit
  UI lives at **LOD0**; export `--lod 0` (the freeze scripts already do). The
  `SB_SCREEN_ASPECT_PROBE` (§6) flags the empty-mesh/LOD case. Watch for STALE
  scene.json: the no-`--lod` export writes the `LOD1_TEX2` package, so the
  `LOD0_TEX0` scene.json + shared `Generated/*.png` can be stale.
- **Material id, not name.** The submesh `material_name` is EMPTY in the export;
  the UI render-target is identified by `material_id` → `MtlFile` name containing
  `RTT_Screen`/`RTT_Hud` (`RTT_Text_To_Decal` / `Glass_*_RTO` are excluded). The
  screen quad maps to its binding via `submesh.node_parent_index` →
  `nmc.nodes[i].name` == the binding `helper_name`.
- **PCA in-plane, not AABB.** Screens are TILTED in the cockpit, so an
  axis-aligned bbox collapses them (gave a false 1.96 for the 5.58 annunciator).
  Use principal-axis (PCA) extents of the `RTT_Screen` vertices. Curved screens
  (radar, the 81-vert MFDs) read the chord aspect (a few % low). Blender
  ground-truth (Clipper loaded): `pts=[mw@v.co]; U,S,Vt=svd(pts-mean);
  ext=sort((pts-mean)@Vt.T max-min)[::-1]; aspect=ext[0]/ext[1]` over the
  object's `RTT_Screen` faces.
- **The freeze sources from `--lod 0`** (`generate_ui_regression_artifacts.sh`),
  so it reaches the cockpit screens regardless of a dossier row's "scene"
  column (that column is the *replay* scene). Re-freezing one cockpit screen can
  surface others changed at LOD0 — inspect the artifact dims before freezing.

## 6. Probe registry

Env-gated diagnostics (zero cost unless set). Add new probes to this table
in the same commit that introduces them.

| Probe | Owner | Channel | Prints |
|---|---|---|---|
| `BB_A3_STYLE_PROBE=1` | `bb_brand_apply` | stderr (`eprintln`) | per node, per cascade pass: name, style tags, matched entry names + their key modifiers (`Name[BackgroundColor=Base]`, `[IsActive=false]`, `[SizeY=0.5]`) |
| `SB_UI_STYLE_PROVENANCE=1` | `bb_brand_apply` | IR field (with `--dump-ir-dir`) | adds `UiIrNode.style_provenance` = `{field: "pass/entry"}` recording which cascade entry WON each colour / `IsActive` / `Size*` / `Anchor*` field — only modifiers actually applied, so a gate-suppressed override is NOT credited (query: `ui_ir_query.py … --fields style_provenance`). None in normal compiles, so freezes/hashes are unaffected |
| `BB_A3_TEXT_PROBE=1` | `bb_bindings` (LocalizedFromBoolean) | log (`log::info` — needs `RUST_LOG=info`) | text branch selection + resolved values |
| `BB_SHRINK_PROBE=1` | `bb_layout` flex shrink | stderr (`eprintln`) | shrink scale + each child's name/type/main-axis sizing |
| `SB_UI_GEOM_PROBE=1` | `bb_bindings::resolve_geometry_fields_into_scene` | stderr (`eprintln`) | bound SizeX/SizeY input chains + resolved values per node |
| `SB_SHIP_VALUES_DUMP=1` | `starbreaker-3d` `ship_values` | stderr (`eprintln`) | every derived registry path = value at export/replay |
| `SB_UI_FONT_DUMP=1` | `text/swf_draw` | stderr (`eprintln`) | one `FONTDUMP` line per rendered text element (see harness doc) |
| `BB_TEXT_FORMAT_PROBE=1` | `bb_brand_apply` | stderr (`eprintln`) | per pass: `TFPROBE` = text-format-route entry applications (FontSize/FillColor on tagged textfields); `TFPROBE-NORMAL` = normal-route entries carrying FontSize (with modifiers + conditions) |
| `BB_DRAW_RECT_PROBE=<1\|filter>` | `ir_compose` custom-shape draw | stderr (`eprintln`) | per asset draw: node name, laid-out `rect`, actual `raster` WxH, asset path (`1` = all; else a name/asset substring filter). For element width from layout, not dim pixels (ledger 45) |
| `SB_SCREEN_ASPECT_PROBE=1` | `starbreaker-3d` `child_payload` (per-screen aspect populate) | stderr (`eprintln`) | `SCREEN_ASPECT helper=… kind=… mesh_verts=… aspect=…` per UI binding. Diagnoses the per-screen render aspect (see §5 *screen-mesh → render aspect*): `aspect=None` + `mesh_verts=0` = empty mesh (WRONG LOD — small HUD screens are culled in LOD1, re-export `--lod 0`); `None` + non-zero verts = no `RTT_Screen` submesh on the helper node. Runs on `entity export` (the populate step) |
| `ui render --dump-ir-dir <dir>` | CLI flag | files (`*.ir.json`) | composed `*.ir.json` per helper (nodes, rects, payloads, tints). FAST IR inspection of a bound screen (~15s, matches the render) — prefer over `mfd_ir_dump` |

Example: `BB_SHRINK_PROBE=1 ./target/debug/starbreaker ui render --scene
"<scene>" --out-dir /tmp/x --helper Screen_Left_Lower_RTT 2>&1 | grep PROBE`.

## 7. Diagnostics (`cargo run -p starbreaker-ui --example <name> --`, plus scripts)

| Example | Use |
|---|---|
| `python3 scripts/ui_ir_query.py query <ir.json> <regex> [--fields a.b,c]` | list IR nodes whose name or text matches the regex: id, parent, type, rect, is_active + dotted-path extras (input: `ui render --dump-ir-dir` output) |
| `python3 scripts/ui_ir_query.py tree <ir.json> <node_id>` | ancestor chain for one node with rect, authored_size, anchor/pivot, padding, margin |
| `python3 scripts/ui_ir_query.py children <ir.json> <node_id> [--depth N] [--fields a.b,c]` | descendant subtree (rect, `right`=x+w, is_active, non-Visible overflow) — the mirror of `tree`, for clip/overflow tracing |
| `python3 scripts/ui_measure.py <image> --box x0,y0,x1,y1 [--ir <ir.json> --node <id>] [--delta N] [--anchor … --anchor-rgb …]` | glyph-run cap heights (contamination-flagged) + colour ratios with `feature_width` (warns when ≤4px that a thin feature on a RECTIFIED capture has a smeared hue — measure colour on the ORIGINAL) + optional additive-haze correction (JSON to stdout) |
| `python3 scripts/ui_measure.py --text-bands <image> [--ref <reference>]` | text-SCREEN mode (no box): bright-text bbox + `centre_x_frac` (is it centred?) + per-line cap-height bands as % of image height; with `--ref` adds `size_ratio_render_over_ref` (the resolution-independent font-scale gap — velocity-num measured 0.11 = render ~9× too small). For diagnosing blank/mispositioned/mis-sized text readouts |
| `python3 scripts/ui_gauge_measure.py <render> [reference] [--montage out.png]` | circular HUD-gauge geometry (g-force/velocity ball, countermeasures, radar): centre-dot offset + circularity (circle vs squircle), cross-arm V/H symmetry, per-cardinal ring perp-offset + radius fraction (JSON), and a centre-aligned render\|reference montage. Use abs paths — a relative `ships/…` resolves to the STALE `StarBreaker/ships/` copy. Caveat: a cardinal window can catch an adjacent diagonal marker (the cross V/H band metric is robust) |
| `ui_stage_diff <canvas.json> [WxH] [--records-root <dir>] [--filter <substr>]` | parse-only vs full-resolve layout diff; flags first name-matched divergence (cracks "which stage broke the geometry") |
| `mfd_ir_dump <canvas-guid> <content-guid> [name-filter] [WxH]` | framed MFD IR dump from the LOCAL record mirror — the no-P4K/no-MCP fallback (~5s: indexes only the UI subtrees + caches the parsed TagDatabase; prints index/compile timing). When P4K/MCP is available prefer `ui_ir_query` (canvas pair) or `ui render --dump-ir-dir` (bound screen). NOTE: uses anim sample 0; `ui render` uses 50 (the render), so a few state-dependent rects differ |
| `query_ui_layout --canvas-guid <guid> --query <pattern>` | per-node layout/draw/text rects + drawn glyph bounds |
| `bb_layout_wireframe <fixture.json> <out.png> [--merge]` | wireframe overlay of layout rects |
| `phase5_certification_dashboard` | representative-family certification table (CI) |
| `freeze_ui_snapshot_ir` | driven by `scripts/freeze_ui_snapshot_ir.sh` |

## 8. Glossary

- **platinum / gold**: regression tiers (pixel-diff budget 0.5% / 1%;
  structural thresholds tighter for platinum). Targets live in the
  manifest; baselines in the IR freeze.
- **known-outlier**: reference-anchored one-sided override for a knowingly
  off element (workflow §6).
- **derivation vs pin**: per-ship value computed from DataCore at export vs
  an at-rest engine-pushed value recorded in the registry with provenance.
- **replay**: `ui render --scene` re-render of an existing export — same
  pipeline + derived values, ~6× faster than exporting.
- **dcb_canvas mirror**: local decompiled DataCore record tree (grep-able
  authored JSON).
- **widget-standard expansion**: bb_resolve instantiation of component
  templates (buttons, icons, scrollbars) from `*componentstandard` records.
- **param relay / slot broadcast**: parent→child ComponentParameter wiring;
  a slot name may have MULTIPLE live defs (all get injected) and plain
  un-namespaced canvas hops relay onward.
- **host-stage scale**: MFD frame path multiplies FontSize by
  max(target/stage) of `BuildingBlocks_root.swf` (1280×720 → ×1.667 at
  1600×1200); geometry does NOT scale.
- **BB_ColorStyle slot order**: authoritative enum index (Base=0 …
  Bright=6, Selected=7, Disabled=8…) — see `crates/starbreaker-ui/docs/ui-architecture-runbook.md`.
- **LOD0 / LOD1 scene**: cockpit MFDs vs interior usables (dossier §3).
