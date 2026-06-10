# UI Architecture and Troubleshooting Runbook

## Architecture Summary

The UI pipeline is split into four stages:

1. Source resolution
- Resolve BuildingBlocks canvases, styles, bindings, and localization.
- Files: `bb_resolve.rs`, `bb_state_filter.rs`, `bb_bindings.rs`, `bb_brand_apply.rs`.

2. Canonical IR compilation
- Compile deterministic `UiIrDocument` output with fidelity fields and provenance.
- File: `ui_ir.rs`.

3. Renderer consumption
- Render from IR only (no source-data probing in renderer).
- Files: `ir_compose.rs`, `hybrid_compose.rs`, `compose.rs` compatibility wrapper.

4. Regression/certification
- Structural snapshot extraction and comparison for representative families.
- File: `ui_snapshot.rs` and example `phase5_certification_dashboard.rs`.

## Reference: BB_ColorStyle colour roles

BuildingBlocks colour tokens (`BuildingBlocks_ColorStyle.color`, e.g. `Base`,
`Bright`, `Accent1`) are the `BB_ColorStyle` DataCore enum. The enum's integer
value is the **direct index into a brand style's `colorStyles` palette array**.
Authoritative order, dumped from `Data\Game2.dcb` via
`starbreaker_datacore::database::Database::{enum_defs, enum_options, resolve_string2}`:

| idx | role | idx | role | idx | role |
|----:|------|----:|------|----:|------|
| 0 | Base | 6 | Bright | 12 | ContactPositiveRep |
| 1 | Positive | 7 | Selected | 13 | ContactNegativeRep |
| 2 | Moderate | 8 | Disabled | 14 | ContactAgressive |
| 3 | Critical | 9 | Background | 15 | ContactUnknown |
| 4 | Accent1 | 10 | ContactNeutral | 16 | MissionObjectives |
| 5 | Accent2 | 11 | ContactParty | | |

Key facts:

- **`Bright` (6) is a muted role, distinct from `Base` (0).** Example: a
  `ComponentLabelCaptionPair` label uses `Heading3`→`Base` while its caption
  value uses `Heading6`→`Bright` (e.g. medical "MEDGELS" light blue label over a
  light-grey "200/200" value). Named text-style colours live in the standard
  textfield widget's per-brand `brandStyles[].entries[].modifiers[FillColor]`
  (`textfieldwidgetstandard.json`), parsed in `ui_ir` (`StandardTextStyle`,
  `brand_text_style_colour_token`).
- **The renderer's token→slot resolvers are role-aware, not pure enum-indexing.**
  `bb_brand_apply/colors.rs::color_style_slot_index` and
  `ir_compose` `resolve_colour_token` / `resolve_surface_colour_token` map the
  same token to different slots depending on whether it paints a *foreground*
  element (icon/text/glyph) or a *surface* (filled shape / chrome). The clearest
  case is `Accent1`: foreground → light slot 0, surface → darker slot 4 (the
  medical fingerprint). When extending the mapping, verify against an in-game
  reference per role; do not assume `slot == enum index`.

## Standard Validation Commands

- Full UI crate checks:
  - `cargo test -p starbreaker-ui`
- Hardcoding guard:
  - `bash .github/scripts/check_ui_hardcoding.sh`
- Family certification drift check:
  - `cargo run -p starbreaker-ui --example phase5_certification_dashboard`

## Fast Render Iteration: `ui render` replay

Do NOT run a full entity export to iterate on UI rendering. Once a
decomposed export exists, replay every UI binding through the SAME live
DataCore + P4K fetchers in about a minute:

```bash
target/release/starbreaker ui render \
  --scene "<workspace>/ships/Packages/DRAK Clipper_LOD0_TEX0/scene.json" \
  --out-dir /tmp/ui_replay
```

This renders all bindings (43 on the Clipper) as
`<helper_name>_TEX0.png` / `<canvas_guid>_TEX0.png`. A full export is only
needed when the binding set itself changed or to refresh the canonical
`ships/Data/UI/Generated/...` PNGs for the regression artifact freeze.

**Trap:** the full export writes its Generated PNGs near the END of the
run. Diffing a PNG right after its mtime first changes can read the
PREVIOUS export's bytes. Wait for export completion before diffing.

For IR-structure questions (node rects, tags, text payloads) use the MCP
`ui_ir_query` tool — but remember the deployed MCP binary may predate
your working-tree changes; treat its layout numbers as stale and only
trust its structure/names, or rebuild+redeploy the MCP first.

## Reference: MFD frame geometry model

A ship MFD render (binding_kind `mfd`) composes through the hosting GFx
stage of `BuildingBlocks_root.swf` (1280×720). Established, verified
constants (all measured against Clipper in-game captures and
cross-checked between two screens):

- **Stage → render-target scale** is per-axis: 1600×1200 target →
  (×1.25, ×1.6667). Text pixel size = `FontSize × max(sx, sy)`
  (`pipeline/host_stage.rs`); geometry does NOT use this scale.
- **The bound content view is hosted in a stage sub-rect** inset 44
  stage-px from the left/right/bottom edges, flush top (runtime
  ActionScript behaviour of `BuildingBlocksView`, not authored in any
  record; constant in `mfd_view.rs`, applied by `apply_bound_mfd_view`
  sizing the landscape slot to Percent(1192/1280, 676/720), anchor/pivot
  (0.5, 0)). Frame chrome (header/footer) outside the slot stays
  full-bleed.
- The frame canvas chain is `M_MFD_Screen` (800×600, full-bleed slot) →
  `m_eng_mfdcontent` (content + 0.11-height footer overlay) → bound
  screen canvas (1920×1080 or 1600×900 content).

## Reference: at-rest binding semantics (static renders)

A static render substitutes for live engine data with these rules
(verified against in-game captures):

- An **unbound variable holds its type default** (bool false, int 0).
  `BindingsIntegerFromBoolean` etc. evaluate accordingly
  (`bb_bindings/eval_numeric.rs`).
- A boolean component parameter **wired to an unbound variable** takes
  the type default `false`, NOT the authored editor `defaultValue`
  (`bb_bindings/param_overrides.rs`). Unwired parameters keep the
  authored default. Integers are deliberately excluded — the footer's
  `bindingid == selectedmfd` unbound-sentinel guard
  (`is_unbound_integer_param`) depends on unresolved integer params.
- A boolean component parameter **named after an authored
  `staticVariables[]` entry** takes that variable's authored value
  (case-insensitive; `bb_state_filter/eval.rs`). Canvases author their
  at-rest state this way (e.g. the power screen's
  `engineeringoverride`/`PresetNotification = false` while the editor
  defaults are `true`).
- **Do NOT add a generic merged-scene IsActive-binding pass.** It was
  tried and reverted: reference captures are LIVE states, and e.g. the
  medical screens' session elements are IsActive-gated chains that are
  false at static rest but true in the captured state. Visibility
  gating lives in `bb_state_filter`'s capture-tested heuristics
  (`Instantiated`/`IsActive`/`Visible`/`Enabled` fields).

## Reference: widget-standard template expansion

`WidgetIcon` and `ComponentGeneralButtonSecondary` hosts expand the
engine's standard template canvases in `bb_resolve`
(`engine_parts/widget_standard_expansion.part`): synthetic component
params from the host's authored properties, implicit framework tags
("icon", "general-button-secondary") resolved from the tag database by
name, host icon identity forwarded onto the template's icon instance,
and the host's parse-time icon fallback cleared. Template nodes are
allocated in a **reserved node-ID band (`0xF000_0000+`)** so injecting
them at any depth never renumbers sibling canvas nodes (platinum
snapshots key elements by node ID). sk-brand chrome (`sk_<brand>_
buttonsecondarystyles`) supplies padding/borders/corner radii; a padded
parent's content box caps fixed-size children in layout (overlay and
flex no-grow paths).

## Reference-capture measurement methodology

- Anchor the capture scale on **frame footer arrows** (x) and measure x
  and y mappings INDEPENDENTLY — do not assume the content maps
  isotropically or full-bleed.
- Prefer near-isotropic captures (check width/height against the
  physical 4:3 screen). The Clipper power capture (1468×1109) is
  trustworthy; the right-upper target capture (1959×1513) is ~4%
  y-ambiguous — use within-content ratios there.
- Derive content mappings from at least two authored-rect features
  (e.g. card box tops), then cross-check the second capture before
  concluding.

## Troubleshooting Flow

1. Confirm source provenance first
- Check selected style/SWF source and unresolved references in diagnostics output.

2. Reproduce with deterministic fixture path
- Use representative fixture canvases under `crates/starbreaker-ui/tests/fixtures/canvas/`.

3. Compare structural snapshots
- Run certification dashboard and inspect failures in:
  - `docs/StarBreaker/ui-rework-artifacts/phase-5/certification-dashboard.md`
  - `docs/StarBreaker/ui-rework-artifacts/phase-5/certification-results.json`

4. Classify fault domain
- Source resolution mismatch: investigate `bb_resolve`/bindings/style application.
- IR mismatch: investigate `ui_ir` compilation fields.
- Rendering mismatch: investigate `ir_compose` use of existing IR fields.

5. Add a regression test with the fix
- Add/update a focused unit/integration test in the touched subsystem.
- Re-run `cargo test -p starbreaker-ui`.

## Incident Checklist

- Is this a source-data issue or renderer misuse?
- Is the behavior represented in IR (`UiIrDocument`) correctly?
- Did certification snapshots drift for previously certified screens?
- Is there an undocumented fallback involved?
- Did the fix add a focused regression test?
