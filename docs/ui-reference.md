# UI parity reference — commands, tools, data

The lookup half of the UI workflow (`docs/ui-workflow.md` is the process).
Every command here was executed during writing (2026-06-11). Paths marked
(ws) are workspace-specific (this machine); the repo-relative ones are
universal.

## 1. Build & test

```bash
cargo build                      # debug — fast, fine for ALL iteration incl. renders
cargo build --release -p starbreaker   # only for deploy / canonical exports

bash scripts/ui_check.sh         # TDD tier: ui lib + live IR guard + line cap
bash scripts/ui_check.sh --full  # boundary tier: + snapshot/visual suites,
                                 #   freeze+artifact validators, 3d lib, font harness
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
  [--scene <scene.json>] [--out <dir>]   # default scene: LOD1 Clipper
```
Raw form (when the wrapper's defaults don't fit):
```bash
./target/debug/starbreaker ui render \
  --scene "/home/tom/projects/scorg_tools/ships/Packages/DRAK Clipper_LOD0_TEX0/scene.json" \
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

**Full export (canonical PNGs under `ships/Data/UI/Generated/...`, ~6 min):**
```bash
./target/release/starbreaker entity export drak_clipper \
  /home/tom/projects/scorg_tools/ships --kind decomposed
```
Required before artifact freezes. The generated PNGs are written near the
END of the run — never diff/freeze until the process exits.

**Freeze tooling** (flows in `docs/ui-workflow.md` §7). The whole artifact
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

## 3. Comparison & screen dossier

```bash
python3 scripts/ui_compare.py <render.png> <reference.png> \
  --regions <preset> --out-dir /tmp/ui_compare [--stats]
python3 scripts/ui_compare.py --regions list   # available presets
```
`--stats` prints per-region bright/dark pixel means + R-normalised ratios
for both images — the photometric review method. Judge HUE from the ratios,
never raw values: every reference capture has its own cast (G/B attenuate)
and bloom lifts B near bright elements. Before judging an unknown colour,
measure a known anchor on the SAME capture (footer text = Base, pip slabs =
Bright); a colour matches a palette slot when the RATIOS line up under that
cast. This method identified the linear-light compositing gap and the
MissionObjectives icon slot.
The reference is auto-scaled to the render width before cropping; READ the
emitted `cmp_*.png` files with vision. Reference screenshots are imperfect
(skew/bloom/capture artifacts; power-screen pip outlines are mouse-hover
artifacts) — compare structurally.

**Screen dossier** — one row per known screen (extend as screens are
worked). References live in
`/home/tom/projects/scorg_tools/reference/in-game/Clipper/` (ws); generated
PNGs in `ships/Data/UI/Generated/ship/drak/Clipper/` named
`buildingblocks_canvas_<canvas>.png`; scenes in
`/home/tom/projects/scorg_tools/ships/Packages/` (ws).

| Screen | Helper / scene | Canvas | Reference image | Preset | Tier / target id | Open issues |
|---|---|---|---|---|---|---|
| Power MFD | `Screen_Left_Lower_RTT` / LOD0 scene | `MC_S_Power_Master` (via `M_MFD_Screen` frame) | `Screen_Left_Lower_RTT.png` | `power` | not frozen (arc in progress) | `docs/ui-clipper-parity-handoff.md` |
| Target MFD | `Screen_Right_Upper_RTT` / LOD0 scene | `MC_S_Target_Master` | `Screen_Right_Upper_RTT.png` | `target` | GOLD `clipper_target_master` | A7 backdrop stack remainder (handoff) |
| Medical bed | usable screen / LOD1 scene | `I_Med_MedicalBed_A` | `screen_16x9_a-[medical1].png` | — (add) | PLATINUM `ui_target_a` | handoff steps 10–13 |
| Medical end-of-bed | usable screen / LOD1 scene | `I_Med_MedicalEndOfBed_A` | `mesh_end_screen_plane-[medical2].png` | — (add) | PLATINUM `ui_target_b` | logo −12px check (handoff) |
| Small door | usable screen / LOD1 scene | `I_Door_Small_DRAK` | `Door-closed.png` | — (add) | GOLD `clipper_small_door` | — |
| Annunciator L | `Screen_Annunciator_L` / LOD1 scene | `H_Eng_Annunciator_Master_Left` | `Screen_Annunciator_L.png` | — (add) | GOLD `eng_annunciator_master_left` | — |
| (unmapped) | `Screen_Left_Upper_RTT`, `Screen_Radar_RTT`, compass, flight HUDs, radars… | — | partial references exist | — | — | map when first worked |

Scene split: **LOD0** (`DRAK Clipper_LOD0_TEX0/scene.json`) carries the
cockpit MFD screens; **LOD1** (`DRAK Clipper_LOD1_TEX2/scene.json`) carries
the interior usables (medical, door, annunciator…) — the font baseline and
the four frozen interior targets need LOD1.

## 4. MCP tools (server `starbreakerMcp`)

Policy: MCP-first for data archaeology; CLI for renders/exports; LOCAL
PNGs/JSON are read directly with the Read tool (vision for images) — never
via MCP. Confirm data is loaded with `p4k_data_status`.

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
| Decompiled record mirror (grep-able authored canvases/styles/tags — the workhorse) | `/home/tom/projects/scorg_tools/ships/dcb_canvas/libs/foundry/records/` (ws); UI under `ui/buildingblocks/...` |
| Decomposed export | `/home/tom/projects/scorg_tools/ships/Packages/<Ship>_LODn_TEXm/scene.json` (ws) |
| Generated screen PNGs | `ships/Data/UI/Generated/ship/<mfr>/<Ship>/buildingblocks_canvas_*.png` (ws, refreshed by full export only) |
| Reference screenshots | `/home/tom/projects/scorg_tools/reference/in-game/Clipper/` (ws) |
| Regression manifest | `crates/starbreaker-ui/tests/fixtures/ui_ir/ui_snapshot_manifest.json` |
| IR freeze (baselines) | `crates/starbreaker-ui/tests/fixtures/ui_ir/ui_snapshot_freeze.json` (schema: `crates/starbreaker-ui/docs/ir-freeze-schema.md`) |
| Known outliers | `crates/starbreaker-ui/tests/fixtures/ui_ir/ui_known_outliers.json` |
| Font baseline | `crates/starbreaker-ui/tests/fixtures/font_size_baseline.tsv` (`docs/ui-font-size-harness.md`) |
| Default-value registry + provenance | `crates/starbreaker-ui/data/default_value_registry_v1.json` + `.notes.md` |
| Ship-value derivation | `crates/starbreaker-3d/src/ui_pipeline/ship_values.rs` (+ `ship_values/tests.rs`) |
| Pipeline stages | see `docs/ui-workflow.md` §2 table; engine code in `crates/starbreaker-ui/src/<stage>/engine_parts/*.part` |
| Fallback register | `docs/ui-fallback-register.md` |

## 6. Probe registry

Env-gated diagnostics (zero cost unless set). Add new probes to this table
in the same commit that introduces them.

| Probe | Owner | Prints |
|---|---|---|
| `BB_A3_STYLE_PROBE=1` | `bb_brand_apply` | per node, per cascade pass: name, style tags, matched entry names |
| `BB_A3_TEXT_PROBE=1` | `bb_bindings` (LocalizedFromBoolean) | text branch selection + resolved values |
| `BB_SHRINK_PROBE=1` | `bb_layout` flex shrink | shrink scale + each child's name/type/main-axis sizing |
| `SB_UI_GEOM_PROBE=1` | `bb_bindings::resolve_geometry_fields_into_scene` | bound SizeX/SizeY input chains + resolved values per node |
| `SB_SHIP_VALUES_DUMP=1` | `starbreaker-3d` `ship_values` | every derived registry path = value at export/replay |
| `SB_UI_FONT_DUMP=1` | `text/swf_draw` | one `FONTDUMP` line per rendered text element (see harness doc) |
| `ui render --dump-ir-dir <dir>` | CLI flag | composed `*.ir.json` per helper (nodes, rects, payloads, tints) |

Example: `BB_SHRINK_PROBE=1 ./target/debug/starbreaker ui render --scene
"<scene>" --out-dir /tmp/x --helper Screen_Left_Lower_RTT 2>&1 | grep PROBE`.

## 7. Diagnostic examples (`cargo run -p starbreaker-ui --example <name> --`)

| Example | Use |
|---|---|
| `ui_stage_diff <canvas.json> [WxH] [--records-root <dir>] [--filter <substr>]` | parse-only vs full-resolve layout diff; flags first name-matched divergence (cracks "which stage broke the geometry") |
| `mfd_ir_dump <canvas-guid> <content-guid> [name-filter] [WxH]` | framed MFD IR dump from the record mirror (filter = lowercase name substring) |
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
  Bright=6, Selected=7, Disabled=8…) — see `docs/ui-architecture-runbook.md`.
- **LOD0 / LOD1 scene**: cockpit MFDs vs interior usables (dossier §3).
