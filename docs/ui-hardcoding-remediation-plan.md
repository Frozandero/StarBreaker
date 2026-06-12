# starbreaker-ui hard-coding remediation plan

Phased plan to eliminate every hard-coded game-data value from
`crates/starbreaker-ui` production code, replacing each with a
data-derived source (DataCore/P4K at run time, or a provenance-noted
extracted fixture where tests must run offline). Test fixtures are exempt
from phases 1–5 EXCEPT where they copy real game values (those load
`src/test_palettes.rs` / future fixture loaders instead).

Ground rules: `AGENTS.md` ("Hard-coded game DATA VALUES are hard-coding
too" + the self-correcting clause), `crates/starbreaker-ui/AGENTS.md`
(Core rules), `docs/ui-workflow.md` rule 1. Guards:
`crates/starbreaker-ui/tests/source_hardcoding_guards.rs`
(`rgba_colour_literals_are_not_hardcoded`,
`brand_palette_fixture_matches_live_records`) and
`scripts/check_ui_hardcoding.sh`. Every phase lands with the standard
battery (`bash scripts/ui_check.sh`, `--full` at the boundary) and
reference-capture verification for any visual change; guard trips follow
`docs/ui-workflow.md` §5.

## Phase 0 — DONE (commit 5bf1d7f84)

- Crate-wide `RgbaColor` literal guard + `hardcoding-guard: synthetic`
  annotation policy.
- `tests/fixtures/ui_ir/brand_palettes_v1.json` (+ `.notes.md`) extracted
  from `S_BIOC`/`S_DRAK_HUD`; `src/test_palettes.rs` loader; all
  real-palette test fixtures migrated.
- Invented "Drake amber" production fallback replaced by
  `StyleLoader::neutral_fallback` (white-on-black, no invented palette).
- `ir_compose` `Accent2` → BB_ColorStyle enum index 5 (was hand-mapped).

## Phase 1 — colour-token slot maps → BB_ColorStyle enum

The authoritative token→slot mapping is the `BB_ColorStyle` DataCore enum
(`docs/ui-architecture-runbook.md` §Reference). Two resolvers still carry
hand-maintained tables that diverge from it:

- [ ] `src/bb_brand_apply/colors.rs` `color_style_slot_index`: `Accent2 |
      Positive | Success → Some(1)` (enum: Accent2=5, Positive=1),
      `Accent3 | Warning → 2`, `Accent4 | Critical | Negative → 3`,
      `Bright | Base → 0` (enum: Bright=6), plus invented aliases
      (`Mid`, `Light`, `Highlight`, `Gold`…) that exist in no enum dump.
      For each divergence: find the reference capture that motivated it
      (the doc comment cites two verified ones) and either re-verify or
      align to the enum; delete aliases with no DataCore occurrence
      (grep the dcb mirror for each token string first).
- [ ] `src/ir_compose/engine_parts/engine_01.part` `resolve_colour_token`:
      same audit for the remaining aliases (`accent3/warning/moderate`,
      `accent5`, `mid`, `light`, `highlight`, `surface`, `bg`,
      `backlight`); `positive/success` still resolve via a
      primary-similarity heuristic.
- [ ] Replace both tables with ONE shared enum-indexed resolver +
      role-divergence table, each divergence carrying a reference
      citation (the per-role divergence is real — foreground vs surface —
      but must be evidence-listed, not open-coded).
- [ ] Extend the guard: ban literal `[f32; 4]`/`[u8; 4]` COLOUR arrays in
      production paths (the RgbaColor guard does not see them; e.g. the
      old `[0.0, 113.0/255.0, …]` test pins were this category).

## Phase 2 — font sizing calibration constants

`docs/ui-font-size-harness.md` guards behaviour; each constant needs a
derivation from font/record data or an explicit
`docs/ui-fallback-register.md` entry with retirement criteria:

- [ ] `src/ui_ir/engine_parts/engine_02.part:1068`
      `FIXED_BAND_HEADING_FILL = 0.381` — the 2026-06-12 power-arc work
      showed the medical banner's 40.0px output equals the AUTHORED
      mainmenu brand entry `FontSize=40` reached via the Parent-wrapped
      text-format route. Once that route lands (in-tree work, handoff
      §13), re-derive: the fallback should become unreachable → RETIRE.
- [ ] `src/ui_ir/engine_parts/engine_02.part:1058`
      `LONG_HEADING1_PROMPT_FONT_SIZE = 28.7` — same family; attempt the
      authored-entry derivation before keeping.
- [ ] `src/ir_compose/engine_parts/engine_01.part:35-36`
      `TEXT_RENDER_SIZE_CALIBRATION = 1.5`,
      `SWF_TEXT_RENDER_SIZE_CALIBRATION = 0.84` and
      `src/bb_layout/engine_parts/engine_02.part:1856`
      `LAYOUT_TEXT_MEASURE_CALIBRATION = 1.5` — derive from the actual
      font metrics (units_per_em / ascent+descent are already parsed; the
      draw-metrics measure landed for SWF fonts — extend it to the TTF
      path so the 1.5 estimate dies).
- [ ] `src/ir_compose/engine_parts/engine_01.part:1798`
      `INLINE_NESTED_TEXTFIELD_WORD_GAP = 0.33` and `:1984`
      `LABEL_CAPTION_PAIR_FLEX_ROW_SPACING = -8.0` — find the authored
      flex spacing/margins that produce these; register as fallbacks
      otherwise.
- [ ] MFD content text scale (open item, handoff §13): the ×4/3 gap is
      currently unexplained — derive structurally (candidate: the
      0.9-scaled content stage height), never land as a bare multiplier.

## Phase 3 — tag/style NAME-keyed semantic decisions

Matching DataCore TYPE strings (`BuildingBlocks_*`) is data-driven and
fine. Deciding COLOURS/sizes from tag or style NAMES is derived
behaviour that the cascade should produce instead:

- [ ] `src/ui_ir/engine_parts/engine_02.part`
      `node_colour_directive_token` (`StateModerate→Base`,
      `StateCritical→Background`, `Bright→Bright`, `Primary→Base`) and
      `semantic_text_colour_token_from_style_tags`
      (`Text_Header`/heading→`Base`) — the 2026-06-12 text-format-route
      work shows authored entries carry much of this; re-audit each arm
      once the route lands and delete arms the data now covers.
- [ ] `src/pipeline/mod.rs` `ScreenNameBackground` tag-name check
      (placeholder-background suppression) — find the structural signal
      (authored background enable/state) instead of the tag name.
- [ ] Inventory: `grep -rn 'eq_ignore_ascii_case("' crates/starbreaker-ui/src`
      minus type/enum strings; classify each surviving NAME match.

## Phase 4 — stage/geometry constants

- [ ] `src/mfd_view.rs:49` `HOST_CONTENT_INSET = 44.0` — measured, not
      authored (memory: content-view sub-rect). Locate the authored
      source (frame canvas layout rects / SWF stage metadata) and derive.
- [ ] Host-stage reference size (1280×720, `BuildingBlocks_root.swf`) —
      read the stage size from the SWF header it already parses rather
      than constants, wherever still inlined.
- [ ] `src/bb_layout/engine_parts/engine_01.part:43`
      `ADDITIVE_ALTERNATE_REVERSE_POSITION_PHASE_RATIO = 2/3` — animation
      phase maths: confirm it mirrors an engine formula (cite) or derive
      from the authored timeline.

## Phase 5 — registry/pins hygiene (pattern is OK, keep it audited)

`data/default_value_registry_v1.json` pins and
`tests/fixtures/ui_ir/brand_palettes_v1.json` are the SANCTIONED shape
for irreducible values (provenance + validator + retirement notes).

- [ ] Sweep `default_value_registry_v1.notes.md` for entries whose
      derivation became possible (e.g. `ship_values.rs` TODOs:
      OUTPUT 2/16, emissions formulas — handoff §8).
- [ ] Add a validator-coverage check: every fixture carrying real game
      values has a matching live-data validator test (currently:
      palettes ✓, registry ✓ via existing flows).

## Tracking

Work phases top-down; one phase per arc/PR with the full battery at the
boundary. Each completed checkbox cites its commit in this file. New
hard-coding discovered mid-phase: fix in place if small, else append
here in the same change (the self-correction rule).
