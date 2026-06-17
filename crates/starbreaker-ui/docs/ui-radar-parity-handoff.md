# Radar screen (Screen_Radar_RTT) parity — handoff

Arc started 2026-06-17. Screen: `Screen_Radar_RTT` (Drake Clipper cockpit radar
MFD). Mode: fully-automated. First content parity pass (only the screen aspect
was touched before — dossier).

## Verified facts

- `Screen_Radar_RTT` binds canvas `BuildingBlocks_Canvas.MapDisplayMaster`
  (GUID `c4fd4ccf-9745-4a83-b9a7-a1aafad77855`), `binding_kind = radar`
  (from the exported LOD0 `scene.json` ui_bindings — ground truth, workflow §9).
- Both reference captures (`Screen_Radar_RTT.png`, `mapdisplaymaster.png`) show
  the SAME radar scope: elliptical concentric rings + 6 radial spokes + central
  white ship triangle + orange contact chevrons round the perimeter + perimeter
  ticks + "130°  0.7km" heading/range readout at bottom, under the DRAK CRT
  vignette. The SC cockpit radar IS a local-area map display, so MapDisplayMaster
  is the right canvas.

## What the render currently shows (baseline, md5 5afe4a36)

Near-blank: a giant cream/pale-yellow rounded panel filling most of the screen,
stray MFD chrome (nav arrows `‹ ›`, partial "APARTMENT/HABITATION" top-centre,
an "ALREADY SELECTED" cream box bottom-left, a lone orange "0" bottom-centre).
The whole radar scope is absent.

## Root-cause map (from the IR dump, 637 nodes)

MapDisplayMaster is a MULTI-MODE map component. Top-level sub-displays (all
ACTIVE at once at static rest — the bug):
`StarMapDisplay`, `InteriorMapDisplay`, `MapVelocitySFX`, `StarMapDisplayRTT`,
`InteriorMapDisplayRTT`, `UIOverlayRTT`, `GalaxyMapDisplay`, `canvas_Readouts`.

- Mode flags are engine state under `/MapNamespace`:
  `IsInteriorMapActive`, `GeneralMapData/IsStarMapActive`, `IsMapActive`,
  `IsVolumetric`, `IsRTT`; plus `GeneralMapData/DisplayRadius`,
  `DisplayOrientation/{x,y,z}`, `DisplayPosition/{x,y,z}`. Absent at static rest
  → per-mode visibility gating doesn't select one mode → InteriorMap chrome wins.
- The actual radar scope is `PlayerRadarPlane > PlaneRoot > RadarCircleBase >
  HostplaneVisuals_Small > RadarPlaneRingsBase` (rings `Circle_Ripple_Textured`)
  under `StarMapDisplayRTT > CanvasProxyRoot > WindowContainer > 3D Root` —
  currently `[OFF]`. The rings are 2D widgets parented under a 3D-projected /
  RTT plane whose transform comes from live camera/orientation; at rest they
  collapse to ~0×0. This is a live 3D-RTT render (analogous to the self-status
  hologram, ledger 70).
- Radar readouts ("130° 0.7km") come from `mapdisplaystarmap_radarreadouts`
  under `canvas_Readouts`; only "0" renders at rest. Heading 130° + contacts are
  LIVE state (unreproducible statically, like the compass live heading);
  `DisplayRadius` would drive the range.
- Contact chevrons + perimeter ticks are projected from live nearby-contact data
  (`EdgeMarkers`/`VisibleMarkers` lists — empty at rest).

## Naming caveat (owner-flagged 2026-06-17)

The canvas's `StarMapDisplay*` / `IsStarMapActive` identifiers are the GENERIC
map-render subsystem (the same pipeline that draws the radar plane) — **NOT the
in-game navigation "starmap"**. This screen is the **MFD radar**. The radar mode
is distinguished by `IsRadar` (+ `IsRTT`); the radar plane is hosted in the
StarMap-RTT display, so `IsStarMapActive` rides along as its host flag.

## LANDED — catalog #1 (mode gating)

Registry pins added (`default_value_registry_v1.json`, exact raw binding-string
keys): `IsRTT=true, IsStarMapActive=true, IsRadar=true, IsInteriorMapActive=false,
IsGalacticMapActive=false, IsVolumetric=false, IsMapActive=true`. TDD guard:
`bb_state_filter::tests_b::radar_mode_registry_defaults_select_starmap_rtt_over_interior_map`
(red→green). Re-render (md5 99cdab81): the interior-map cream over-paint is GONE,
the DRAK dashboard vignette shows, node count 637→148. Catalog #1 + #6 ✓. The
`canvas_Readouts` "RadarMagnification" bar now renders (via `IsRadar`) — currently
a `LockedIcon` + "º" heading suffix + empty magnification (catalog #3, below).

## Diff catalog (priority order) + status

1. **[CRITICAL] Mode gating — interior-map chrome over-paints.** ✅ **FIXED**
   (registry pins, above). Interior/Star/Galaxy modes deactivated; dark vignette
   shows; node count 637→148.
2. **[CRITICAL/dominant] Radar scope grid** (rings/spokes/ship triangle).
   ✅ **LANDED — data-faithful RTT renderer (owner directed "build it").** The disc
   is NOT invented: it's the REAL engine texture
   `UI/Textures/R_RadarMapScreen/3D_Object_Textures/r_radarmapscreen_radial_gradients.dds`
   (concentric rings + perimeter degree-tick scale + axis), bound by the
   `Circle_Radial_Grid` Primitive node's `ui_r_radarmapscreen_radial_grid.mtl`
   (TexSlot1). Pipeline (mirrors the SELF-STATUS hologram):
   `gfx::radar_plane::project_radar_disc` decodes + tilt-projects that texture
   (flat disc → ellipse) tinted by the brand `Accent1`, + the `Circle_Ripple_Textured`
   WidgetCircle outer ring (Accent stroke) + the white own-ship triangle;
   `HologramFetcher::fetch_radar_plane` (P4kHologramFetcher) loads the brand disc
   `.mtl`→texture (split-mip DDS) and projects; `ir_compose` composites it into the
   radar `WidgetWindow` (material `map_window`), gated so no frozen screen is
   touched. **PER-MANUFACTURER**: the IR `primitive_material` prefers the
   cascade-applied `PrimitiveMaterialPath` brand override (Greycat `ui_grin_…`,
   RSI `…_RSI`) over the authored generic (DRAK keeps generic) — so the disc
   texture is manufacturer-correct. **DATA vs owner-tuned**: disc art + ring +
   ship + tint all DATA; ONLY the camera tilt (37°, the engine runtime camera is
   absent at rest) + emissive brightness are owner-tuned (the hologram-camera
   boundary). Commits `9e38f1501`/`d76270597`/`e530c0820`. RESIDUAL: the 6 radial
   spokes read fainter than the reference (in the gradients texture the material
   binds; ref likely brighter via emissive bloom / radialUV); live contacts +
   heading/range "130° 0.7km" unreproducible at rest (live state, like compass).
   **History — ⛔ was a PROVEN 3D-RTT BLOCKER (researched per owner):**
   After the gating fix the radar plane IS active but COLLAPSED to ~0.5×0.8px at
   centre (512,392): `PlayerRadarPlane > PlaneRoot > RadarCircleBase >
   HostplaneVisuals_Small > RadarPlaneRingsBase` / `RadarSpokeLinesBase` /
   `NavigationGrid{1,2,3}{Mid,Alt}Plane`, all under `3D Root > WindowContainer`.
   **`WindowContainer` is a `BuildingBlocks_WidgetWindow`** (`rendererType:Primitive`,
   material `Materials/UI/Starmap/map_window.mtl`, **`camera {WindowCamera,
   fieldOfView:20}`**, `windowPreviewScene` RTT) — an RTT WINDOW that renders 3D
   content through a 3D camera into a texture. `PlayerRadarPlane` (a `WidgetCanvas`
   loading the radar-disc sub-canvas: rings/spokes/ship/nav-grids) is that 3D
   content. The window's camera transform comes from the live radar state
   (`/MapNamespace/GeneralMapData/DisplayPosition/{x,y,z}`,
   `DisplayOrientation/{x,y,z}`, `DisplayRadius` — decoded NumberVariables, absent
   at static rest) → the projection collapses to a point. Reproducing the scope
   needs a **WidgetWindow RTT 3D-projection renderer + radar-disc geometry
   rasterisation** — a substantial subsystem comparable to / larger than the
   self-status hologram CPU rasteriser (ledger 70), and even with it the at-rest
   camera transform is live (the ref's populated 130°-heading scope is an
   in-flight capture). EVIDENCE TRAIL: `Screen_Radar_RTT`→`MapDisplayMaster`→radar
   mode→`StarMapDisplayRTT`→`mapdisplaystarmap_window`→`WindowContainer`(WidgetWindow,
   camera FOV 20)→`PlayerRadarPlane`(3D disc). Owner declined "build it" (option 3)
   at the catalog gate; researched-further at the major-item gate confirms the
   blocker.
3. **[HIGH] Heading/range readout** "130° 0.7km".
   - **#3a lock icon** ✅ **FIXED** (`ShowRadarLocked=false` pin): the padlock
     (`LockedIcon`, `icon_common_door_locked.svg`) is gone.
   - **#3b orange backplate** ✅ **FIXED** (AR_HoloVolume Primitive-card
     suppression). The readout card `RadarMagnification` is `rendererType:Primitive`
     with material `AR_HoloVolume_standards/ui_ar_card_in_holo_volume.mtl` and
     authored `background.color = ColorStyle "Base"` (DRAK orange). The
     `Type(Card)∧Tag(Locked)→BackgroundColor:null` suppressor exists ONLY in the
     `s_grin_hud`(Greycat)/`s_rsi` brand blocks of `mapdisplaystarmap_radarreadouts`
     — the canvas has NO drak brand block and an EMPTY `defaultStyles`, so a DRAK
     (no-brand-match) ship leaks the authored "Base" backplate as an opaque orange
     bar (the reference shows bright text on the DARK vignette, RGB [54,36,14], NO
     backplate — measured). Fix: `node_background_enabled` now suppresses the flat
     background of a `rendererType:Primitive` node whose `primitiveMaterialPath`
     is an **AR HOLO-VOLUME material** (a 3D-volume card; its flat backplate isn't
     drawn on the flat UI). Discriminator is the MATERIAL, not the colour, so the
     real palette-role Primitive fills keep rendering — power `PipBox_Fill` (empty
     material) + master-mode `card_BarFill` (`materials/default_rtt.mtl`); the
     self-status hologram (`WidgetRuntimeImage`) is excluded. No frozen MFD/HUD
     screen uses AR_HoloVolume cards (grep-verified). TDD guards
     (`node_background_enabled_tests` in `ui_ir/engine_parts/engine_01.part`).
     RESIDUAL (live, deferred): the readout VALUES — heading
     `FlightController/Compass/Value`, range `…/radarrangemeters` — are LIVE; the
     ref's 130°/0.7km is an in-flight capture (render shows at-rest 0°). Like the
     compass live heading, unreproducible statically. So full readout parity is
     bounded regardless.
4. **[MED] Contact chevrons absent** — live nearby-contact data
   (`EdgeMarkers`/`VisibleMarkers`, empty at rest). Deferred-with-proof.
5. **[MED] Perimeter ticks absent** — part of the radar plane (#2).
6. **[LOW] Background vignette** ✅ correct (dark DRAK vignette shows after #1).

## Landed commits
- (this arc) registry radar-mode pins + `ShowRadarLocked` + TDD guards
  (`bb_state_filter::tests_b::radar_mode_registry_defaults_select_starmap_rtt_over_interior_map`,
  `radar_locked_icon_hidden_when_show_radar_locked_pinned_false`).

## Remaining items (owner-requested, diagnosed 2026-06-17)

- **Background image (#11)** — the radar's `DRAK_GroundVehicle_Dashboard_background_2.dds`
  (dark panel + orange edge/corner glow; a DIFFERENT texture than other MFDs) is
  the `image_Background` root node, authored `isActive=false` with `IsActive ←
  NOT(IsVolumetric)`. At the flat radar (IsVolumetric=false) it SHOULD activate,
  but `bb_state_filter` only DEACTIVATES nodes — it never ACTIVATES an
  authored-false node on a true binding. RISK: a generic IsActive-activation pass
  is the documented medical-breaker (workflow §10). Possible SCOPED fix: activate
  authored-false nodes whose `IsActive` op resolves a genuine `Some(true)` from a
  PINNED/resolved state (not the unset→override true that medical's live-gated
  nodes hit) — must validate with `--full` (medical platinum is the canary).
- **Outer dots/chrome (#10)** — `r_radarmapscreen_coordinates(_novalue).dds` is a
  heading-degree-label + glyph ATLAS (010–350 grid); the `headingtape` material
  places cells around the outer ring per heading. Complex (angular atlas-cell
  placement on the tilted ring). Per-manufacturer (`grin_coordinates.dds`).
- **Spokes (#9)** — the radial spokes are in the disc `radial_gradients` texture
  (the material's bound TexSlot1) but read faint; the reference's prominence is
  likely emissive bloom + the radial-UV mapping. Not a separate asset (the
  navigation-grid texture is a plain white line tile; the grid is geometry).
- **Readout kerning (#12)** — the bottom readout (`Text_Radar*` = WidgetTextField
  components, Electrolize font) is too tightly spaced. No authored LetterSpacing
  on the field; it's the font advance / text-format — a delicate text-metrics
  residual (cf. prior LetterSpacing-pitch deferrals).

## Next step
Major-item blocker gate for #2 (radar scope = 3D-RTT render). Owner decision on
#3b (attempt the token-leak fix vs defer).
