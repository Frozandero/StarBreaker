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
                                 #   guards run authoritatively, freeze+artifact
                                 #   validators, 3d lib, font harness
cargo test --workspace           # everything (slow)
```

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
from the helper — cockpit MFD `*_RTT` screens (e.g. `Screen_Left_Lower_RTT`) use
LOD0, interior usables (medical/door/annunciator) LOD1. So the power screen no
longer needs the long `--scene` path: `ui_render.sh --helper Screen_Left_Lower_RTT --ir`.
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
| Small door | usable screen / LOD1 scene | `I_Door_Small_DRAK` | `Door-closed.png` | `door` | GOLD `clipper_small_door` | — |
| Annunciator L | `Screen_Annunciator_L` / LOD1 scene | `H_Eng_Annunciator_Master_Left` | `Screen_Annunciator_L.png` | `annunciator` | GOLD `eng_annunciator_master_left` | — |
| (unmapped) | `Screen_Left_Upper_RTT`, `Screen_Radar_RTT`, g-force/radar squares, `velocity_ball`, compass, flight HUDs… | — | partial references exist | — | — | map when first worked. Square/non-4:3 screens render WRONG aspect (4:3) — per-screen aspect step-3 hand-off: `crates/starbreaker-ui/docs/ui-mfd-square-aspect-handoff.md` |

Scene split: **LOD0** (`DRAK Clipper_LOD0_TEX0/scene.json`) carries the
cockpit MFD screens; **LOD1** (`DRAK Clipper_LOD1_TEX2/scene.json`) carries
the interior usables (medical, door, annunciator…) — the font baseline and
the four frozen interior targets need LOD1.

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
