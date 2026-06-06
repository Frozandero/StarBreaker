# Plan: Faithful Flash (SWF) hybrid UI rendering + MFD frame composition

Status: Phases 0–7 implemented (2026-06-05). First live `drak_clipper` export +
visual inspection found **three regressions** (blank MFDs, stretched annunciator,
regression-suite blind spot) — root-caused below; fixes are **Phases 8–10**
(open). Do **not** roll back features to fix these — the new features are sound;
the bugs are in production wiring, the alpha-inheritance pass, and test coverage.
Owner: (assign)
Goal: render BuildingBlocks (BB) UI screens that use Flash/Scaleform SWF content
the way the game does, generically for **any ship / any manufacturer / any
screen**, with **no hard-coding and no heuristics**. This closes the two open
DRAK Clipper target-MFD gaps (the "NO TARGET" font and the "TARGET STATUS"
footer) and, more importantly, makes every hybrid screen render correctly.

Read first (in order): `StarBreaker/AGENTS.md`,
`StarBreaker/.github/copilot-instructions.md`,
`StarBreaker/crates/starbreaker-ui/AGENTS.md`,
`StarBreaker/crates/starbreaker-ui/docs/ui-matching-workflow.md`, and the sibling
`clipper-target-mfd-plan.md` (the investigation that produced this plan). The
recalled memory note `clipper-mfd-hybrid-flash-architecture` summarises the data
model.

---

## ⚠️ DATA-SOURCE AUTHORITY — READ BEFORE ANY RESEARCH

**Live DataCore and the P4K archive are the single source of truth.** Always
verify facts with the MCP `datacore_record` / `datacore_query` / `p4k_*` tools
(or the in-game reference capture).

**The `ui_*` MCP tools (`ui_ir_query`, `ui_canvas_style_inventory`,
`ui_scene_style_probe`) are NOT ground truth.** They compile from a **local,
decompiled copy** of the records (`../ships/dcb_canvas/...`) and from StarBreaker's
own pipeline, so they show **derived data that can be stale or already
transformed by the code under investigation**. Use them only to see *what the
pipeline currently produces*, never to establish *what the game authored*.

Concrete lesson from this work: while debugging a separate medical-screen tint
bug, `ui_ir_query` reported a shape's tint as `null` — but **live DataCore showed
it was authored `"Accent1"`**. Trusting the IR tool there would have sent the fix
in the wrong direction. When the IR tool and DataCore disagree, **DataCore wins**,
and the disagreement itself is often the bug.

---

## 0. Approved decisions (from the user — do not re-litigate)

1. **Fidelity = static template + named-state frames.** Render the SWF's
   display list for the *correct state*, selecting a named timeline frame
   (`FrameLabel`) / sprite frame when the SWF exposes one; otherwise the default
   (frame 0). No ActionScript VM. Dynamic live data (real target name, live
   gauges) is out of scope; the *static reference state* (e.g. the "no target"
   state) must match.
   NOTE (verified, see §1b / Phase 3): many SWFs — including the target screen —
   have **no frame labels**; their states are AS-driven nested-sprite visibility.
   So "named-state frames" is one mechanism among two: frame-label selection where
   present, **BB-resolved-state-driven sprite selection** otherwise. Do not assume
   frame labels exist.
2. **SWF wins, BB is fallback.** For a widget whose `rendererType == "Flash"`,
   the SWF presentation is authoritative (its graphics + its fonts). The
   BB-native render of that widget's subtree is the fallback used only when no
   SWF resolves. (So "NO TARGET" renders in the SWF's Furore, not the BB
   `blenderpro-thin`.)
3. **Deterministic SWF resolution.** Replace today's multi-path candidate
   probing (and the hard-coded `brand_ship_subdirs` / `annunciator_ship_subdirs`
   lists) with a resolver derived from game data + the P4K asset tree. No
   hard-coded ship lists.

---

## 1. Verified game-data facts (already confirmed via MCP — don't re-derive)

Re-verify with the MCP **P4K**/**datacore** tools (ground truth); the `ui_*` tools
are derived/decompiled data — see the DATA-SOURCE AUTHORITY callout above.

- **Binding** (`scene.json`, helper `Screen_Right_Upper_RTT`, kind `mfd`):
  `canvas_guid` = `33bda02c…` (`M_MFD_Screen`, 800×600 = 4:3 frame),
  `content_canvas_guid` = `b8d2d65c…` (`MC_S_Target_Master`, 1920×1080 = 16:9).
- **Frame chain**: `M_MFD_Screen` → `canvas_MFDContent` (WidgetCanvas) →
  `m_eng_mfdcontent.json` (800×600). `m_eng_mfdcontent` statically embeds:
  `canvas_PortraitMFDView`→`mc_s_target_master`,
  `canvas_LandscapeMFDView`→`mc_s_self_master`, `canvas_incomingCallOverride`,
  and `canvas_Header / Footer`→`gen_mc_s_header` (the footer, 11% height,
  bottom-anchored, pivot/anchor y=1.0). All three content views default
  `isActive:true, alpha:1.0`; the engine shows one via `aspectRatioLibrary`
  (`aspectratiototag_mfd.json`) + "Content Canvas Scaling" style conditions.
  `m_eng_mfdcontent`'s root `base_Root` has authored `alpha:0.0` with no
  animation timeline (a runtime page-in start state).
- **Content canvas** `MC_S_Target_Master` scene = `base_Root` +
  `canvas_TargetStatus` (**`rendererType: "Flash"`, `canvas: null`**). Its
  `defaultStyles` modifier sets the canvas ref to `gen_mc_s_target.json`
  (per-brand variants `mrai/rsi/grin`; DRAK → default). `gen_mc_s_target` holds
  the BB `text_NoTarget` ("NO TARGET", `blenderpro-thin` → `$Text1Thin`) and
  target-info fields (FACTION/VELOCITY/HAIL, `is_active:false` in no-target
  state). `renderer_hint = hybrid`.
- **SWF reference is NOT explicit.** `vehicle_screen_mfd`'s
  `UIBuildingBlocksEntityComponentParams` references only the canvas GUID; the
  Flash widget carries only a `BuildingBlocks_FlashRendererPolicy` (no path).
  SWFs live by convention at
  `Data\UI\ShipInterface\assets\SWF\<BRAND>\[<ShipSubdir>\]<ScreenSet>\<File>.swf`,
  e.g. `…\DRA\DRAK_Dragonfly\Support_Bespoke_2\TargetStatus.swf`. The Clipper
  has **no own SWF dir** and reuses another DRAK ship's set.
- **Font fact**: the reference "NO TARGET" is **Furore** (proven by rendering
  every `fonts_en.swf` symbol through `draw_swf_font`; `$Furore` matches, Blender
  Pro variants do not). `TargetStatus.swf` *imports* `$Furore`+`$OrbitronLight`
  from `…/fonts/Shared/fonts_en.swf` and renders its own text; it defines **no**
  fonts itself. The shared font library is already merged at render time
  (`load_first_swf` merges `fonts_en.gfx` and follows `ImportAssets`).
- **The "orange bar"** (D5): a stage element that appeared when stage frame 0 was
  drawn. The current code (`swf_render/stage.rs::draw_swf_visual_exports`) now
  **skips all stage-frame-0 shapes** and the hybrid path
  (`hybrid_compose.rs`) applies **no** SWF stage/visual-export overlay at all —
  only per-node `WidgetCustomShape` symbols (`draw_swf_symbol`) and SWF fonts
  (`draw_swf_font`) are used. This is why Furore text + footer are missing.
- **`swf` crate 0.2** exposes `Tag::FrameLabel` (43), `DefineSprite`,
  `PlaceObject(2/3/4)`, `ShowFrame`, `DefineEditText`, `DefineText`.
- **CRITICAL (verified by probing `TargetStatus.swf`)**: this SWF has **1
  main-timeline frame and NO frame labels**. Its main timeline places a single
  document sprite (`id=27`). States are **ActionScript-driven visibility of
  nested exported sprites**, not timeline frames:
  `TargetSelection_Placeholder` (`id=23`) is the **no-target** state (the Furore
  "NO TARGET"); `TargetSelectionShip` (`id=18`) / `TargetSelectionEntity`
  (`id=20`) are the acquired states; `TargetStatus_ImageBox` (`id=2`) etc. are
  other parts. ⇒ **"named-state frames" does NOT apply here.** The achievable
  generic approach is to drive SWF content selection from the **BB-resolved
  active state** (the BB IR already resolves it — `text_NoTarget` is active and
  the target-info fields are `is_active:false` in the no-target state) and render
  the matching SWF exported sprite, treating other state sprites as hidden. Some
  other SWFs (e.g. annunciator/background) MAY use frame labels — support both
  (frame-label selection when present; BB-state-driven symbol selection
  otherwise). This reframes Phase 3 (below).

## 1b. Phase 3/4 DE-RISK FINDINGS (verified by probing `TargetStatus.swf`)

These materially simplify the work — read before Phase 3/4.

- **The no-target "NO TARGET" is STATIC text in the SWF; no ActionScript needed.**
  Sprite `id=23` (`TargetSelection_Placeholder`) places `id=22`, a
  `DefineEditText` whose `initial_text` is **HTML**:
  `<p align="center"><font face="$Furore" size="6" color="#ffffff"
  letterSpacing="0.600000">@hud_NoTarget</font></p>`.
- **Typography is specified IN the SWF EditText HTML** — `face="$Furore"`,
  `size`, `color`, `align`, `letterSpacing`. This is the authoritative,
  data-driven font source for SWF-rendered text (parse the HTML; do NOT use the
  BB font record for SWF text). `$Furore`/`$OrbitronLight` resolve via the
  already-merged shared font library.
- **Text content is a localization key** (`@hud_NoTarget`) → resolve via the
  existing loc system (`loc_fetcher`; the pipeline already resolves `@…` keys for
  BB text). `letterSpacing="0.6"` matches the reference's wide tracking.
- **State = static vs sample data.** The acquired-state sprites (`id=18`
  `TargetSelectionShip`, `id=20` `TargetSelectionEntity`) hold *sample* EditText
  ("EM", "Emissions", "Ship Name/Label", "000") that AS replaces with live data —
  meaningless in a static export. The **placeholder/no-target sprite is the
  correct static state**. The root sprite `id=27` places only AS *manager*
  objects (`ScreenContextParams`/`TargetStatusManager`/`TargetSelectionManager`,
  ids 26/25/24) — these are non-visual; the visual sprites hang off the managers.
- **Implication**: for the immediate font goal, Phase 4 can render the SWF
  EditText fields of the active/placeholder state directly (parse HTML → font +
  size + color + align + letterSpacing; resolve loc key; draw with `draw_swf_font`
  in the EditText bounds, placed by sprite matrix). No AS VM, no BB↔SWF symbol
  mapping required for this path. The probe (`swf_text_probe`) now dumps sprite
  trees, frame labels, `DefineText`, and EditText `initial_text` to repeat this
  analysis for any SWF.
- **Still to confirm during impl**: which sprite(s) constitute the
  "static/default" state generically (heuristic-free rule). Candidate rule:
  render EditText whose text is a loc-key/static literal; treat fields carrying
  obvious sample data as live (AS) and either skip or fill from BB bindings. The
  BB IR's resolved active state (e.g. `text_NoTarget` active) is the cross-check.

## 2. Current code map (what exists, what to change)

- `crates/starbreaker-ui/src/swf_assets/` — SWF parse/extract: `extract.rs`
  (fonts, shapes, bitmaps, exports, edit-text metrics), `stage.rs`
  (`extract_stage_frame`, `extract_sprite_first_frame`, `extract_stage_size`),
  `library.rs` (`SwfAssetLibrary`: caches + `merge_swf_bytes`, `content_hash`),
  `types.rs` (`PlaceRecord`, `ShapeRecord`, `FontGlyphSet`, …).
- `crates/starbreaker-ui/src/swf_render/` — rasterise: `stage.rs`
  (`draw_swf_stage`, `draw_swf_visual_exports`, `draw_swf_symbol`,
  `draw_stage_character*`), `shape.rs` (`draw_shape`, `matrix_to_dest`).
- `crates/starbreaker-ui/src/text/swf_draw.rs` — `draw_swf_font` (SWF glyph text).
- `crates/starbreaker-ui/src/pipeline/swf_selection/` — SWF path resolution:
  `flash_paths.rs` (candidate generation; **hard-coded** `brand_ship_subdirs`,
  `annunciator_ship_subdirs`), `candidates.rs` (`build_swf_selection_manifest`,
  `canvas_has_flash_renderer`), `loader.rs` (`load_first_swf`: merges Canvas.swf
  + `fonts_en.gfx`, follows imports).
- `crates/starbreaker-ui/src/ir_compose/` — BB IR → pixels (engine_parts/part_*).
  `part_04.part` draws text nodes (SWF-font selection `select_imported_ui_font`,
  `used_swf_font`). This is where SWF-vs-BB precedence + SWF content compositing
  must integrate.
- `crates/starbreaker-ui/src/hybrid_compose.rs` — `render_ui_ir_with_swf_overlay`
  (currently == `render_ui_ir_document`).
- `crates/starbreaker-ui/src/pipeline/mod.rs` — `compile_ir_for_binding`
  (`frame_canvas_aspect` for mfd 4:3), `render_for_binding_ir`.
- `crates/starbreaker-3d/src/pipeline/child_payload.rs` — binding population,
  `build_mfd_view_canvas_map` (`SMFDView`), `collect_mfd_default_canvases`
  (`SCItemSeatDashboardParams.MFDParams`).
- Diagnostics: `examples/swf_text_probe.rs` (now takes path args, prints stage
  size/aspect/fonts/exports/edit-text). `SB_UI_FONT_TELEMETRY=1` logs font
  selection per text node.

## 3. Guardrails (apply to EVERY phase)

- **Track progress IN THIS DOC.** As each TODO completes, change its `- [ ]` to
  `- [x]` and append a short note + commit hash on the same line. Update phase
  status as you go. This file is the living progress tracker — keep it current so
  any later session (or a fresh agent) can resume precisely from here. If a step's
  reality differs from the plan, edit the plan to match what you found.
- **No hard-coding / no heuristics in production code.** No ship/screen/brand
  name branches, no magic path lists, no "try many paths until one exists". Drive
  everything from DataCore fields + the P4K asset tree (enumerate, don't guess).
  Named assets are allowed only in tests/fixtures.
- **IR is the styling/positioning authority.** SWF content placement must come
  from SWF matrices + the BB widget's resolved rect; do not invent positions.
- **TDD**: write a failing test that encodes the expected behaviour first, watch
  it fail, implement, watch it pass. Unit tests use small synthetic SWFs or
  checked-in fixture SWFs (see Phase 0), never the live P4K.
- **Regression cadence**: after every phase (and before declaring any task done)
  run the required suite (Appendix C). Do **not** edit baselines to pass; fix root
  cause. Baseline/tier changes need explicit user approval (workflow doc).
- **Keep files < 500 lines**; split by responsibility; every new `.rs` starts
  with a `//!` header.
- **Performance**: parse each SWF once per export (cache by content hash / path
  in the fetcher or a per-run map); the deterministic resolver must do O(1)–O(k)
  lookups, not O(paths) probing; reuse `SwfAssetLibrary` across the bindings that
  share a SWF. Measure export wall-time before/after (Phase 7).

---

## Phase 0 — Test harness & regression safety net (do first)

Objective: be able to test SWF rendering deterministically and catch regressions.

TODO:
- [x] 0.1 Add 1–2 **tiny fixture SWFs** under
  `crates/starbreaker-ui/tests/fixtures/swf/` that exercise: a labeled frame
  (`FrameLabel`), a `DefineSprite`, a `DefineText` static string, a
  `DefineEditText`, and an `ImportAssets` font reference. Prefer generating them
  programmatically in a test helper (so they're readable/maintainable) over
  binary blobs. Document each fixture's contents in a `//!` header.
  NOTE (implemented): builders live in `tests/swf_helpers/mod.rs` rather than a
  `fixtures/swf/` subdirectory (programmatic, no binary blobs).
  `make_labeled_frames_swf()` → FrameLabel + multi-frame; `make_state_sprites_swf()` →
  DefineSprite, DefineText, DefineEditText (HTML + @hud_NoTarget), ImportAssets.
  Added `swf = "0.2"` and `tiny-skia = "0.12"` to [dev-dependencies].
- [x] 0.2 Capture the **current** required-suite results as the green baseline (Appendix C). Record the current generated dimensions of every Clipper UI image
  so later phases can detect unintended drift.
  BASELINE (2026-06-05): 373 lib tests + snapshot 11 + live-IR 4 + visual 4 all green.
  Clipper image dimensions: mc_s_target_master 1600×1200, mc_s_self_master 1600×1200,
  mc_s_power_master 1600×1200, h_eng_annunciator_master_left 1920×432,
  h_eng_annunciator_master_right 1920×432. All other screens: 1920×1080 or native
  authored size.  These must not change through Phases 1–6 without approval.
- [x] 0.3 Add a `cargo test` helper that renders a fixture SWF via the
  production path and asserts pixel/coverage invariants (non-empty regions, glyph
  bounds), to be reused by later phases.
  NOTE: `swf_helpers::assert_swf_symbol_has_non_empty_coverage` in
  `tests/swf_helpers/mod.rs`; tested via `tests/swf_rendering_fixtures.rs`
  (13 tests: parse, frame-label, export, sprite, ImportAssets, DefineText,
  DefineEditText HTML, and 3 pixel-coverage assertions).
- [x] 0.4 Confirm `examples/swf_text_probe.rs` + `SB_UI_FONT_TELEMETRY=1` are
  sufficient diagnostics; extend generically if a phase needs another signal.
  CONFIRMED: probe covers FrameLabel, DefineSprite trees, DefineText, DefineEditText
  HTML initial_text, ImportAssets, ExportAssets, font defs. No changes needed.

Validation: suite green; fixtures parse; helper renders.

## Phase 1 — Deterministic SWF resolution (remove the heuristic)

Objective: given a binding (canvas + manufacturer + ship/dashboard context),
resolve the **exact** SWF path(s) deterministically from data, with a
data-derived fallback when a ship has no own assets. Remove hard-coded ship
lists.

Research TODO (use MCP P4K + datacore):
- [x] 1.1 Find the config that selects a ship's UI skin / screen set.
  FINDING: `SCItemSeatDashboardScreen.Style.Type` maps to P4K subdir for
  some brands (AEG: `MFD_16_9`, `MFD_4_3`, `Annunciator`) but DRA ships only
  list `HeadUpDisplay` — no support-screen style link in DataCore for DRA.
  `SMFDView.urlpostfix` links view to SWF filename (e.g. `"targetstatus"` →
  `TargetStatus.swf`). DRAK Clipper has **no own SWF directory** in P4K.
- [x] 1.2 Path grammar confirmed: `SWF\{BRAND}\[{ship}\]{screen-set}\{file}.swf`.
  BRAND = first 3 chars of manufacturer_id, uppercase (drak→DRA). Ship subdirs
  start with the brand prefix (DRAK_*, AEGS_*, etc.). Screen files derived from
  canvas name (MC_S_Target → TargetStatus, annunciator → AnnunciatorHalve{1,2}).
- [x] 1.3 Fallback rule: enumerate P4K `SWF\{BRAND}\` alphabetically; generate
  candidates for all ship subdirs whose name starts with the brand prefix;
  first valid SWF wins.  Alphabetical ordering naturally picks DRAK_Buccaneer
  first (thin annunciator strip) and Dragonfly for support screens that
  Buccaneer lacks (e.g. Support_Bespoke_2).

Implementation TODO:
- [x] 1.4 Added `list_swf_dirs` default method to `SwfFetcher` trait
  (`pipeline/mod.rs`). In `flash_paths.rs`, `p4k_ship_subdirs` enumerates
  `SWF\{BRAND}\` via the fetcher and filters by brand prefix, sorted
  lexicographically — replaces both hard-coded lists.
- [x] 1.5 Deleted `brand_ship_subdirs` and `annunciator_ship_subdirs`;
  `support_screen_candidates_for_brand` and `annunciator_swf_candidates` now
  call `p4k_ship_subdirs(brand, fetcher)`.  All signatures thread
  `fetcher: &dyn SwfFetcher` through.
- [ ] 1.6 Thread the ship/skin context from `child_payload.rs` into the binding.
  DEFERRED (skin-context preference) — but ⚠️ **see Phase 8**: the enumeration this
  phase depends on (`SwfFetcher::list_swf_dirs`) was **never implemented on the
  production `P4kSwfFetcher`**, so in the live export the resolver returns no
  ship-subdir SWFs at all (annunciator stretched, `TargetStatus.swf` unreachable).
  Phase 1's tests passed only because `MockDirFetcher` implements `list_swf_dirs`.
  Production wiring is Phase 8 (required, not optional).

Tests (TDD):
- [x] `no_ship_dirs_produces_only_brand_level_candidates` — empty fetcher → no DRAK_* candidates.
- [x] `p4k_enumeration_excludes_dirs_not_returned_by_fetcher` — only listed dirs appear.
- [x] `ship_dir_candidates_appear_in_alphabetical_order` — reverse fetcher order → sorted.
- [x] `annunciator_candidates_enumerate_ship_dirs_alphabetically` — Buccaneer before Dragonfly.
- [x] `new_ship_dir_from_fetcher_produces_candidates` — previously-unknown ships supported.
- [x] `flash_candidates_no_panic_on_empty_manufacturer` — no panic, empty result.

Validation: 379 lib tests green; required suite (snapshot 11, live-IR 4, visual 4) green.
Committed 2026-06-05.

## Phase 2 — SWF content rendering core (re-enable + correct the display list)

Objective: a correct, generic SWF display-list rasteriser for a chosen frame:
walk `PlaceObject(2/3/4)` at the target frame, resolve characters
(shapes/sprites/text/bitmaps) recursively with composed matrices, color
transforms, depth order, and clipping. This generalises the existing
`draw_stage_character_depth` and removes the blanket "skip stage shapes".

TODO:
- [x] 2.1 Added `clip_depth: Option<Depth>` to `PlaceRecord` in `swf_assets/types.rs`;
  both extraction functions in `swf_assets/stage.rs` now populate it from
  `PlaceObject.clip_depth`.
- [x] 2.2 Rewrote `swf_render/stage.rs`: single `draw_character` function replaces
  `draw_stage_character_depth` + `sprite_origin_in_dest` + `apply_place_matrix_to_dest`.
  Matrices are now COMPOSED as the tree is descended (`compose_matrix`), so scaled/rotated
  parents correctly affect all their children.  Color transform applied at each leaf
  shape.  `MAX_SPRITE_DEPTH` raised from 4→8.  Clip mask rendering deferred (field stored,
  no masking yet — no clip-depth content in the target screens).
- [x] 2.3 `draw_swf_symbol` now uses stage-size scaling (sw/sh from `assets.stage_size()`)
  so symbols are positioned stage-relatively, not stretched to fill dest.  Fallback to
  dest size when stage header is degenerate.
- [x] 2.4 Cycle detection via `visited: HashSet<CharacterId>` on the call stack — stops
  immediately on self-reference.  `MAX_SPRITE_DEPTH` = 8 as belt-and-suspenders backup.

Tests (TDD):
- [x] `doubly_nested_sprite_renders_inner_shape` — 2-level sprite tree renders shape.
- [x] `scaled_outer_sprite_composes_matrix_with_inner` — 2× scale parent, pixel at (30,30).
- [x] `scaled_outer_sprite_does_not_extend_beyond_composed_bounds` — pixel (50,50) transparent.
- [x] `self_referential_sprite_does_not_panic` — cycle handled, returns false.

Validation: 379 lib + all integration tests green 2026-06-05.  No visual change in pipeline
output yet (Phase 5 wires this in).

## Phase 3 — State selection (BB-state-driven; fixes the orange bar)

Objective: render only the SWF content for the **active state**, so the orange
bar (a different-state / always-placed element) is not shown and the no-target
content is. NOTE (verified): `TargetStatus.swf` has **no frame labels** — its
states are AS-driven nested-sprite visibility. So selection is primarily
**BB-state-driven symbol selection**, with frame-label selection as a secondary
path for SWFs that do use labels.

Research TODO:
- [x] 3.1 Parse `FrameLabel` into a `label → frame_index` map for the main
  timeline. Added `extract_main_timeline_labels` in `swf_assets/extract.rs`;
  `SwfAssetLibrary` stores the map as `frame_labels` field (populated in `new()`)
  and exposes `frame_label_index(label) -> Option<u32>`.  Sprite-level labels
  deferred (no need identified; target SWF has none).
- [x] 3.2 BB-state → SWF-content mapping research complete.
  FINDING: there is **no direct name link** between BB node names
  (`text_NoTarget`, `canvas_TargetStatus`) and SWF export names
  (`TargetSelection_Placeholder`).  Neither `mc_s_target_master.json` nor
  `gen_mc_s_target.json` contains any reference to SWF symbol names.
  `visualState` is `null` for all nodes.  The mapping must be derived from SWF
  content:
  - A sprite whose EditText `initial_text` contains an `@loc` key is a **static
    placeholder** state (render it).
  - A sprite whose EditText holds sample data (no `@` prefix, e.g. "Ship
    Name/Label", "000") is a **live/dynamic** state driven by AS at runtime
    (suppress it in static renders).
  - A sprite with no EditText is always-visible graphical content (render it).
  This rule is data-driven (reads SWF EditText content), not hard-coded.
  Implementation lives in Phase 4/5 (needs the EditText parser built in Phase 4).
- [x] 3.3 Mutually exclusive state groups: the `TargetSelection_*` exports
  (`_Placeholder`, `Ship`, `Entity`) are at the same depth in the document
  sprite's display list, placed by the same parent.  The generic grouping rule:
  exports placed at the same depth in the same sprite frame are mutually
  exclusive states.  The @loc vs sample-data content rule (3.2) identifies which
  to render without needing to enumerate depths.  No code needed for Phase 3 —
  the suppression API (`draw_swf_symbol_excluding`) lets Phase 5 pass the
  suppressed set computed from the editorial rule above.

Implementation TODO:
- [x] 3.4 Added `draw_swf_symbol_excluding(pixmap, assets, symbol, suppressed,
  dest, tint, alpha)` in `swf_render/stage.rs`.  `suppressed: &HashSet<CharacterId>`
  is propagated through `draw_character`; any character whose ID is in the set
  (and its full subtree) is silently skipped.  All existing callers pass an empty
  set so behaviour is unchanged.
- [x] 3.5 Added `draw_swf_at_frame_label(pixmap, assets, label, dest, tint, alpha)`
  in `swf_render/stage.rs`.  Looks up the frame index via
  `SwfAssetLibrary::frame_label_index`, then calls the shared
  `draw_stage_at_frame` helper.  Returns `false` for unknown labels.

Tests (TDD):
- [x] `frame_label_state_a_renders_red_shape` — state_a frame → red pixels at (50,50).
- [x] `frame_label_state_a_suppresses_state_b_green` — state_a frame → no green.
- [x] `frame_label_state_b_renders_green_shape` — state_b frame → green pixels.
- [x] `frame_label_unknown_returns_false` — unknown label returns false, empty pixmap.
- [x] `draw_without_exclusion_renders_all_states` — empty suppress set, all 3 shapes visible.
- [x] `suppress_state_b_hides_blue_pixels` — suppress StateB_Content → (10,45) transparent.
- [x] `suppress_state_b_keeps_orange_and_state_a` — suppress StateB → orange + red still visible.
- [x] `suppress_state_a_keeps_orange_and_state_b` — suppress StateA → (45,10) transparent.

Validation: 379 lib + all integration tests green 2026-06-05.  No visual change in
pipeline output yet (the @loc/sample-data suppression logic and Phase 4 EditText
rendering are the remaining pieces; full visual fix lands in Phase 5).
Committed 2026-06-05.

## Phase 4 — SWF text rendering (the Furore "NO TARGET")

Objective: render SWF text in SWF fonts with the SWF-specified typography. Per the
verified findings (§1b), the **primary** path for the target screen is **static**:
a `DefineEditText` whose `initial_text` is Flash-HTML carrying the font, size,
colour, alignment, letter spacing, and a **localization key** — render it directly
(parse HTML → resolve loc key → draw). No ActionScript and no BB↔SWF mapping are
needed for this. A **secondary** path covers genuinely dynamic fields (an EditText
whose text is AS-filled sample data, e.g. live target name); those are out of
scope for the static reference state — skip them, or optionally fill from a BB
binding later. Static `DefineText` glyph runs are a third, lower-priority path.

Research TODO (mostly answered — see §1b):
- [x] 4.1 ANSWERED for the target screen: "NO TARGET" is a `DefineEditText`
  (`id=22`) whose `initial_text` is HTML carrying the font (`$Furore`), size,
  color, align, `letterSpacing`, and a loc key (`@hud_NoTarget`). Confirm
  `@hud_NoTarget` resolves to "NO TARGET" via the loc system, and check a couple
  of other hybrid SWFs to confirm the HTML-`initial_text` pattern generalises.

Implementation TODO:
- [x] 4.2 Parse the EditText `initial_text` **Flash-HTML** fragment with the
  workspace's existing **`quick-xml` 0.37** dependency. Implemented as
  `parse_swf_html` in `swf_render/edit_text.rs`. Handles `<p align=…>` and
  `<font face/size/color/letterSpacing>`. Multiple runs, entity decoding via
  quick-xml, tolerant of unknown elements.
- [x] 4.3 `draw_edit_text` in `swf_render/edit_text.rs` renders a `DefineEditText`
  into a `Pixmap`. `loc_fn: &dyn Fn(&str) -> Option<String>` parameter threads
  from public entry points down through `draw_character`. Uses EditText bounds
  transformed through the placement matrix + stage→dest scale. Font resolved by
  HTML face name (strip `$` → `find_font_by_name`) or by font_id fallback.
  `composite_rgba_over_pixmap` added to blend text output into main Pixmap.
- [ ] 4.4 Also support static `DefineText` glyph runs (some SWFs use them).
  Lower priority — the target screen uses EditText. Deferred to Phase 7.
- [x] 4.5 Imported fonts resolved by HTML `face` attribute (strip leading `$` →
  `SwfAssetLibrary::find_font_by_name`). Phase 5 will pass `fonts_en` bytes via
  `merge_swf_bytes` before rendering.

Tests (TDD): 6 tests in `tests/swf_edittext_render.rs`. HTML parser (3 cases),
loc-key detection, EditText extraction from fixture, stage rendering produces
non-zero pixels. All pass. Suite: 379 lib + all integration tests green.

Committed: `28dff435e` — 2026-06-05.

## Phase 5 — SWF-wins / BB-fallback precedence (wire into hybrid output)

Objective: for `rendererType == "Flash"` widgets, render the resolved SWF content
(Phases 2–4) in place of the BB-native subtree; fall back to BB only when no SWF
resolves. Compose at the widget's resolved rect.

TODO:
- [x] 5.1 Added `is_flash_renderer: bool` to `UiIrNode` (populated from
  `rendererType == "Flash"` in `push_ui_ir_node.part`); `state_select.rs` implements
  the data-driven `compute_sample_data_export_ids` suppression rule.
- [x] 5.2 `render_ui_ir_with_swf_overlay` in `hybrid_compose.rs`: collects Flash
  node IDs, removes their BB subtrees via `collect_subtree_ids` (BFS), renders the
  reduced BB document, then composites the SWF stage at each Flash node's
  `computed_rect` using `draw_swf_stage_rgba_in_rect` + sample-data suppression.
- [x] 5.3 Non-Flash nodes unaffected; BB path unchanged for them.
- [x] 5.4 `render_ui_ir_with_swf_overlay` fully rewritten (was a no-op shim).
  `draw_swf_stage_with_state` added to `swf_render/mod.rs`; `state_select` exposed
  as `pub mod`. `loc_fn: &dyn Fn(&str) -> Option<String>` threaded through.

Tests (TDD): 5 tests in `tests/swf_phase5_wiring.rs` — all pass (commit `16f40b71e`).

Validation: 379 lib + all integration tests green; committed 2026-06-05.

## Phase 6 — MFD frame composition (the "< TARGET STATUS >" footer)

Objective: render the screen the way the engine does — the **frame** chrome
(`gen_mc_s_header` footer) appears with the content. Generic for any MFD.

Research TODO:
- [x] 6.1 Screen name source confirmed: `SMFDView.name` field carries the loc key
  (e.g. `@ui_MFD_View_TargetStatus`). Populated into `UiBindingView.screen_name_loc_key`
  in `child_payload.rs` (from `SCItemSeatDashboardParams.MFDParams` → `SMFDView`).

Implementation TODO:
- [x] 6.2 `compile_ir_for_binding` in `pipeline/mod.rs` now uses the frame canvas
  (`canvas_guid`) for `binding_kind == "mfd"` when it differs from `content_canvas_guid`.
  Post-processing: `base_Root` alpha=0.0 patched to 1.0 (BB animation start-state);
  `text_ScreenName` nodes receive the resolved screen name from `screen_name_loc_key`.
  `UiBindingView` gains `screen_name_loc_key: Option<&str>`. **Fully wired**:
  `build_mfd_view_canvas_map` (child_payload.rs) reads `SMFDView.name`
  (e.g. `@ui_MFD_View_TargetStatus`) and threads it through `UiBinding.screen_name_loc_key`
  → `UiBindingView` → IR. Verified: `MFD_View_Target_Status.name == "@ui_MFD_View_TargetStatus"`.
- [x] 6.3 Footer renders via the existing BB path as part of the frame canvas —
  no separate composite step needed.

Tests (TDD): 5 tests in `tests/pipeline_mfd_frame.rs` — all pass (commit `1d7418f54`).

Validation: 379 lib + all integration tests green; committed 2026-06-05.
⚠️ **INCOMPLETE — see Phase 9.** The synthetic tests passed but the live export
renders **blank**: 6.2's `base_Root` alpha patch runs after `inheritsAlpha` is
already baked (no-op), and the active-view selection bullet was never implemented.
The footer screen-name wiring is correct (`text_ScreenName` → "Target Status") but
invisible until the alpha-inheritance fix (Phase 9.4) lands.

## Phase 7 — Performance, cleanup, baseline refresh

TODO:
- [x] 7.0 Deferred items reviewed: Phase 1.6 (skin context threading) and Phase 4.4
  (DefineText glyph runs) remain deferred — both have adequate fallbacks and are not
  blocking the primary goal. Phases 5 and 6 confirmed complete (see above).
- [x] 7.1 Sprite first-frames are now parsed once at `SwfAssetLibrary::new`
  (`extract_all_sprite_first_frames`) and cached, instead of re-decompressing/parsing
  the whole SWF on every recursive `draw_character` call and every
  `compute_sample_data_export_ids` node. Exact behavioural parity (cache built from
  the primary SWF bytes, matching the prior `&self.raw` re-parse). Stage-frame and
  stage-size extraction remain per-call (low frequency). Formal wall-time
  before/after measurement still TODO if a perf regression is suspected.
- [x] 7.2 `//!` headers updated in `pipeline/mod.rs`, `hybrid_compose.rs`,
  `swf_render/mod.rs`. All modified files under 500 lines; no dead code/shims found.
- [x] 7.3 `docs/ui-fallback-register.md` updated (SWF path-probing retired in Phase 1).
  Plan doc updated to mark Phases 5 and 6 complete.
- [ ] 7.4 With **explicit user approval**, refresh gold/platinum baselines and the
  IR snapshot freeze for the intentionally-changed screens (4:3 MFDs, thin
  annunciator, Furore text, footer) per the workflow doc's onboarding steps
  (`add_ui_regression_target.sh`, `freeze_ui_snapshot_ir.sh`,
  `freeze_ui_regression_artifacts.sh`, validate scripts). Do not commit PNGs.

---

## Post-Phase-7 export validation — THREE REGRESSIONS FOUND (2026-06-05)

The first real `entity export drak_clipper` + visual inspection of the generated
PNGs (the validation Phases 1/6 deferred) revealed three distinct, independent
regressions. All three are **fixable without rolling back any feature** — the
new features (deterministic resolver, frame-canvas footer) are sound; the bugs
are in how they connect to the production fetcher, the alpha-inheritance pass,
and the regression harness. Phases 8–10 below fix each root cause. Evidence and
exact code locations are recorded so a fresh agent can act directly.

### Root Cause A — production SWF resolution is dead (`list_swf_dirs` never implemented)

- **Symptom 1**: `h_eng_annunciator_master_left.png` is now **1920×1080** (16:9,
  stretched ~2.5× tall); the gold baseline is **1920×432** (thin strip, aspect
  4.44). The panels (PWR/WPN/THR/SHLD/COOL) render but at the wrong aspect.
- **Symptom 2**: the target MFD never gets the Furore "NO TARGET" SWF text
  (`TargetStatus.swf` is unreachable), independent of Root Cause B.
- **Mechanism**: Phase 1 replaced the hard-coded `*_ship_subdirs` lists with
  P4K enumeration via `SwfFetcher::list_swf_dirs`. The trait default
  (`pipeline/mod.rs:81`) returns `vec![]`. The **production** fetcher
  `P4kSwfFetcher` (`crates/starbreaker-3d/src/ui_pipeline.rs:117`) overrides only
  `fetch_swf_bytes` — it does **not** override `list_swf_dirs`. So
  `p4k_ship_subdirs` (`flash_paths.rs:168`) gets an empty list, no ship-subdir
  candidates are generated, and every ship-subdir SWF is unfindable. The
  annunciator's SWF aspect override in `compile_ir_for_binding` (the
  `selected_swf_source` → `stage_visual_bounds` branch) is then skipped, so the
  raster falls back to the authored 1920×1080 instead of the SWF-derived
  1920×432. IR confirms `selected_swf_source: None`, `renderer_hint: bb`.
- **Ground truth (P4K, verified)**: the SWFs exist exactly where the resolver
  would look if enumeration worked —
  `Data\UI\ShipInterface\assets\SWF\DRA\DRAK_Buccaneer\AnnunciatorScreen\AnnunciatorHalve1.swf`
  and `…\DRA\DRAK_Dragonfly\Support_Bespoke_2\TargetStatus.swf`. Ship subdirs
  present under `SWF\DRA\`: `DRAK_Buccaneer`, `DRAK_Caterpillar`, `DRAK_Dragonfly`.
- This is Phase 1.6 (marked "DEFERRED") — but it is **not** optional: the whole
  Phase-1 resolver is inert in production without it.

### Root Cause B — frame-canvas MFD render is blank (alpha inheritance + no view selection)

- **Symptom**: `mc_s_target_master.png` (and self/power) are blank — the dark
  Drake background only. The NO TARGET text, the dashed lines, and the `>>`/`<<`
  chevrons (all BB content that rendered **before** Phase 6) are gone, and the
  `< TARGET STATUS >` footer never appears. Reference:
  `reference/in-game/Clipper/Screen_Right_Upper_RTT.png`.
- **Mechanism B1 (alpha inheritance — the dominant cause)**: Phase 6 renders the
  **frame** canvas `m_eng_mfdcontent`. Its root `base_Root` is authored
  `alpha: 0.0` (a page-in start state; it carries an `animation` block with
  `animationTimeline: null`, `duration: 1.0`, `additive: true`) and
  `inheritsAlpha: true`. The content views and footer are authored `alpha: 1.0`
  **but also `inheritsAlpha: true`**, so `effective_alpha_for_node`
  (`ui_ir/engine_parts/part_10.part:321`, called during IR compilation) multiplies
  every descendant by `base_Root`'s `local_alpha_for_node` = 0.0 → **every node
  bakes to effective alpha 0.0**. Verified: authored `canvas_PortraitMFDView`,
  `canvas_Header / Footer`, `base_content` = alpha 1.0, but the compiled IR shows
  them at 0.0. The Phase 6 `base_Root` patch in `compile_ir_for_binding` sets the
  `base_Root` *node's* IR alpha to 1.0 **after** inheritance is already baked into
  the children, so it is a no-op for the descendants. (Animation-sample percent is
  irrelevant: `base_Root.animationTimeline` is null, so sampling at 0% vs 100%
  produces identical alphas — confirmed empirically.)
- **Mechanism B2 (active-view selection — secondary)**: the frame embeds
  `canvas_PortraitMFDView` (→ `mc_s_target_master`, the target view),
  `canvas_LandscapeMFDView` (→ `mc_s_self_master`), and an incoming-call overlay.
  The engine activates exactly one via `aspectRatioLibrary` + "Content Canvas
  Scaling" style conditions. Our pipeline does not select by the binding's
  `content_canvas_guid`: the IR shows `canvas_PortraitMFDView` `active=False`
  while `canvas_LandscapeMFDView` `active=True` — i.e. the wrong view is "active"
  (and both are invisible anyway under B1). Phase 6.2's "select the single active
  content view" bullet was never implemented.
- **Mechanism B3 (separate bug)**: forcing the old content-canvas path
  (`use_frame_canvas = false`) fails IR validation with
  `node 1 references missing child 3` — a pre-existing structural defect in the
  standalone `mc_s_target_master` compile that the frame-canvas switch masked.
  Must be understood and fixed so the content view renders correctly *inside* the
  frame too.

### Root Cause C — regression coverage has two blind spots (refined 2026-06-05)

- **Symptom**: a gold-standard image changing 1920×432 → 1920×1080, and three
  MFDs going fully blank, produced **zero** failing tests during the Phase 1–7
  commits.
- **Finding 1 — the visual guard works but is decoupled from the build.**
  `tests/manifest_visual_regression.rs::manifest_targets_whole_image_colour_regression_guard`
  **does** compare a freshly-exported `ships/Data/UI/Generated/...png` against a
  frozen baseline and **does** catch dimension drift — it correctly fired
  `eng_annunciator_master_left: dimension drift baseline=(1920,432)
  render=(1920,1080)` the moment the broken export refreshed `ships/`. The gap:
  it reads **pre-exported artifacts** from the workspace `ships/` tree, which is
  **not** regenerated by the test. The Phase 1–7 commits never re-exported, so the
  guard kept comparing *stale-correct* PNGs and stayed green. It is only as fresh
  as the last manual `entity export`; nothing in the build/CI loop refreshes it.
- **Finding 2 — the MFDs have no whole-image coverage at all.** The manifest has
  exactly four visual targets (`ui_target_a`/`_b` = medical bed/end,
  `clipper_small_door`, `eng_annunciator_master_left`). The MFD screens
  (`mc_s_target_master` etc.) are **not** targets, so even a fresh export of the
  blank MFDs trips no whole-image guard.
- **Finding 3 — the IR guards are SWF/asset-blind.** `tests/manifest_live_ir_guard.rs`
  and `tests/ui_ir_representative.rs` compile IR from a local decompiled canvas
  tree using `DummySwfFetcher` (`list_swf_dirs` → empty default) +
  `DummyAssetFetcher`. They validate IR-structure stability against synthetic
  inputs and can never see SWF resolution, real assets, or rendered dimensions.
- Net: the annunciator drift *was* catchable (Finding 1) but the loop never ran
  the export; the blank MFD was *not* catchable by any existing guard (Findings
  2+3). Phase 10 closes both: wire a build-coupled export+dimension/coverage
  guard and add the MFDs as targets.

### Corrections to earlier plan claims

- §1 was **right** that the content views default `isActive:true, alpha:1.0` and
  that `base_Root` is authored `alpha:0.0` with a null animation timeline. The bug
  is purely in how our IR pass bakes `inheritsAlpha` from the page-in-start root.
- Phase 6.2's note "Footer renders via the existing BB path … no separate
  composite step needed" is only true *after* B1 + B2 are fixed.

---

## Phase 8 — Restore production SWF resolution (implement `list_swf_dirs`)

Objective: make the Phase-1 deterministic resolver actually work in the live
export by enumerating P4K SWF directories from the production fetcher. Fixes the
annunciator aspect regression and re-enables ship-subdir SWFs (incl.
`TargetStatus.swf`) for every ship — generically, no hard-coding.

Research TODO:
- [x] 8.1 Confirmed: `MappedP4k` exposes `entries()` (`&[P4kEntry]`, `entry.name`
  is a native `\`-separated `String`) and even a `list_subdirs(dir_path)` binary
  search — but the latter is **case-sensitive** and P4K casing is not guaranteed
  to match the resolver's uppercase-brand prefix (the reason `fetch_swf_bytes`
  uses `eq_ignore_ascii_case`). So enumeration must be case-insensitive. P4K
  confirms `SWF\DRA\` contains `DRAK_Buccaneer`, `DRAK_Caterpillar`,
  `DRAK_Dragonfly`, `RadarScreen`.

Implementation TODO:
- [x] 8.2 Added `swf_immediate_subdirs(names, prefix)` (pure, case-insensitive,
  deduped+sorted via `BTreeSet`, skips direct files) and overrode
  `list_swf_dirs` on `P4kSwfFetcher` to call it over `entries()`
  (`crates/starbreaker-3d/src/ui_pipeline.rs`). O(entries) per call; memoise later
  if profiling shows it.
- [x] 8.3 Phase 1.6 marked done (this was its missing half — see Phase 1). The
  only production fetcher is `P4kSwfFetcher`; the example diagnostic fetchers
  don't drive the export.

Tests (TDD):
- [x] 8.4 `swf_immediate_subdirs_lists_ship_dirs_case_insensitively` (+ empty
  case): lower-cased prefix vs mixed-case entries → returns the three `DRAK_*`/
  `RadarScreen` dirs sorted, excludes the brand-level direct file and other
  brands. In `ui_pipeline/tests.rs`.
- [x] 8.5 Covered by the existing Phase-1 `flash_paths` tests (MockDirFetcher),
  which assert `annunciator_swf_candidates` / support-screen candidates produce
  the `DRAK_Buccaneer\…\AnnunciatorHalve1.swf` and
  `DRAK_Dragonfly\Support_Bespoke_2\TargetStatus.swf` paths from the enumerated
  dirs — the exact shape `swf_immediate_subdirs` now produces in production.

Validation: re-rendered the annunciator from the live export — now **1920×432**
(ar 4.444), `selected_swf_source =
…\DRA\DRAK_Buccaneer\AnnunciatorScreen\AnnunciatorHalve1.swf`, and the strip
renders un-stretched (verified via Read tool). NOTE: the target MFD's
`selected_swf_source` is still `None` because it compiles the **frame** canvas
`M_MFD_Screen`, which does not itself reference `TargetStatus.swf` — that SWF is
on the embedded content view's Flash node and only resolves once Phase 9 routes
frame→active-content→Flash. 379 ui-lib + 373 3d-lib tests green (2 new).

## Phase 9 — Fix the blank MFD frame render (settled page-in alpha + active-view selection)

Objective: make the frame-canvas MFD render show the **active** content view
(NO TARGET, dashes, chevrons) **and** the footer (`< TARGET STATUS >`), matching
the reference, while keeping genuinely-hidden overlays (incoming-call, warnings,
low-power) hidden. Generic for any MFD; no per-asset gating.

Research TODO:
- [x] 9.1 Signature pinned and implemented (`settle_pagein_start_roots`): scene
  root + `alpha==0` + `isActive` + non-null `animation` block. Confirmed against
  `m_eng_mfdcontent.base_Root`; excludes `background_Primitive` (isActive:false).
  Done at parse time (not the IR layer) because the merge re-parents these roots.
- [x] 9.2 Rule determined and implemented: match the binding's content canvas
  `_RecordName_` to the embedded `canvas_*MFDView` whose `canvas:` reference equals
  it. Applied at **resolution time** via `mfd_view::apply_bound_view_instantiation`
  (peers = same-parent + same-layer slots), threaded through `resolve_canvas_graph`.
- [x] 9.3 Moot — the frame path resolves the content correctly (Pass 1 follows
  `mc_s_target_master`'s `defaultStyles` → `gen_mc_s_target` → `text_NoTarget`). The
  `missing child 3` error only appeared when force-rendering the content canvas
  *standalone*, which the frame approach never does, so no fix is needed there.

Implementation TODO:
- [x] 9.4 Page-in alpha settled at the SOURCE — but at **parse time**
  (`bb_scene::parse::settle_pagein_start_roots`), not in `local_alpha_for_node`.
  Reason: in the merged frame scene `base_Root` is re-parented under its
  `WidgetCanvas` and is no longer a scene root, so the IR-layer root check missed
  it (it passed a single-canvas unit test but failed the real merged frame — a
  Root-Cause-C-in-miniature). Settling per-canvas at parse, where the page-in node
  is still a root, fixes the real cascade. Signature: scene root + alpha==0 +
  isActive + page-in `animation`. Removed the ineffective `base_Root` name patch.
  Commit `57e25256f`. Verified: alpha dist flipped {1.0:12,0.0:209} → {1.0:193,0.0:6}.
- [x] 9.5 Active-view selection done at **resolution time** (not post-resolution):
  the frame's state filter put the bound view's `WidgetCanvas` slot in
  `instantiated_false` (runtime view-selector boolean has no static default), so
  Pass 2 **skipped merging the content entirely** — a post-resolution deactivate
  re-activated an empty slot. Fix: thread the bound content canvas `_RecordName_`
  through `resolve_canvas_graph` (new `..._with_loc_and_bound_view`);
  `mfd_view::apply_bound_view_instantiation` forces the matching slot instantiated
  and its mutually-exclusive peers (same parent+layer) out, before Pass 2. Commit.
  Verified: target MFD now renders **NO TARGET + dashed lines + >>/<< chevrons +
  "Target Status" footer** — matches the reference's core content.
- [ ] 9.6 Furore SWF overlay (TargetStatus.swf) routing into the embedded Flash
  node (`canvas_TargetStatus`) inside the frame — not yet done. The frame compiles
  `selected_swf_source: None` (the frame canvas doesn't itself reference the SWF;
  it's on the embedded content view's Flash node). The BB fallback text_NoTarget
  renders now (blenderpro-thin, not Furore); routing the SWF upgrades it to Furore.
- [x] 9.10 **Boolean/integer-gated overlay suppression — DONE** (the major
  blocker). Extended `bb_state_filter::eval::eval_bool_ref` to resolve the
  integer-state op-types to their at-rest values instead of `eval=None`:
  - `BooleanFromIntegerSwitch` → its authored `defaultValue` (at rest the integer
    is not in `exceptions`) — hides CallingState etc.
  - `BooleanFromInteger` → when **both operands resolve statically** (an
    `IntegerComponentParameter` carries an authored `defaultValue`) the **real
    comparison** is computed (`Equal`/`NotEqual`/`Greater`/`GreaterOrEqual`/`Less`/
    `LessOrEqual`; a wired `inputR` overrides the inline `value` literal). When an
    operand is a runtime `IntegerVariable` binding (no static default — the
    frame's `powerstate`/`criticalWarningState` gates) it falls back to the
    at-rest heuristic: `Equal value` is **false** / `NotEqual value` is **true**
    for ANY value (at rest no specific integer state-value is active). This both
    hides event overlays (`countdown == 5` → false) AND keeps the frame's own
    `Invert(powerstate == 0)` true (the screen we render is **on**). Ordered
    comparisons on the heuristic path stay unresolved (conservative). Resolution
    logic factored into `bb_state_filter::integer`. (Finding B, see 9.12.)
  - `BooleanEvaluateAnd`/`Or` → short-circuit on a determining resolved operand
    (any `false`→And `false`; any `true`→Or `true`) instead of bailing to `None`
    when one operand is unresolved.
  NOTE: a blanket "unknown → hidden" flip was tried and **proven wrong** (it blanks
  the screen — content is also gated by unresolved bindings) and reverted; and a
  naive "integer = cold 0" assumption was **proven wrong** (it made
  `powerstate == 0` true → dropped the whole frame). The "no specific integer
  state-value at rest" rule above is the correct generic interpretation.
  Verified live: the incoming-call signal bar, countdown, and call overlays are
  **gone**; NO TARGET + dashes + chevrons + footer survive. **Regression-safe**:
  the shared-evaluator change passed `manifest_visual_regression` 4/4 (medical/
  door/annunciator unchanged) + 384 lib tests. TDD: `integer_state_ops_resolve_to_at_rest_values`.
- [ ] 9.11 **Footer state-tag visibility — the last remaining overlay.** The
  footer (`gen_mc_s_header`) has parallel name cards — `card_ScreenName` (normal,
  gated by `Invert(ComponentParameter ParamInput2)` → shown), `card_ScreenName_LowPower`,
  `card_ScreenName_Warning`. The latter two carry **no** boolean `IsActive`/`Instantiated`
  gate; their visibility is driven by `BindingsStringField` **state tags**
  (`PrimaryStateTag`/`SecondaryStateTag`/`TertiaryStateTag`) feeding BB **style
  conditions** — a different mechanism from boolean bindings. So they render at
  rest ("LOW POWER" / "LOADOUT" overlapping the screen name). FIX (next): evaluate
  the state-tag → style-condition visibility for the at-rest state (the normal card
  wins; the alert cards' state tags don't match) — i.e. extend the style-condition
  evaluator to gate visibility, generic across screens. Everything else matches the
  reference.

- [x] 9.12 **Phase 8/9 code-review follow-ups — DONE.** Three findings + one nit
  from the review of the Phase 8/9 code, each fixed TDD-first (failing test → fix
  → green) with no MFD regression:
  - **Finding A — evaluator consistency.** The two boolean evaluators
    (`eval_bool_ref` recursive; `evaluate_bool_ops` iterative) had diverged on the
    integer/short-circuit op-types. Both now share the same op handling.
  - **Finding B — static integer resolution.** `BooleanFromInteger` resolved by a
    blunt `Equal→false`/`NotEqual→true` heuristic even when the integer was
    statically known. Now factored into `bb_state_filter::integer`
    (`resolve_static_integer` + `eval_bool_from_integer`): an
    `IntegerComponentParameter` default yields the **real comparison** (all six
    operators; wired `inputR` overrides the inline `value`), while runtime
    `IntegerVariable` bindings keep the heuristic. Confirmed against live data: the
    frame's gates are all `IntegerVariable` (heuristic, unchanged); only the header
    uses `IntegerComponentParameter`, and its one `IsActive`-gating op resolves to
    the same `false` — the other feeds only an unimplemented `PrimaryStateTag`
    (9.11), so **zero visibility change**. TDD:
    `from_integer_with_component_parameter_resolves_real_comparison`.
  - **Finding C — page-in raw/field consistency.** `settle_pagein_start_roots` set
    the parsed `alpha` field to 1.0 but left `raw["alpha"]` at the 0.0 start value;
    now settles both so a later re-derivation from `raw` can't reintroduce the
    blank. TDD: `pagein_start_root_settles_alpha_in_field_and_raw`.
  - **Nit — empty-prefix guard.** `swf_immediate_subdirs("")` would have enumerated
    the whole archive's top-level dirs; guarded to return empty. TDD:
    `swf_immediate_subdirs_empty_prefix_returns_nothing`.
  Verified: 387 ui-lib + ui_pipeline tests, `manifest_visual_regression` 4/4, both
  `line_count_guard`s green; re-render of all 43 Clipper bindings (new release
  binary) = 0 failures; the target MFD render is unchanged (NO TARGET + dashes +
  chevrons + footer, LOADOUT overlap still pending 9.11).

Tests (TDD):
- [x] 9.7 `compile_ir_settles_pagein_start_root_alpha` (in `pagein_alpha_tests.part`):
  page-in root settles descendants to 1.0; a no-animation alpha=0 root stays 0.0.
- [x] 9.8 `mfd_view` tests: `apply_bound_view_instantiation` forces the bound slot
  on and its same-parent/same-layer peers off, leaving the footer (other layer).
- [x] 9.9 B3 (`missing child 3`) — N/A: the frame path resolves the content
  correctly (the error only appeared when force-rendering the content canvas
  standalone, which the frame approach doesn't do). New eval test:
  `integer_state_ops_resolve_to_at_rest_values` covers the 9.10 op-types.

Validation: target MFD shows NO TARGET + dashes + chevrons + footer (✅, matches
reference core). REMAINING before "done": overlay suppression (9.10) and the
Furore SWF (9.6); then self/power MFDs.

## Phase 10 — Close the regression blind spots (build-coupled + MFD coverage)

Objective: ensure a future SWF-resolution break, dimension drift, or blank render
is caught automatically — not by manual visual inspection months later. The
existing `manifest_visual_regression` guard already detects dimension/colour drift
**when fed a fresh export**; the gaps are (1) nothing refreshes the export in the
loop, (2) the MFDs aren't targets, (3) the production `list_swf_dirs` wiring has
no unit guard. Address all three.

Research TODO:
- [x] 10.1 Tiers decided from the post-export findings: (a) a pure-unit guard on
  the production `list_swf_dirs` wiring (cheap, no P4K — would have caught Root
  Cause A directly); (b) add the MFD screens as `manifest_visual_regression`
  targets so blank/again-correct MFDs are guarded; (c) make the live guard
  *build-coupled* — an opt-in test gated on `SC_DATA_P4K` that **renders fresh**
  (not from stale `ships/`) and asserts dimensions + non-blank coverage, so it
  cannot pass on stale-correct artifacts.

Implementation TODO:
- [ ] 10.2 Unit guard (a): assert the production `P4kSwfFetcher` does not use the
  empty `list_swf_dirs` default — e.g. a focused test on `swf_immediate_subdirs`
  plus a wiring assertion that `P4kSwfFetcher`'s impl is non-default. (The
  `swf_immediate_subdirs` tests added in Phase 8.4 are the core of this.)
- [ ] 10.3 Coverage (b): add `mc_s_target_master` / `_self_` / `_power_` (and the
  right annunciator halves) as visual-regression targets with frozen baselines
  (needs user approval for the baselines — see 7.4), so the blank-MFD class of
  bug trips a guard.
- [ ] 10.4 Build-coupled live guard (c): an `SC_DATA_P4K`-gated test that renders
  the Clipper MFD + annunciator helpers **live** and asserts (i) annunciator
  aspect ≈ 4.44, (ii) target MFD non-blank (distinct-colour ratio, reuse
  `tests/visual_diff.rs`), (iii) `selected_swf_source` is `Some` where expected.
  Skips cleanly without the archive.
- [ ] 10.5 Update `docs/ui-regression-policy.md`: document that the
  `manifest_visual_regression` guard only protects what's been **re-exported**,
  that the `Dummy*Fetcher` IR guards are SWF/asset-blind, and that the live guard
  (10.4) is the contract for SWF resolution + real dimensions.

Validation: 10.2 fails on a tree where `P4kSwfFetcher` lacks `list_swf_dirs`
(Root Cause A), passes after Phase 8; 10.3/10.4 pass post-Phase-8/9 and fail if
either regresses, *without* needing a manual re-export first.

---

## Appendix A — Risks & open questions to resolve during research (not blockers)

- **State selection (Phase 3) is the central feasibility risk.** Confirmed: the
  target SWF has no frame labels — states are AS-driven nested-sprite visibility.
  We do not run ActionScript, so we must infer the active state from the BB IR
  (which resolves it) and map it to the SWF sprite to show. If a robust generic
  BB→SWF-symbol link cannot be established (3.2), fall back to: render the
  exported sprite whose name/`SymbolClass` best matches the active BB subtree, and
  if even that is ambiguous, render the BB-native subtree (current behaviour) for
  that widget rather than risk drawing the wrong/overlapping state (the orange
  bar). Document whichever rule is used; never hard-code symbol names.
- **BB-node ↔ SWF-text-field mapping (4.1)**: prefer instance-name/`SymbolClass`
  linkage; fall back to position only if no name link exists, and document it.
- **Frame-render vs footer-composite (6.2)**: frame-render is faithful but must
  not regress the 3 working MFD screens; gate behind the required suite and
  per-screen visual checks; keep content-only as the proven fallback until the
  frame render is validated.
- **`.gfx` vs `.swf` font glyphs**: the shared lib is loaded as `.gfx`; if any
  glyph fidelity issue appears, prefer the `.swf` outlines (both exist in P4K).

## Appendix B — Done = all true

- Target MFD matches the reference: 4:3, Furore "NO TARGET", no orange bar,
  "< TARGET STATUS >" footer; chevrons/dashes intact.
- self/power MFDs and the annunciator still correct; no other screen regressed.
  (Annunciator back to 1920×432 via Phase 8; MFDs non-blank via Phase 9.)
- No hard-coded ship/screen/brand/path lists in production; SWF resolution is
  data-driven **and live in production** (`list_swf_dirs` implemented — Phase 8).
- Required suite green; baselines refreshed only with approval; export time not
  meaningfully worse.
- A live-export guard (Phase 10) catches future SWF-resolution, dimension, and
  blank-render regressions — the synthetic IR guards alone are insufficient.

## Appendix C — Required validation commands (run every phase)

```bash
cd StarBreaker
cargo test -p starbreaker-ui --lib
cargo test -p starbreaker-ui --test manifest_snapshot_regression -- --nocapture
cargo test -p starbreaker-ui --test manifest_live_ir_guard -- --nocapture
cargo test -p starbreaker-ui --test manifest_visual_regression -- --nocapture
cargo test -p starbreaker-ui --tests        # broad crate check
# Re-export + view (accuracy is judged against the reference, via the Read tool).
# Build the release binary once, then export with --kind decomposed (a directory
# export root; do NOT use the default bundled kind here — it treats the output as
# a file and errors on a directory):
cargo build --release -p starbreaker
SC_DATA_P4K="$HOME/Games/star-citizen/drive_c/Program Files/Roberts Space Industries/StarCitizen/LIVE/Data.p4k" \
  ./target/release/starbreaker entity export drak_clipper \
  "$(cd .. && pwd)/ships" --kind decomposed --lod 0 --mip 0 --materials all
```
Reference: `reference/in-game/Clipper/Screen_Right_Upper_RTT.png`.
Generated: `ships/Data/UI/Generated/ship/drak/Clipper/buildingblocks_canvas_mc_s_target_master.png`.
Diagnostics: `examples/swf_text_probe.rs <p4k\path.swf>`; `SB_UI_FONT_TELEMETRY=1`.
