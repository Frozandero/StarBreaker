# DRAK Clipper — Target Status MFD parity plan

Reference: `reference/in-game/Clipper/Screen_Right_Upper_RTT.png` (4:3 in-game RTT crop)
Generated: `ships/Data/UI/Generated/ship/drak/Clipper/buildingblocks_canvas_mc_s_target_master.png`

Binding (`scene.json`, helper `Screen_Right_Upper_RTT`, kind `mfd`):
- `canvas_guid`     = `33bda02c…` → `BuildingBlocks_Canvas.M_MFD_Screen`     (frame, 800×600 = 4:3)
- `content_canvas_guid` = `b8d2d65c…` → `BuildingBlocks_Canvas.MC_S_Target_Master` (content, 1920×1080 = 16:9)

## Evidence (DataCore)

- `M_MFD_Screen` 800×600, `coordinateMethod: auto`. Scene = `base_root` + `canvas_MFDContent`
  (WidgetCanvas → `m_eng_mfdcontent.json`, `paramInputValues: []`, instantiation gated on
  `/seatdashboard/powerstate`). The frame does **not** statically name the content screen or the
  footer label — both are runtime dashboard/seat parameters. This is why the pipeline renders
  `content_canvas_guid` directly.
- `M_Eng_MFDContent` 800×600 (4:3). Holds the content slot + `canvas_Header / Footer`
  (11% height, bottom-anchored → `GEN_MC_S_Header`). Its root `base_Root` has authored
  `alpha: 0.0` with **no** per-widget animation (`animation.animationTimeline: null`); the
  fade-to-visible is a runtime "current page" effect, so a static render of this frame is black.
- `GEN_MC_S_Header` renders cleanly standalone; its `text_ScreenName` is the
  placeholder `@ui_leaderboards_Loadout` — the real "TARGET STATUS" label is injected at runtime.
- `MC_S_Target_Master` is authored 1920×1080 but its widgets use relative/percent layout
  (centred text, chevrons, dashed separators).

## Issues → root cause → fix

### #2 Missing main text/image (REGRESSION) — fix first, lowest risk
Cause: in-progress `compile_ir_for_binding` change renders the frame (`M_MFD_Screen`), whose
content sub-tree is alpha-0 → black image.
Fix: revert `compile_ir_for_binding` to render `content_canvas_guid` (with `canvas_guid` fallback).

### #1 Aspect ratio 16:9 → 4:3
Cause: (a) the SWF stage-visual-bounds aspect override forces ~0.609 h/w for MFD;
(b) the MFD target size is 16:9 (1600×900). The physical MFD/frame is 4:3 (800×600).
Fix: drive the MFD render aspect from the **frame canvas** (`canvas_guid`) authored size
(800×600 → 4:3); do not let the SWF stage-bounds override the MFD aspect. Verify the
percent-laid-out content reflows to 4:3 (re-render and compare).

### #4 Font weight/shape ("G" in NO TARGET)
Authored font is `blenderpro-thin`; reference glyphs are heavier + wider-tracked.
Investigate whether (a) the font record fails to resolve and the renderer falls back to a thin
default, or (b) a DRAK brand style overrides the font, or (c) tracking/weight handling differs.
Fix the owning stage (font resolution in `ui_ir` or glyph selection in `compose`).

### #3 Footer "< TARGET STATUS >" (never visible) — largest, decide scope
The footer is chrome from the 4:3 frame (`M_Eng_MFDContent` → `GEN_MC_S_Header`) which the
pipeline never renders. Faithful reproduction needs frame chrome + runtime content injection +
screen-name parameter + page-in alpha resolution. Screen-name string source (dashboard/seat
config) still to be located. Scope options A/B/C — see question to user.

## Status / findings (this pass)

- **#2 DONE**: reverted `compile_ir_for_binding` to content-only rendering. 369 lib tests + snapshot
  + live-IR guards pass.
- **#1 IMPLEMENTED**: added `frame_canvas_aspect` in `pipeline/mod.rs`. For `binding_kind == "mfd"`
  the render target aspect is taken from the frame canvas (`canvas_guid`, M_MFD_Screen 800×600 →
  4:3) instead of the SWF stage bounds. Scoped to `mfd` because `physical` bindings (annunciators)
  share a frame canvas but render content at native aspect — applying it there wrongly reshaped the
  annunciator (1920×1595 → 432), proving the scope boundary. Pending: visual confirm via re-export.
- **#4 BLOCKED (font)**: renderer bundles only DejaVu Sans/Mono (`text/mod.rs` `include_bytes!`) and
  renders ALL text in DejaVu regardless of the authored font record. The game font is
  `BuildingBlocks_FontStyle.BlenderPro-Thin` → `UI/fonts/Install/BlenderPro-Thin.slug` (Slug GPU
  font). Blender Pro is **not present in the P4k** as TTF/OTF and is a commercial font, so the exact
  "G"/letterforms cannot be matched without sourcing the font. Needs a user decision (bundle a
  licensed/lookalike font + font-record→file mapping, vs accept the DejaVu substitute). A lookalike
  would change all UI text and shift gold/platinum baselines.
- **#3 footer (large, generic)**: `M_Eng_MFDContent` (reached via M_MFD_Screen → canvas_MFDContent)
  statically embeds `canvas_PortraitMFDView`→target, `canvas_LandscapeMFDView`→self,
  `canvas_incomingCallOverride`, and `canvas_Header / Footer`→gen_mc_s_header. All views default
  `isActive:true, alpha:1.0`; the engine picks one via `aspectRatioLibrary` (aspectratiototag_mfd)
  + "Content Canvas Scaling" style conditions, and the page fades in from `base_Root alpha:0.0`.
  Faithful generic footer therefore needs: (a) render/compose the frame chrome (footer) for mfd
  bindings, (b) resolve the page-in alpha to settled visible, (c) aspect-ratio-tag view selection
  so only the bound content view shows, (d) screen-name label resolution ("TARGET STATUS" — source
  still to be located; gen_mc_s_header's text_ScreenName is the placeholder @ui_leaderboards_Loadout).
  This is a multi-step feature; sequence it after #1/#4 are confirmed.

## Pre-existing issue noticed (not from this pass)
- `manifest_targets_visual_regression_guard` fails: `eng_annunciator_master_left` reference
  (1920×1595, stale on-disk) vs current artifact (1920×432). Both predate this session; the 432
  artifact was generated before my edits and the annunciator is `physical` (untouched by the mfd
  aspect rule). Likely a side-effect of the prior D5 SWF change or simply stale artifacts. Flag to
  user; resolve via artifact regeneration / D5 review, separate from these 4 issues.

## Round 2 findings (after user feedback)

The target MFD is a **hybrid Flash + BB** screen:
- `MC_S_Target_Master` scene = `base_Root` + `canvas_TargetStatus` (rendererType **Flash**, `canvas:null`).
  Its `defaultStyles` modifier sets the canvas ref to **`gen_mc_s_target.json`** (per-brand variants for
  mrai/rsi/grin; DRAK has none → default). So the BB content the pipeline renders is `gen_mc_s_target`.
- `gen_mc_s_target` renderer_hint = `hybrid`. It holds "NO TARGET" (`text_NoTarget`, blenderpro-thin →
  `$Text1Thin`) and target-info fields (FACTION/VELOCITY/HAIL, all `is_active:false` in the no-target
  state). No "TARGET STATUS" footer text exists here.

### Font — reference is FURORE (SWF Flash layer), proven by side-by-side render
Rendered "NO TARGET" with each `fonts_en.swf` symbol via the production `draw_swf_font` path:
`$Furore` matches the reference letterforms exactly (squared, notched corners on N/A/G/R);
`$Text1Thin`/`$Text1Book`/`$Text1Med` (Blender Pro) are humanist/rounded and do NOT match.
The BB `text_NoTarget` faithfully resolves to `$Text1Thin` (blenderpro-thin); the shared style
`MFD_G_TargetStatus` has no font modifiers and DRAK has no brand override on this canvas — so there
is **no BB/style data path to Furore**. In-game the Furore "NO TARGET" is the SWF Flash widget's own
text (`TargetStatus.swf` imports `$Furore`). Matching it requires rendering SWF Flash text content,
which the renderer deliberately skips (D5 stage-skip). → needs a Flash-text-rendering feature.

### Font (#1, older note) — confirmed working from SWF; residual diff is BB-vs-Flash content
- `SB_UI_FONT_TELEMETRY=1` shows `text_NoTarget`: requested `$Text1Thin`, selected `$Text1Thin`,
  source `resolved-record-symbol`, **swf_used=true**. So the BB text already renders the authored
  Blender Pro Thin glyphs extracted from the SWF (via `fonts_en.gfx`/`fonts_en.swf`, merged in
  `load_first_swf`). Letterforms now match Blender Pro.
- The reference "NO TARGET" is heavier/squared = **Furore**, which `TargetStatus.swf` imports
  (`$Furore`) and uses for its own status text. i.e. the in-game text is the **SWF Flash** widget's
  text, not the BB placeholder. Matching it requires rendering the SWF Flash text content (currently
  skipped along with AS-driven stage content). This is a hybrid-Flash-rendering feature, not a
  font-extraction bug.

### Footer (#2) — frame chrome, not content
- The "< TARGET STATUS >" bar (3 cards: prev `<`, screen-name, next `>`, each with a top line) is
  `gen_mc_s_header`, hosted by the **frame** `m_eng_mfdcontent` (`canvas_Header / Footer`), NOT by the
  content canvas. The bottom of the generated image is genuinely empty (verified by 4× brightened
  crop). Requires the frame-composition feature (render frame chrome + page-in alpha + screen-name).

### Annunciator (#3) — FIXED
- Root cause: prior D5 change unified the annunciator ship-subdir order to `brand_ship_subdirs`
  (DRAK_Dragonfly first). Dragonfly's `AnnunciatorHalve1.swf` stage is 143×143 (square, visual aspect
  ~0.83 → 1920×1595, too tall). Buccaneer's is 364×82 (aspect 0.225 → 1920×432, the correct thin
  strip). Added `annunciator_ship_subdirs` (Buccaneer-first for DRA) so the annunciator falls back to
  the strip-shaped SWF while support screens keep Dragonfly-first. Re-export → annunciator ~1920×432.

## Validation
- `cargo test -p starbreaker-ui` (+ manifest snapshot/live-ir/visual regression suites).
- Re-export: `cargo run -p starbreaker --release -- entity export "drak_clipper" "~/projects/scorg_tools/ships" --kind decomposed --lod 0 --mip 0 --materials all`
- View generated PNG vs reference directly (Read tool), not pixel diff.
