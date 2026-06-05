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
