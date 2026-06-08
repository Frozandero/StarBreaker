# UI Fallback Register

This register tracks active UI fallbacks in `starbreaker-ui` and explicitly records owner, scope, trigger signal, and retirement target.

## Active Fallbacks

| Component | Fallback | Owner | Scope | Trigger Signal | Retirement Target |
| --- | --- | --- | --- | --- | --- |
| `crates/starbreaker-ui/src/bb_atlas.rs` | Manufacturer/gen atlas fallback paths | UI pipeline | Asset resolution when exact texture path is missing | `fallback_counter` in pipeline diagnostics and unresolved asset refs | Replace with authoritative source path resolution for all certified families in Phase 6B |
| `crates/starbreaker-ui/src/bb_loc.rs` | Ordinal localization parameter fallback | UI data resolution | Missing explicit named localization parameters | Localization fallback counters + unresolved localization warnings | Retire when localization map coverage proves complete on certification sets |
| `crates/starbreaker-ui/src/defaults.rs` | Versioned static default-value registry | UI data policy | Placeholder values for unresolved bindings/localization | `fallback_counter` diagnostics + unresolved binding warnings | Replace with authoritative runtime/default source extraction after fallback trigger rate is near-zero |
| `crates/starbreaker-ui/src/style.rs` | Drake amber style fallback | UI style selection | Missing/invalid style source resolution | Selected style provenance + fallback warnings | Retire once style selection is always sourced from canonical style data for certified families |
| `crates/starbreaker-ui/src/bb_layout.rs` | Unknown sizing behavior defaults to fill-parent | Layout resolver | Unknown `sizingBehavior` values | Layout warning telemetry and unusual rect diffs in certification output | Replace with explicit sizing behavior coverage + hard-fail in non-release checks |
| `crates/starbreaker-ui/src/text/swf_draw.rs` (space advance `size × 0.33`) | Word-space advance for SWF text | Text renderer | Every space character | Width drift on multi-word text vs the reference | Load-bearing — the engine renders spaces ~0.1×em wider than the font's space-glyph advance (≈0.225×em); the font advance is NOT a valid substitute (verified: it regresses the reference-matched widths by ~2%). Retire only if the engine's word-spacing rule is recovered from data. |
| `crates/starbreaker-ui/src/ui_ir/.../part_09.part` (`fixed_band_heading_prompt_size`, `FIXED_BAND_HEADING_FILL = 0.375`) | Fixed-height heading band → reduced "banner prompt" size = `band_height × 0.375` (pre-caps; the banner is `caseModifier: Upper`, so the result is further scaled by `CAPS_FORCED_UPPER_FONT_SCALE`, ≈0.98 → effective ≈0.3675, calibrated to the user's 96% width measurement of the Clipper medical WelcomeText) | UI font sizing | A `Heading1` whose `sizing.height.behavior == "Fixed"` and band height `< 2.5 ×` the brand FontSize (the engine fits such a heading to its band; this band-fit lives in compiled BuildingBlocks layout, not DataCore). Verified the data does not carry it: the field fits the heading at full size, `overflow:Visible`, no wordWrap/maxFontSize/scale, plain loc string. | font-size harness drift on the medical `WelcomeText` prompt | Replace if the engine's fixed-band heading fit is recovered from data (e.g. a band layout/leading field) instead of a calibrated ratio. |
| `crates/starbreaker-ui/src/ui_ir/.../part_10.part` (`CAPS_FORCED_UPPER_FONT_SCALE = 0.98`, applied in part_04/part_05) | All-caps display reduction: text whose `caseModifier ∈ {Upper, AllCaps}` renders at ~0.98× its nominal brand/authored font size (line spacing scales with it). Excludes `autoFontSize` (fit to rect render-side). | UI font sizing | Any `WidgetTextField` / caption-pair label or value with `caseModifier`/`labelProperties.caseModifier`/`captionProperties.caseModifier` = `Upper`/`AllCaps` | Width drift on all-caps elements vs the reference (the Clipper medical header reads every `caseModifier: Upper` element at ~98% of the renderer's nominal width; mixed-case `None` "Drake Clipper" is exact). Gate is the `caseModifier` itself — `T3` (unchanged by an upper-case transform) is in the reduced set, ruling out a glyph-substitution cause. | Replace if the engine's all-caps layout reduction is recovered as a numeric field in DataCore (e.g. a per-style caps tracking/scale) instead of a calibrated ratio. |

## Recently Retired / Reduced Fallbacks

| Component | Previous Fallback | Status | Evidence |
| --- | --- | --- | --- |
| `crates/starbreaker-ui/src/pipeline.rs` | Pipeline-local defaults builder and split fallback policy | Retired | Consolidated to `DefaultValueRegistry::with_pipeline_defaults(...)` in Phase 4 |
| `crates/starbreaker-ui/src/ir_compose.rs` | Name/path-based screen/manufacturer hardcoded rendering branches | Retired | Eliminated in Phase 2B source-backed IR pass and guarded by `.github/scripts/check_ui_hardcoding.sh` |
| `crates/starbreaker-ui/src/pipeline/swf_selection/flash_paths.rs` | Hard-coded `brand_ship_subdirs` / `annunciator_ship_subdirs` ship lists for SWF path probing | Retired (Phase 1, 2026-06-05) | Replaced by `p4k_ship_subdirs` which enumerates P4K brand dir dynamically; no hard-coded ship names remain. |

## Operational Policy

- Every fallback in production must be listed here.
- New fallbacks must declare scope, trigger, and sunset target before merge.
- Fall-backs without telemetry or retirement criteria are not allowed.
- When a fallback is removed, move it to the retired section with evidence.
