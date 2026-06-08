# `starbreaker-ui` — Performance Baseline (B0)

**Date:** 2026-06-08  
**Machine:** Linux (same machine as dev)  
**Build profile:** `release` (Rust optimised)  
**P4K build:** LIVE Data.p4k  
**Benchmark ship:** `DRAK_Clipper`  
**Run mode:** `RAYON_NUM_THREADS=1` (serial, for per-image isolation)  
**Command:**
```bash
SB_UI_TIMING=1 SC_DATA_P4K="..." RAYON_NUM_THREADS=1 RUST_LOG=info \
  starbreaker entity export "DRAK_Clipper" "ships" \
  --kind decomposed --lod 0 --mip 0 --materials all
```

---

## X — Aggregate stage breakdown (all 43 bindings, serial)

| Stage | Total (s) | % of wall |
|---|---|---|
| `ir_compile` (`compile_ui_ir_from_scene_with_animation_sample`) | 303.9 | **93.2%** |
| `graph2` (`resolve_canvas_graph_with_loc_and_bound_view`) | 3.5 | 1.1% |
| `render` (rasterise + composite) | 3.4 | 1.0% |
| `swf_load` (`load_first_swf`) | 2.7 | 0.8% |
| `encode` (PNG encode) | 0.1 | 0.0% |
| `manifest` (`build_asset_reference_manifest`) | 0.1 | 0.0% |
| `graph1` (`CanvasWidgetTreeResolver::resolve`) | 0.0 | 0.0% |
| `fetch` / `style_load` | 0.0 | 0.0% |
| **Total serial** | **326.1** | 100% |

> Note: `compile` (314.4s) wraps `ir_compile` + setup; `ir_compile` is the stage timer
> for `compile_ui_ir_from_scene_with_animation_sample` alone. Percentages relative to
> total wall time (326.1s), not to `compile`.

---

## X — Per-binding totals (named bindings only)

| Binding | Kind | Total (s) |
|---|---|---|
| Screen_Left_Lower_RTT | mfd | ~78 (two renders: 77.5s + 79.1s) |
| Screen_Radar_RTT | radar | 67.1 |
| Screen_Left_Upper_RTT | mfd | 26.4 |
| Screen_Right_Upper_RTT | mfd | 19.5 |
| `?` (door/medical) | physical | 10.0 |
| `$slot_standing_screen` | physical | 5.3 |
| Screen_Annunciator_L | physical | 4.2 |
| Screen_Annunciator_R | physical | 4.2 |
| Screen_Left_Upper_RTT_Small | physical | 2.7 |
| Screen_Small_Radar2 | physical | 2.3 |
| cabinet_attach_loc (×2) | physical | ~1.8 |
| Screen_Small_Radar1 | physical | 1.5 |
| Countermeasures_Screen | physical | 1.4 |
| screen_flight_hud_right_upper | physical | 1.3 |
| screen_flight_hud_right | physical | 1.3 |
| Screen_Central_Compass | physical | 1.2 |
| `?` (radar small) | radar | 1.2 |
| screen_flight_hud_left_upper | physical | 1.2 |
| screen_flight_hud_left | physical | 0.8 |
| `?` (multiple fast ≤0.6s each) | physical | 0.4–0.6 |
| `?` slow single | physical | 3.7 |

**Max single-image time: ~79s.** Median: ~0.8s.  
**Total serial time for 43 bindings: 326.1s.**

---

## Key finding

`compile_ui_ir_from_scene_with_animation_sample` accounts for 93% of all render time.  
Within that function, the dominant cost is per-node `resolve_record`/`fetch_canvas_by_name`
calls — O(records) DataCore scans per style-tag, per node (driver #1 from §3 of the review).

`graph2` (the second full graph-resolution pass) is 1.1% — cheap but still wasteful.  
SWF loading (`load_first_swf`) is 0.8% — the 20×+ SWF re-parse problem (driver #3) is real but
secondary to the DataCore scan problem.

---

## Optimisation plan (ordered by measured impact)

1. **B2a** — memoising canvas fetcher: cache `guid/name → Value` per binding. Should cut `ir_compile` dramatically (every repeated scan becomes a hash lookup).
2. **B2b** — name/path O(1) index: build once, turn O(records) scans to O(1). Completes what B2a starts.
3. **B2c** — stop decompressing textures for diagnostics (currently <0.1% — low priority but cheap).
4. **B2d** — load localization once (currently <0.1% — low priority but cheap).
5. **Re-profile** after B2a+B2b to see updated split before tackling SWF/graph work.
