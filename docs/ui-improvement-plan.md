# UI improvement plan — tooling, truth mining, text calibration, cascade unification

## HANDOFF — start here (written 2026-06-12 for a fresh session)

You are working in `/home/tom/projects/scorg_tools/StarBreaker` (branch
`feature/ui`), crate `starbreaker-ui`. Your job is to execute THIS plan,
phases top-down (P0 → P6; see §Sequencing for the recommended order and
which phases may be reordered).

Read, in order, BEFORE any work:
1. `StarBreaker/AGENTS.md` (note the hard-coding ban + self-correction rule)
2. `crates/starbreaker-ui/AGENTS.md` (core rules; the guards)
3. `docs/ui-workflow.md` (THE process: TDD, guard adjudication §5, freezes §7)
4. `docs/ui-reference.md` (commands, probes, data locations, screen dossier)
5. This file, fully — including §"Verified facts" (do not re-derive them)
   and §"Current state" below.

Working discipline:
- Work one checkbox at a time; mark it `[x] (<date> <commit>)` IN THIS
  FILE in the same commit as the work. Cite the plan step (e.g. "plan
  P1.2") in every commit message.
- `bash scripts/ui_check.sh` green per commit; `--full` at each phase
  boundary. Renders: `bash scripts/ui_render.sh --helper <name> --scene
  "/home/tom/projects/scorg_tools/ships/Packages/DRAK Clipper_LOD0_TEX0/scene.json"`
  (LOD0 = cockpit MFDs; default LOD1 = medical/door/annunciator).
- A platinum/gold guard tripping is the system working — adjudicate per
  `docs/ui-workflow.md` §5 against the reference captures in
  `/home/tom/projects/scorg_tools/reference/in-game/Clipper/`. NEVER
  compare against `ships/Data/UI/Generated/*.png` without a fresh export
  first (~50s; the stale-export trap is ledger item 20 — P0 makes it loud).
- Steps marked APPROVAL-GATED stop and present deltas to Tom; everything
  else proceeds autonomously.
- If a step's premise turns out wrong, record what was falsified against
  the checkbox (a miss is a result), fix the plan text, and continue.

### Current state (end of the 2026-06-12 session, HEAD ≈ 5b0c25435)

Tree clean, `ui_check.sh --full` ALL GREEN (513 lib tests; 5 frozen
targets; font harness 26/26). Landed that session (don't redo): the
RgbaColor hard-coding guard + brand-palette fixture (`src/test_palettes.rs`);
colour tokens aligned to the BB_ColorStyle enum (`src/style/colour_roles.rs`);
the tag-conditioned TEXT-FORMAT style route (Parent-wrapped entries style
a textfield's text format; brand `s_*` containers only; `__EntryFontSize`
outranks the named-style table ONLY via the route — commit 07c821a83);
the MFD host-path `imageSizePercent` division for ALL size classes
(commit 15d1e3b99) — power/target text now matches the references; eight
empirically-deleted hard-coded rules + three proven load-bearing (P4.4/P5
carry the survivors); audited IR + artifact re-freezes (approver tom).
Arc history/diagnoses: `docs/ui-clipper-parity-handoff.md` §13–§16;
process ledger: `docs/ui-process-improvements.md` (Part D = this plan's
findings).

Open power-screen parity items NOT in this plan (separate parity arc;
diagnoses in the handoff): P3 separator dots (parked, full diagnosis +
two blockers in handoff §12), P4 pip slab brightness (rides the gated
linear-light question, handoff §10), P13 header side bars, P8 footer
letter pitch. P7 (scrollbar slider width) IS in this plan (P2.2 item b).

---

The implementation plan for `docs/ui-process-improvements.md` Part D (items
19–27) merged with the architecture recommendations from the 2026-06-12
review and the three derivation arcs left open by the completed
hard-coding remediation plan (removed by 7a69b7112 "Plan completed";
evidence record: `docs/ui-process-improvements.md` Part D + git history). Written to be executed from fresh context by a less capable
agent: every step names its files, commands, and acceptance check.

Ground rules (non-negotiable, from `docs/ui-workflow.md` +
`crates/starbreaker-ui/AGENTS.md`):
- Behaviour-affecting changes go through TDD + the guard battery
  (`bash scripts/ui_check.sh` per commit; `--full` at phase boundaries).
- Baseline changes ONLY via the audited freeze flow with approver+reason
  (`docs/ui-workflow.md` §7); Phase 3 is explicitly approval-gated.
- No hard-coded game values; the self-correction rule applies.
- Mark each checkbox `[x] (<date> <commit>)` when done, in this file.
- One commit per coherent item, message citing the plan step (e.g.
  "plan P1.2") and/or ledger item number.

Verified facts this plan relies on (re-verify only if something fails):
- Full Clipper export ≈ 50s: `cargo run -p starbreaker --release -- entity
  export drak_clipper /home/tom/projects/scorg_tools/ships --kind decomposed`.
- `BuildingBlocks_root.swf` is SWF v8 / AVM1: 127 `DoInitAction` tags, no
  DoABC. Extract: `./target/release/starbreaker p4k extract --output
  /tmp/bbswf --filter "**/BuildingBlocks_root.swf"`.
- The vendored `swf = "0.2"` crate exposes `swf::avm1::types::{Action,
  ConstantPool, Push}` (and `avm1` readers) — no new dependency needed.
- Ruffle/Flash PLAYBACK is a proven dead end — do not propose it
  (`docs/ui-architecture-runbook.md` §"Why Ruffle/Flash playback is a dead
  end").
- References are manual, skewed captures in
  `/home/tom/projects/scorg_tools/reference/in-game/Clipper/`; there is no
  bulk-capture option.

---

## Phase 0 — guardrails and battery hardening (small, do first)

Goal: the failure modes that produced wrong verdicts this arc become loud.

- [x] (2026-06-12, commit "plan P0.1") **P0.1 Export stamp.** In the UI export path (the code that writes
      `ships/Data/UI/Generated/ship/<mfr>/<Ship>/*.png` — find it via
      `grep -rn "Generated" crates/starbreaker-3d/src/ui_pipeline/`), also
      write `ships/Data/UI/Generated/.export_stamp.json` containing
      `{ "written_at_epoch_s": <now>, "git_describe": "<git describe
      --always --dirty>", "binary_built_at_epoch_s": <mtime of
      current_exe> }`. Use `std::process::Command` for git (fall back to
      "unknown" on error — the stamp must never fail an export).
      Acceptance: run a full export; the stamp exists and parses.
- [x] (2026-06-12, commit "plan P0.2") **P0.2 Staleness hard-fail in the visual guard.** In
      `crates/starbreaker-ui/tests/manifest_visual_regression.rs`
      (`manifest_targets_whole_image_colour_regression_guard`), before
      comparing: read the stamp; FAIL (not skip) with a distinct message
      ("STALE EXPORT: Generated PNGs predate the current build — re-export
      (~50s) before artifact comparison") when (a) the stamp is missing,
      or (b) any compared `source_generated_png` mtime < stamp
      `written_at_epoch_s` − 60, or (c) the test binary's own
      `std::env::current_exe()` mtime is NEWER than the stamp by more than
      30 minutes. Escape hatch: env `UI_ALLOW_STALE_EXPORT=1` downgrades
      to an eprintln warning (needed for repo-only CI without game data —
      keep the existing missing-data skip path untouched).
      Acceptance: touch the test binary's mtime forward / delete the
      stamp → guard fails with the new message; after a fresh export it
      passes. Cite ledger item 20 in the commit.
- [x] (2026-06-12, commit "plan P0.3") **P0.3 Examples join the battery.**
      Found+fixed in the same change: `render_phase2_comparison.rs` had
      bit-rotted (missing `colour_overlay_enabled`) AND carried copied drak
      palette literals — colours neutralised, and the RgbaColor hardcoding
      guard now scans `examples/` too (the discovered missed category). Add `cargo check -p
      starbreaker-ui --examples` to `scripts/ui_check.sh` (TDD tier,
      before the test runs). Acceptance: introduce a deliberate
      compile error in `crates/starbreaker-ui/examples/dump_ui_ir_targets.rs`,
      see `ui_check.sh` fail, revert. Cite ledger item 26.
- [x] (2026-06-12, commit "plan P0.4") **P0.4 Probe channel consistency.** Change `BB_A3_STYLE_PROBE`'s
      output in `crates/starbreaker-ui/src/bb_brand_apply/mod.rs` from
      `log::info!` to `eprintln!` (matching `BB_TEXT_FORMAT_PROBE`), and
      annotate the probe-registry table in `docs/ui-reference.md` §6 with
      the output channel for every probe. Acceptance: `BB_A3_STYLE_PROBE=1
      bash scripts/ui_render.sh --helper Screen_Annunciator_L` prints
      probe lines WITHOUT `RUST_LOG` set. Cite ledger item 25.
- [x] (2026-06-12, commit "plan P0.5") **P0.5 Battery green.** `bash scripts/ui_check.sh --full` after the
      phase; no baseline changes expected (P0 must not alter rendering).
      ALL GREEN, no baseline changes.

## Phase 1 — measurement tooling and the measurement bank

Goal: pixel adjudication becomes a tool call + a lookup, not throwaway
python. (Ledger items 19, 21, 23.)

- [x] (2026-06-12, commit "plan P1.1") **P1.1 `scripts/ui_ir_query.py`.** Promote the session's /tmp
      helpers: input = an IR JSON produced by `ui render --dump-ir-dir`
      (see `docs/ui-reference.md` §6). Subcommands:
      `query <ir.json> <regex> [--fields a,b,c]` — print id, parent,
      node_type, name, computed_rect, is_active + requested fields
      (matching node name OR text_payload.text);
      `tree <ir.json> <node_id>` — ancestor chain with rects,
      authored_size, anchor/pivot, padding, margin.
      Keep it dependency-free (stdlib json/re only). Add both commands to
      `docs/ui-reference.md` §7 (diagnostics table) — run each once
      (verify-on-write). Acceptance: `python3 scripts/ui_ir_query.py query
      /tmp/<any>.ir.json 'text_'` lists textfields.
- [x] (2026-06-12, commit "plan P1.2") **P1.2 `scripts/ui_measure.py` — the contamination-guarded
      measurer.** (Default delta 45; suspect = run crossing BOTH side
      edges or abutting top/bottom — single-edge contact is reported in
      `touches` without the flag, since the reference P-glyph itself
      nudges one box edge.) Inputs: an image + an element box (`--box x0,y0,x1,y1`
      or `--ir <ir.json> --node <id>` to derive the box from the IR rect).
      Outputs (JSON to stdout):
      - `glyph_runs`: bright-row runs inside the box (threshold = median +
        configurable delta), each `{y0,y1,h}` — the CALLER picks the glyph
        run; the tool flags runs touching the box edge as
        `"suspect_contamination": true` (the footer-bar-line trap, ledger
        19);
      - `cap_height`: tallest non-suspect run;
      - `colour`: mean RGB + R-normalised ratios of above-threshold pixels;
      - `--anchor x0,y0,x1,y1`: also measure an anchor region and emit the
        ADDITIVE-haze-corrected ratios (model: measured_ratio ≈ true_ratio
        + haze_offset, solved from the anchor whose true colour is given
        via `--anchor-rgb r,g,b` — document the model in the script
        docstring; it is the refined form of `docs/ui-reference.md` §3's
        photometric method).
      Acceptance: reproduce two known numbers — power footer P-glyph cap
      52 on `Screen_Left_Lower_RTT.png` (box around x430-470, y1085-1190;
      the bar line at y1097-1103 must be flagged suspect, glyph 1126-1177)
      and our render's footer cap ≈53 on a fresh
      `/tmp/.../Screen_Left_Lower_RTT_TEX0.png`.
- [x] (2026-06-12, commit "plan P1.3") **P1.3 Rectification.** `scripts/ui_compare.py --rectify
      <corners.json>`: corners.json holds the capture's four screen-corner
      pixel coordinates (`{"tl":[x,y],"tr":..,"br":..,"bl":..}`), stored
      NEXT TO the reference image as `<reference>.corners.json`. Compute
      the homography to the render's rectangle (pure-python DLT, 8×8
      linear solve — numpy allowed, no OpenCV) and warp (bilinear) before
      region cropping. When the corners file exists, `ui_compare.py` uses
      it automatically and prints "rectified via <file>". Acceptance: a
      synthetic test — warp a render with a known homography, write
      corners, `--rectify` recovers it (mean abs pixel error < 3 on the
      glyph regions). Add a one-line how-to (how to pick corners in GIMP:
      pointer coordinates of the screen bezel corners) to
      `docs/ui-reference.md` §3. Cite ledger item 21.
- [x] (2026-06-12, commit "plan P1.4") **P1.4 Measurement bank.**
      All boxes re-measured with ui_measure.py; 10 of 13 arc numbers
      reproduced exactly. Three battery-card values did NOT (zeros 51
      vs arc 53, slash 55 vs 58, OFFLINE 49 vs 43 — the 43 was already
      "unexplained" in the handoff): bank records the tool-reproduced
      values flagged SOFT, with the arc numbers in the notes. New fixture
      `crates/starbreaker-ui/tests/fixtures/ui_ir/reference_measurements_v1.json`
      + `.notes.md` (provenance: capture file, date, method=ui_measure.py,
      rectified or not). Schema per entry: `{ "capture": "<file>",
      "element": "<free-form id, e.g. power.footer.P-glyph>", "metric":
      "cap_height_px|colour_ratio|rect", "value": <num|array>, "box":
      [x0,y0,x1,y1], "anchor": <optional>, "notes": "<traps>" }`. Seed it
      with this arc's settled numbers (power: footer cap 52, IR 53,
      3.5K 54, 294.1 55, OUTPUT 41, "2" 93, "/16" 58, 0/0 53/58,
      OFFLINE 43, ºC 37; target: NO TARGET 77@1959w, footer 64@1959w —
      note the 1959×1513 capture scale). Adjudications consult the bank
      FIRST (note this in `docs/ui-workflow.md` §4). No validator needed
      (references can't be machine-checked) — the notes file carries the
      audit trail.
- [ ] **P1.5 Battery green** + docs_reference_guard passes (new scripts
      are cited in docs, so the guard must see them exist).

## Phase 2 — AVM1 truth mining (replace measured framework constants)

Goal: read the `__Packages.*` AVM1 bytecode for the constants we
calibrated from captures. (Ledger item 27.)

- [ ] **P2.1 Dumper tool.** New example
      `crates/starbreaker-ui/examples/swf_avm1_dump.rs` (or a
      `starbreaker ui swf-actions` subcommand if the CLI is preferred —
      example is less plumbing). Input: an SWF path. For every
      `DoInitAction`/`DoAction` tag: parse the action stream with
      `swf::avm1::read::Reader` (the crate's avm1 reader; if the public
      reader API differs in 0.2.2, parse the raw records: opcode 0x88 =
      ConstantPool — u16 count then NUL-terminated strings; opcode 0x96 =
      Push — typed values incl. f32/f64/constant-pool refs). Emit, per
      export name (pair `DoInitAction.id` with the `ExportAssets` name):
      the constant pool strings and every numeric push with its index.
      Acceptance: running it on `/tmp/bbswf/.../BuildingBlocks_root.swf`
      lists 127 classes; grepping the output for `44` finds candidate
      pushes (the content-view inset) — record which class they're in.
- [ ] **P2.2 Mine the known constants.** Search the dump for:
      (a) the content-view placement (44 / 1192 / 676 family — expected in
      a view/layout class, e.g. a `bhvr.*` view manager) →
      if found, update `crates/starbreaker-ui/src/mfd_view.rs` doc
      comments to cite the class+offset (the VALUE stays a constant but
      its provenance becomes the bytecode, closing the register entry's
      derivation criterion);
      (b) the scrollbar slider sizing math (look in the scrollbar widget
      class for pushes like 0.5/track ratios and method-name strings like
      `setThumb`, `scrollRatio`) → derive the formula and fix the power
      P7 item (`docs/ui-clipper-parity-handoff.md` §13: ours 393 vs ref
      ~432) with a TDD'd layout change;
      (c) any text-scale / fontlib handling confirming the
      `imageSizePercent` host division (string refs to `fontLib`,
      `setTextSize` etc.) → cite in
      `crates/starbreaker-ui/src/ui_ir/engine_parts/engine_02.part`
      `apply_font_image_size_percent`'s comment.
      Each finding lands as its own commit; misses are recorded in this
      file against the checkbox (a miss is a result — it bounds where the
      constant lives, likely the C++ side).
- [ ] **P2.3 Also dump `fonts_en.swf` and the per-screen content SWFs**
      (`./target/release/starbreaker p4k extract --output /tmp/uiswf
      --filter "Data/UI/**/*.swf"` — large; filter narrower if slow) and
      check `TargetStatus.swf`-class files for layout constants used by
      the hybrid path. Record findings in
      `docs/ui-architecture-runbook.md`.
- [ ] **P2.4 Battery green**; doc updates verify-on-write.

## Phase 3 — text-calibration arc (APPROVAL-GATED re-freeze at the end)

Goal: retire `TEXT_RENDER_SIZE_CALIBRATION = 1.5`,
`LAYOUT_TEXT_MEASURE_CALIBRATION = 1.5`,
`SWF_TEXT_RENDER_SIZE_CALIBRATION = 0.84`, the caption-pair `-8.0` and
word-gap `0.33` (register entries carry their criteria). This arc
re-freezes every text baseline — get Tom's approval BEFORE starting it,
and again at the freeze.

- [ ] **P3.1 Kill the DejaVu fallback's reason to exist.** The TTF
      fallback (`crates/starbreaker-ui/src/text/mod.rs`, bundled
      DejaVuSans/Mono) renders text when no imported SWF font is
      selected. The game's fonts are ALL in `fonts_en.swf` (29
      DefineFont3 tags) and the renderer already rasterizes
      `FontGlyphSet`s (`swf_assets`/`text/swf_draw.rs`). Step: make the
      fallback path load the game fonts — when a text element has no
      selected font, select from a lazily-loaded `fonts_en.swf` glyph set
      (fetch path via the existing SwfFetcher; the font-matching helper is
      `select_imported_ui_font_from_assets` in
      `crates/starbreaker-ui/src/ir_compose/engine_parts/engine_01.part`).
      DejaVu remains ONLY as the no-game-data CI fallback. Acceptance:
      `SB_UI_FONT_DUMP=1` on the LOD1 scene shows no DejaVu families for
      any frozen-target text; font harness
      (`scripts/font_size_check.py` via `ui_check.sh --full`) deltas are
      presented to Tom (expected: small drifts — STOP for approval before
      re-capturing the TSV per `docs/ui-font-size-harness.md`).
- [ ] **P3.2 Derive the 1.5.** With game fonts on the fallback path, the
      1.5 estimate must be re-derived or die: measure (with
      `scripts/ui_measure.py`) the frozen-target texts that previously
      used DejaVu vs their references; if game-font rendering at nominal
      size matches, DELETE `TEXT_RENDER_SIZE_CALIBRATION` and
      `LAYOUT_TEXT_MEASURE_CALIBRATION` together (they must stay equal —
      measure==draw); if a factor remains, derive it from the font record
      (`ascent+|descent|` vs `units_per_em` are parsed in `swf_assets`)
      and write the formula, not a constant.
- [ ] **P3.3 Derive the 0.84.** Same procedure on the SWF-path
      calibration. Investigate the Slug lead first (the engine may
      rasterize via Terathon Slug — `docs/ui-architecture-runbook.md`
      §Ruffle note): if Ghidra access exists, confirm via RTTI strings;
      otherwise derive empirically per font record from the measurement
      bank. A surviving constant goes back to the register with the new
      evidence; a derived formula deletes it.
- [ ] **P3.4 Caption-pair stack + inline word gap.** Model the
      label→value handoff from line-box font metrics (ascent/descent of
      the two styles) instead of `-8.0`; the inline nested-textfield gap
      from the measured space-glyph advance instead of `0.33×em` (the
      space advance is already extracted — see
      `crates/starbreaker-ui/src/text/swf_draw.rs`
      `swf_space_advance_px`). Verify against medical1's
      MEDGELS→200/200 (~28px top-to-top) and T3→MEDICAL (~17px gap) from
      the measurement bank.
- [ ] **P3.5 The audited re-freeze.** Full export; present per-target
      deltas; `bash scripts/ui_freeze_cycle.sh --approver tom --reason
      "<cite every identity>"` plus
      `bash scripts/freeze_ui_snapshot_ir.sh` if IR fields moved; both
      validators; `ui_check.sh --full`; font TSV re-capture per
      `docs/ui-font-size-harness.md` (approval-gated). Update the
      register: retired entries move to Retired with evidence.

## Phase 4 — style-cascade unification (the architecture debt)

Goal: ONE selector engine; delete the accreted special passes. This is
the riskiest phase — land it as a REFACTOR with byte-identical output,
verified by the full battery at every step. (The remediation audit's
surviving name-keyed rules and ledger item 17 land here.)

- [ ] **P4.1 Inventory the passes.** Write
      `docs/ui-cascade-passes.md` documenting every current entry
      application: style-link, defaultStyles (editor-time — NOT applied),
      sharedStyles, brand container, embeddedStyles, node inlineStyles,
      widget-standard module sheets (scoped), widget-standard embedded,
      deferred late-state subtree passes, the text-format route, and the
      `__InlineFontSize`/`__EntryFontSize` markers. For each: source
      container, palette sources, scope, ordering, and the reference
      evidence that pinned it (grep `crates/starbreaker-ui/src/bb_resolve/
      engine_parts/engine_01.part` `apply_canvas_style_cascade` and
      `crates/starbreaker-ui/src/bb_brand_apply/mod.rs` — most of it is
      in comments already). Acceptance: the doc's pass list reproduces
      the probe output order of `BB_A3_STYLE_PROBE=1` on one medical and
      one power render.
- [ ] **P4.2 Selector-engine types.** New module
      `crates/starbreaker-ui/src/bb_style_engine.rs` (or directory):
      `StyleSheet { tier: Tier, identifier, palettes, entries }`,
      `Tier { StyleLink, Shared, Brand, Embedded, Inline, StandardModule,
      DeferredState }`, `Target { Widget, TextFormat }`. ONE function
      `apply(scene, &[StyleSheet])` that evaluates conditions
      (reusing `bb_brand_apply::conditions` verbatim), resolves targets
      (the text-format route = Target::TextFormat with the brand-tier
      gate), and applies modifiers in tier order with the existing marker
      semantics. Pure refactor: behaviour pinned by the existing 513 lib
      tests + live IR guard.
- [ ] **P4.3 Migrate call sites pass-by-pass** (one commit each, battery
      green each): brand pass first, then shared, then embedded, then
      deferred late-state (its origin-identifier logic becomes
      `Tier::DeferredState` + the original tier), then inline. Delete
      `apply_style_entries_filtered`'s special-casing as it empties.
- [ ] **P4.4 Re-audit the survivors on the new engine.** Re-run the
      disable→adjudicate audit (workflow §5 method, fresh export!) on:
      the `Bright` role-tag directive, `base_animatedelements`
      deactivation, the annunciator 25px frame, `RootGhost` chrome
      extraction (the new engine should let the RootGhost entry be
      evaluated against the actual node instead of plucked by name —
      a remediation-audit leftover). Delete what the engine now
      covers; keep + document what trips.

## Phase 5 — remaining structural derivations (each its own mini-arc)

- [ ] **P5.1 `base_animatedelements`** (load-bearing, ui_target_a
      draw-order pins): find the structural at-rest-hidden signal —
      compare the authored `animation` blocks of those containers vs
      animated-but-visible nodes (suspect: additive looped timelines that
      START from alpha 0 / off-screen). Replace the name match with the
      data property; battery adjudicates.
- [ ] **P5.2 Annunciator 25px frame** (`root_annunciator_items`): check
      the Phase 2 AVM1 dump first (the frame inset may be host-side like
      the 44px); else search the annunciator canvas/standards for the
      authored padding the engine applies. Replace name+magic-number;
      battery adjudicates (gold pins).
- [ ] **P5.3 Background slot 8 vs 9** (`StyleLoader::
      parse_buildingblocks_style_record` uses slot 8 for `background`;
      the enum names slot 9): ASK TOM for one capture of a screen whose
      slots 8/9 differ visibly (drak: (20,13,5) vs (38,27,10) — any MFD
      in a dark room may discriminate). Approval-gated; do not change
      without the capture.
- [ ] **P5.4 `HOST_STAGE_SIZE` from the SWF header**: when next touching
      `crates/starbreaker-ui/src/mfd_view.rs`, plumb the host stage size
      (already parsed by `SwfAssetLibrary::stage_size` in the pipeline)
      into `apply_bound_mfd_view` instead of the (1280,720) constant.
      Zero behaviour change expected; battery confirms.

## Phase 6 — documentation and bootstrap closure

- [ ] **P6.1 Runbook "engine model" entries** (ledger item 24): add to
      `docs/ui-architecture-runbook.md` short sections for (a) padding ×
      canvas geometry scale (evidence: power pip-top/stride pins), (b)
      the text-format route + literal-match precedence (T3
      counterexample, commit 07c821a83), (c) the host-path
      `imageSizePercent` division (commit 15d1e3b99, per-element
      measurements), (d) the additive-haze photometric model (now in
      `scripts/ui_measure.py`'s docstring — cross-link). Each with its
      capture/commit citations.
- [ ] **P6.2 Workflow §5: the empirical audit method** (ledger item 22):
      document disable→adjudicate with preconditions (FRESH EXPORT — cite
      the stamp guard; consult lib + live-IR + visual suites) and the
      scope caveat (proves "no frozen pin", not "correct everywhere").
- [ ] **P6.3 Dossier hygiene**: `docs/ui-reference.md` §3 — the power row
      gains its current state (text parity reached 2026-06-12; open: P3
      dots, P4 pips, P13 bars, P7 slider, P8 pitch); add a
      `reference_measurements_v1.json` pointer to §5's data table.
- [ ] **P6.4 Bootstrap dry-run** (the retro acceptance test): read ONLY
      `docs/ui-workflow.md` + `docs/ui-reference.md` + the dossier and
      walk the per-screen prompt for `Screen_Left_Lower_RTT` — every
      command/path must resolve. Any excursion is a doc bug: fix it in
      the same commit.

## Sequencing and ownership

Recommended order: P0 → P1 → P2 → (P5.1/P5.2/P5.4 opportunistically — P2
may solve P5.2) → P3 (approval-gated) → P4 → P5.3 → P6. P0+P1 are an
afternoon; P2 is a day; P3 and P4 are real arcs (plan a session each);
P6 closes whenever the rest lands. Anything discovered mid-phase that is
hard-coding gets fixed-or-flagged in the same change (self-correction
rule, AGENTS.md); anything that changes baselines stops for approval.
