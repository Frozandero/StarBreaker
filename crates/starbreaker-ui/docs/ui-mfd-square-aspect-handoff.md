# MFD per-screen aspect ratio — Step 3 hand-off

**Status:** not started. Steps 1–2 (power-screen card width + battery icon, both
landed/frozen) are done; this is the remaining piece.

**Goal:** the square / non-4:3 MFD screens (g-force/radar = 1:1, `Screen_Radar_RTT`
≈ 5:4, `velocity_ball`, and any other non-4:3 displays) must render at their TRUE
physical aspect instead of the 4:3 every MFD currently gets. The owner confirmed
the symptom: *"some of the other currently blank MFDs are the wrong aspect ratio,
e.g. the g-force one."*

Related memory: `mfd-aspect-tag-content-scaling`, `power-screen-parity-plan`,
`clipper-mfd-hybrid-flash-architecture`. Landed power-screen commits:
`1177002ff`, `58e5b574b`, `d50afa34f`.

---

## 1. Symptom

Every Clipper MFD renders at **4:3 (1600×1200, aspect h/w = 0.75)**. That is
CORRECT for the three big MFDs (power + the two upper screens are physically 4:3)
but WRONG for:

| Screen mesh | True aspect (w/h) | We render | Verdict |
|---|---|---|---|
| `Screen_Left_Lower_RTT` (power) | 1.333 (4:3) | 4:3 | ✅ correct |
| `Screen_Left_Upper_RTT` | 1.333 (4:3) | 4:3 | ✅ correct |
| `Screen_Right_Upper_RTT` (target) | 1.333 (4:3) | 4:3 | ✅ correct |
| `Screen_Radar_RTT` | **1.229 (~5:4)** | 4:3 | ❌ off |
| `Screen_Small_Radar1` / `Screen_Small_Radar2` (g-force) | **1.0 (square)** | 4:3 | ❌ wrong |
| `Countermeasures_Screen` | **1.0 (square)** | 4:3 | ❌ wrong |
| `velocity_ball` (and others) | TBD — not yet measured | 4:3 | ❌ likely wrong |

These aspect values are GROUND TRUTH, measured from the loaded Clipper cockpit
geometry in Blender (§4). A square screen rendered at 4:3 squashes/distorts its
content; the owner sees it most clearly on the blank g-force canvas shape.

---

## 2. The engine mechanism (the full chain — researched this session)

The render aspect is data-driven, but the INPUT aspect for "auto" screens is the
cockpit mesh geometry. The chain:

1. **Per-screen aspect source = the display entity.** The Clipper's screen
   hardpoints map to display entities (from the ship loadout — see §4 for the
   query):
   - `Screen_Left_Lower_RTT` / `Screen_Left_Upper_RTT` / `Screen_Right_Upper_RTT`
     → **`Vehicle_Screen_MFD`**
   - `Screen_Radar_RTT` → **`Radar_Display_Screen_Template`**
   - `Screen_Annunciator_L` / `_R` → `Vehicle_Screen_Physical`
   - `hardpoint_engineering_screen` → `SoftLock_EngineeringScreen_StandardShip`

   Each display entity carries
   `Components[UIRenderToTextureEntityComponentParams].aspectRatioOverride`:
   - **non-zero** → explicit aspect (physical screens in the data use 0.52 / 0.55
     / 3.0; holo MFDs use 1.777778).
   - **0.0** → AUTO: the engine derives the aspect from the actual screen at
     runtime (the cockpit mesh / render target shape).

   Both `Vehicle_Screen_MFD` and `Radar_Display_Screen_Template` have
   `aspectRatioOverride = 0.0` (AUTO), an empty `SGeometryResourceParams` geometry
   path, and `Components[SCItemDisplayScreenComponentParams].uiSourceParams
   .screenPreset = null`. So for the Clipper, **the aspect is auto = the cockpit
   screen MESH aspect** (the §4 Blender numbers). When a screen DOES set
   `screenPreset` it points at an `SCItemDisplayScreenPreset` (e.g. `MFD_4x3` →
   geometry `Objects/Squadron42/universal/ui/ui_screen_4x3_a.cgf` + material
   `Materials/UI/ui_screen_mfd_master.mtl`; presets exist for 16_9 / 4_3 / 9_16 —
   **there is NO 1x1 preset**, which is why square screens must come through the
   auto/geometry path, not a preset).

2. **Aspect → layout tag.** The aspect is quantized to a tag via a
   `BuildingBlocks_AspectRatioLibrary` record. MFD screens use
   `AspectRatioToTag_MFD` (GUID `bb73a18f-d53f-4de3-8ea1-be28c80d70e1`):
   - 1.777778 → tag `135f3670-…` (16:9)
   - 1.333333 → tag `efca6c81-9944-4038-af42-0760d058894b` (4:3)
   - 1.0      → tag `b976442c-2c5e-4357-a12d-ba74eff985b2` (1:1)

   The general `AspectRatioToTag` (`fe0cd29b-…`) additionally has 2.333 / 3.556 /
   0.5625 / 0.75 / 0.28125. The matched tag is applied to the MFD root at runtime.

3. **Tag → responsive layout.** `m_eng_mfdcontent` (the MFD content wrapper)
   carries "Content Canvas Scaling" `embeddedStyles` gated on the aspect tag:
   - 16:9 → `SizeX 2.0`, `WidthBehavior = PercentOfY`
   - 4:3  → `SizeX 1.45`, `PercentOfY`
   - **1:1 → `SizeX 1.45` PercentOfY + `ScaleX 0.79` + `ScaleY 0.79` + `AnchorY
     0.45` + `PivotY 0.5`** ← the square case also SCALES the content down 0.79
     and re-anchors.

The owner's hunch ("star engine may not use geometry but DataCore data") is
half-right: the LAYOUT adaptation is DataCore (AspectRatioToTag); the INPUT aspect
for these auto screens is the mesh geometry.

---

## 3. Where we currently get the aspect (and why it's wrong for step 3)

`crates/starbreaker-ui/src/pipeline/canvas_aspect.rs :: frame_canvas_aspect` reads
the **frame canvas** record's authored `size` (x,y) and returns `h/w`. For the
power MFD the frame canvas is `M_MFD_Screen` (GUID
`33bda02c-099a-447f-ba1e-2e6b59bfafce`, `size` = 800×600 → 0.75 = 4:3).

`crates/starbreaker-ui/src/pipeline/mod.rs` (~lines 289–343) uses it:

```rust
let frame_aspect = if b.binding_kind == Some("mfd") {
    frame_canvas_aspect(b.canvas_guid, b.content_canvas_guid, inputs.canvas_fetcher)
} else { None };
if let Some(aspect) = frame_aspect {
    let width = inputs.target_size.0.max(1);            // texture/mip width (fixed)
    let height = ((width as f32) * aspect).round() ...; // height derived from aspect
    effective_target_size = (width, height);
}
```

and the same `frame_aspect` feeds `apply_mfd_content_canvas_scaling`
(commit `1177002ff`).

**The bug:** every MFD whose frame canvas is `M_MFD_Screen` (4:3) gets aspect
0.75, regardless of its physical shape. `frame_canvas_aspect` is the SHARED frame
shape, not the per-screen physical aspect. (Open question O1: do the radar/square
screens even bind a distinct frame canvas, or the same `M_MFD_Screen`? If they
have their own square frame canvas, fixing `frame_canvas_aspect`'s source record
may already help; if they share `M_MFD_Screen`, they need the per-screen aspect
threaded in. Verify before coding.)

Note `inputs.target_size.0` (the render WIDTH) comes from the RTT texture mip in
the export, so the width may already be per-screen; it is the HEIGHT that
`frame_aspect` overrides to 4:3. Determine whether the per-screen RTT texture is
square for square screens (then maybe the override should be skipped) or whether
height must be derived from the per-screen aspect (O2).

---

## 4. Evidence gathered (reproduce these)

**Mesh aspects (Blender, Clipper loaded).** With a screen mesh isolated in edit
mode, outer edges selected, the aspect = larger/smaller of the two in-plane axes
(drop the smallest axis = depth/curvature). Curvature-corrected via per-edge UV
arc length. Script pattern used (run via the blender MCP `execute_blender_code`):

```python
import bpy
def plane_aspect(o):
    vs=[v.co for v in o.data.vertices]
    xs=[v.x for v in vs]; ys=[v.y for v in vs]; zs=[v.z for v in vs]
    dims=sorted([('x',max(xs)-min(xs)),('y',max(ys)-min(ys)),('z',max(zs)-min(zs))],key=lambda t:t[1])
    a,b=dims[1][1],dims[2][1]
    return round(b/a,4)  # long/short of the two in-plane axes
# power Screen_Left_Lower_RTT -> 1.333; Screen_Small_Radar2 -> 1.0; Screen_Radar_RTT -> 1.229
```

Power CRT is curved (depth in local Y ≈ 0.0095 of a 0.194 span); curvature only
adds ~0.6% so the chord (0.75) and arc-corrected (0.76 h/w) agree — it is a true
4:3. `velocity_ball` and any other instruments were NOT measured — re-run the
sweep over all screen meshes.

**Display entity aspect data (StarBreaker MCP `datacore_query`):**

```
Vehicle_Screen_MFD  Components[UIRenderToTextureEntityComponentParams].aspectRatioOverride -> 0.0
Vehicle_Screen_MFD  Components[SCItemDisplayScreenComponentParams].uiSourceParams.screenPreset -> []
Vehicle_Screen_MFD  Components[SGeometryResourceParams].Geometry.Geometry.Geometry.path -> ""
Radar_Display_Screen_Template  ...aspectRatioOverride -> 0.0
```

Component list on a representative MFD display entity
(`vehicle_screen_mfd_holographic`): `SCItemDisplayScreenComponentParams` (idx 2,
has `uiSourceParams.screenPreset`), `UIRenderToTextureEntityComponentParams`
(idx 7, has `aspectRatioOverride`, `renderType`, camera `fieldOfView`/`nearClip`
— NO pixel resolution), `UIBuildingBlocksEntityComponentParams` (idx 12),
`ItemDashboardScreenMFDParams` (idx 13, `isHolographic`).

**Loadout mapping (which entity each screen uses):** walk
`entities/spaceships/drak_clipper.json` for `itemPortName` → `entityClassName`
where the class name contains screen/mfd/display/radar (the §2 table).

**The aspect→tag records:** `ships/dcb_canvas/.../ui/aspectratiototag_mfd.json`
and `.../ui/buildingblocks/buildingblockspresets/aspectratio/aspectratiototag.json`;
the screen presets are under `ships/dcb_canvas/.../scitemdisplayscreenpreset/`
(`mfd_4x3.json`, `16_9_hightech.json`, `4_3_lowtech.json`, `9_16_hightech.json`, …).

---

## 5. What is already built (reuse it)

The power-screen work added the data-driven machinery; step 3 mostly needs to feed
it the RIGHT per-screen aspect.

- `crates/starbreaker-ui/src/pipeline/aspect_tag.rs` (pure, 4 unit tests):
  - `nearest_aspect_tag(library: &Value, aspect_w_over_h: f32) -> Option<String>`
    — maps a continuous aspect to the nearest `AspectRatioToTag_MFD` option's tag.
  - `content_scaling_width(frame_record: &Value, tag_id: &str) -> Option<ContentScalingWidth>`
    — reads the matched tag's `SizeX` + `PercentOfY` from a content-frame
    `embeddedStyle`. **Only returns width (SizeX); the 1:1 `ScaleX/ScaleY/AnchorY/
    PivotY` are NOT yet read** — extend this for the square case (§7).
- `crates/starbreaker-ui/src/pipeline/mod.rs :: apply_mfd_content_canvas_scaling`
  — resolves the tag, BFS-walks the frame's `canvas:` embeds to find the
  content-frame, locates the landscape content slot via
  `crate::mfd_view::landscape_slot_id`, and sets its width
  (`scaling.size_x / MFD_CONTENT_HEIGHT_FRACTION`, PercentOfY). The aspect it
  passes is `frame_aspect` (h/w) — THIS is the value step 3 must make per-screen.
- `crates/starbreaker-3d/src/ui_pipeline.rs :: datacore_ui_lookup_type_names`
  now indexes `BuildingBlocks_AspectRatioLibrary` so `AspectRatioToTag_MFD`
  resolves by name in the export fetcher.

---

## 6. What step 3 needs to do

1. **Compute the per-screen aspect (w/h):**
   - if the display entity's `aspectRatioOverride != 0.0` → use it;
   - else if `screenPreset` is set → use the preset geometry aspect (decode the
     preset name / `ui_screen_*x*_a.cgf`);
   - else (AUTO, the Clipper case) → use the **cockpit screen MESH aspect**.
2. **Thread that aspect into the UI binding.** The mesh aspect is only known on the
   3d/export side, so `UiBinding` (`crates/starbreaker-3d/src/types.rs`) / the
   `UiBindingView` (`pipeline/mod.rs`) need a per-screen aspect field that the
   export populates (the export already walks the ship geometry; compute each
   screen helper's mesh plane aspect there — same `plane_aspect` logic as §4, or
   from the RTT texture dimensions if those carry the aspect).
3. **Use that per-screen aspect** in place of `frame_canvas_aspect` for BOTH:
   (a) `effective_target_size` (the RTT render dimensions — resolve O2 first), and
   (b) `apply_mfd_content_canvas_scaling` (the AspectRatioToTag → Content Canvas
       Scaling). `nearest_aspect_tag` already maps 1.0 → the 1:1 tag, so once the
       right aspect arrives the square layout is selected automatically.
4. **Honour the 1:1 Content Canvas Scaling extras** (ScaleX/Y 0.79, AnchorY 0.45,
   PivotY 0.5) — see §7.
5. Keep it generic/universal — no per-screen-name gating (a hard project rule).

---

## 7. The 1:1 special case

`content_scaling_width` currently returns only `SizeX`. For the square tag
(`b976442c`) the "Content Canvas Scaling (1:1)" `embeddedStyle` also sets
`ScaleX 0.79`, `ScaleY 0.79`, `AnchorY 0.45`, `PivotY 0.5` (all
`FieldModifierNumber`). Extend the resolver to return these and apply them to the
content slot (scale + anchor/pivot) alongside the width. Verify against an
in-game square-screen reference once one is captured.

---

## 8. Open questions to resolve before coding

- **O1** — Do the radar/square screens bind a DISTINCT frame canvas (square
  `size`) or share `M_MFD_Screen` (4:3)? Dump the binding's `canvas_guid` /
  frame record `size` for `Screen_Radar_RTT` / a square screen. If distinct +
  square, `frame_canvas_aspect` may already be right and the bug is narrower.
- **O2** — RTT render resolution per screen: is `inputs.target_size` (texture mip)
  already square for square screens (then SKIP the height override), or 4:3-shaped
  (then derive height from the per-screen aspect)? Check the exported RTT texture
  dimensions for a square screen.
- **O3** — `velocity_ball` and the other unlisted instruments: find their display
  entity + mesh aspect (re-run §4 over all screen meshes; map via the loadout).
  Some may not be MFD-binding screens at all.
- **O4** — Do square/radar screens even have bound CONTENT (they looked blank), or
  is only the blank canvas aspect wrong? The blank shape still shows the aspect.

---

## 9. Verification

- **Mesh aspect:** Blender MCP, §4 script (Clipper must be loaded).
- **Render aspect:** `./target/release/starbreaker ui render --scene
  "$HOME/projects/scorg_tools/ships/Packages/DRAK Clipper_LOD0_TEX0/scene.json"
  --helper Screen_Radar_RTT` (P4K auto-discovered). FAST (~9s/screen). Then
  inspect the PNG dimensions / the blank canvas shape.
- The `mfd_ir_dump` example (`cargo run -p starbreaker-ui --example mfd_ir_dump --
  <frame-guid> <content-guid> <filter> <WxH>`) gives computed rects but is SLOW
  (~94s — it parses the whole record mirror at startup; that is harness cost, NOT
  a pipeline loop — see §10). Prefer `ui render`.
- After any change that touches the MFD aspect/content-scaling, the frozen
  `clipper_target_master` (platinum) and `clipper_power_master` (gold) baselines
  may shift again → re-freeze via `bash scripts/ui_freeze_cycle.sh --approver
  owner --reason "…"` then `bash scripts/freeze_ui_snapshot_ir.sh --approver owner
  --reason "…"` (read its delta), and bump the manifest count if you register new
  targets. Baseline-affecting → owner-approval-gated.

---

## 10. Gotchas / lessons (paid for this session)

- **The `mfd_ir_dump` harness is slow (~94s), not a loop.** It loads + parses
  every JSON in the record mirror at startup. The real export pipeline (`ui
  render` / `entity export`, DataCore fetcher) renders a screen in ~9s. Do NOT
  declare an infinite loop from a high-CPU/high-RSS sample — time it to completion
  or use the real fetcher. (Cost me a long detour believing the PercentOfY width
  cycled; it did not.)
- **`PercentOfY` width works** (the two-pass layout resolver handles it); no cycle.
- **The content-height calibration is sensitive near the flex unshrink
  threshold** — the battery icon jumps from squashed to full (80px) over a narrow
  factor range; small aspect/height errors move it a lot.
- **Measuring dim (alpha-0.2) glyphs from the PNG is unreliable.** Use a temporary
  render-time probe of the icon draw rect (`iw`/`ih` in `draw_ir_node`, the custom
  shape path at `ir_compose/.../engine_01.part:~302`) rather than pixel scanning.
- **MCP fetcher is canvas-only by name** (`mcp/src/tools.rs :: find_by_name`
  searches `BuildingBlocks_Canvas` only) — `ui_ir_query` won't resolve
  `AspectRatioToTag_MFD`, so it won't reflect the content-scaling path unless that
  fetcher is extended too. The export fetcher was already extended.
- **No per-asset-name gating** and **no hard-coded game-data values** — read from
  DataCore / thread from geometry; measured constants (like the host inset or
  `MFD_CONTENT_HEIGHT_FRACTION`) need a provenance note.

---

## 11. Key references

- Records (StarBreaker MCP / `ships/dcb_canvas/libs/foundry/records/`):
  `Vehicle_Screen_MFD`, `Radar_Display_Screen_Template`, `Vehicle_Screen_Physical`
  (display entities); `AspectRatioToTag_MFD` (`bb73a18f…`), `AspectRatioToTag`
  (`fe0cd29b…`); `SCItemDisplayScreenPreset.MFD_4x3` (+ 16_9/4_3/9_16);
  `M_MFD_Screen` (`33bda02c…`, frame), `M_Eng_MFDContent` (content wrapper with the
  Content Canvas Scaling styles), `MC_S_Power_Master` (`3f3ba0cc…`).
- Code:
  `crates/starbreaker-ui/src/pipeline/canvas_aspect.rs`,
  `crates/starbreaker-ui/src/pipeline/mod.rs` (~289–343 + `apply_mfd_content_canvas_scaling`),
  `crates/starbreaker-ui/src/pipeline/aspect_tag.rs`,
  `crates/starbreaker-ui/src/mfd_view.rs` (`landscape_slot_id`),
  `crates/starbreaker-3d/src/types.rs` (`UiBinding`),
  `crates/starbreaker-3d/src/ui_pipeline.rs` (export UI pipeline + name index).
- Tag GUIDs: 16:9 `135f3670-b489-4727-b58d-7fb2570db0c1`; 4:3
  `efca6c81-9944-4038-af42-0760d058894b`; 1:1
  `b976442c-2c5e-4357-a12d-ba74eff985b2`.
