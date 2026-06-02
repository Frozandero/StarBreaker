# Target Master Canvas — Phase A Findings

**Date**: 2026-01-06
**Canvas**: `buildingblocks_canvas_mc_s_target_master`
**Manufacturer**: drak
**Ship**: Clipper

## Canvas Source Record

- **Record Name**: `BuildingBlocks_Canvas.MC_S_Target_Master`
- **Record ID**: `b8d2d65c-05c5-49f2-bdf5-3a722c92a3d9`
- **Widget `canvas_TargetStatus`**: `rendererType: "Flash"` — confirmed Flash-rendered widget
- **Canvas size**: 1920x1080
- **Coordinate method**: `aspectOverridesWidth`

## Canvas Reference Chain

- `defaultStyles.entries[0].modifiers[0].field.value` → `gen_mc_s_target.json` (generic fallback canvas)
- Brand styles reference: `mrai_mc_s_target.json`, `rsi_mc_s_target.json`, `grin_mc_s_target.json`
- `gen_mc_s_target.json` has **empty `defaultStyles.entries`** — no canvas reference, only brandStyles

## SWF Selection Analysis

### DRA brand candidates (fail):
- `DRA/SupportScreen16-9/TargetStatus.swf` ❌ DOES NOT EXIST in P4k
- `DRA/SupportScreen16-9/Target.swf` ❌ DOES NOT EXIST in P4k

### RSI fallback candidates (generic MC):
- `RSI/SupportScreen16-9/TargetStatus.swf` ✅ EXISTS (51.4 KB)
- `RSI/SupportScreen16-9/Target.swf` ❌ DOES NOT EXIST in P4k

### Other brand SWFs that DO exist:
- `AEG/SupportScreen16-9/TargetStatus.swf` ✅ EXISTS (58.5 KB)
- `AEG/SupportScreen1-1/TargetStatus.swf` ✅ EXISTS (58.0 KB)
- `KRI/SupportScreenBespoke2/TargetStatus.swf` ✅ EXISTS (51.9 KB)
- `DRA/DRAK_Dragonfly/Support_Bespoke_2/TargetStatus.swf` ✅ EXISTS (37.9 KB, ship-specific)
- `AEG/AEGS_Retaliator\SupportScreenBespoke2\TargetStatus.swf` ✅ EXISTS (37.9 KB, ship-specific)

## Renderer Hint Issue

- IR query returns `renderer_hint: "bb"` instead of `"hybrid"` or `"swf"`
- **Root cause identified**: MCP `ui_ir_query` uses `McpNullSwfFetcher` — SWF fetches always fail,
  so `valid_candidates` is always empty, so `selected_swf_source` is always `None`,
  so `renderer_hint` is always `UiRendererHint::Bb`.
- **This is expected behavior for the MCP query tool** — it cannot be used to test SWF rendering.
- The actual rendering pipeline (via CLI export) uses `P4kSwfFetcher` which CAN reach P4k.
- **The IR renderer_hint logic** (from `part_03.part`):
  ```rust
  let renderer_hint = match (has_selected_swf_source, has_text, has_custom_shape) {
      (true, true, true) => UiRendererHint::Hybrid,
      (true, false, true) => UiRendererHint::Swf,
      _ => UiRendererHint::Bb,
  };
  ```
  If `selected_swf_source` is `Some(...)`, the renderer will use SWF/hybrid mode.
- **The actual export pipeline** should work correctly if SWF candidates are valid and P4k is reachable.

## SWF File Locations in P4k

```
Data\UI\ShipInterface\assets\SWF\AEG\SupportScreen1-1\TargetStatus.swf
Data\UI\ShipInterface\assets\SWF\AEG\SupportScreen16-9\TargetStatus.swf
Data\UI\ShipInterface\assets\SWF\RSI\SupportScreen16-9\TargetStatus.swf
Data\UI\ShipInterface\assets\SWF\KRI\SupportScreenBespoke2\TargetStatus.swf
Data\UI\ShipInterface\assets\SWF\DRA\DRAK_Dragonfly\Support_Bespoke_2\TargetStatus.swf
Data\UI\ShipInterface\assets\SWF\AEG\AEGS_Retaliator\SupportScreenBespoke2\TargetStatus.swf
```

## Brand Styles Summary (from gen_mc_s_target.json)

## Visual Difference Catalog (Generated vs Reference)

| # | Element | Generated (BB) | Reference (SWF) | Ownership |
|---|---------|----------------|-----------------|-----------|
| 1 | Corner brackets | Present (4 L-shapes) | Absent | bb_layout or scene |
| 2 | "NO TARGET" text | Present, centered | Present, with ">>" and "<<" markers | Text content OK, markers missing |
| 3 | Dashed separator lines (top/bottom) | Missing | Present | bb_layout or draw_primitives |
| 4 | ">>" and "<<" markers | Missing | Present | Scene elements or SWF-only |
| 5 | Navigation footer bar "< TARGET_STATUS >" | Missing | Present | Scene elements or SWF-only |
| 6 | Greeble rectangles | Present (brown rects) | Absent | bb_layout or scene |
| 7 | Background color/tone | Dark brown | Dark amber/brown | Style colors |
| 8 | Overall layout | BB-rendered approximation | SWF-rendered flash | Pipeline: SWF not loaded |

**Key finding**: The generated image shows BB-rendered elements that are NOT in the reference
(corner brackets, greeble rectangles), and is MISSING elements that ARE in the reference
(dashed lines, footer bar, navigation markers).

This strongly suggests:
1. The SWF IS the authoritative source — the BB scene contains elements that are
   only meant to be fallbacks or are incorrectly included.
2. The canvas has `rendererType: "Flash"` but the pipeline falls back to BB because
   `selected_swf_source` is None during export.
3. **The fix should be in SWF selection** — ensure the RSI fallback SWF is actually
   being selected and loaded during the export pipeline.
4. The BB-rendered elements (corner brackets, greebles) are from the canvas scene
   that would be overridden by SWF content in hybrid mode.

## Brand Styles Summary (from gen_mc_s_target.json)

The generic canvas has brandStyles for each manufacturer (AEGS, DRak, ARGO, AV, DRAK, RSI, GRIN, MRAI, ANVL, KRI). Each brand style modifies:
- Image paths (greebles, containers, backdrop textures)
- SVG paths (decorative elements)
- Color modifiers (FillColor, StrokeColor)
- Position/size adjustments for brand-specific layouts

Key observation: DRak brand style sets `Page Greebles Left.ImagePath` to empty string `""` — this element would be invisible in DRak brand.