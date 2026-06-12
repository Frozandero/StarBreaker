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

## Phase 1 — colour-token slot maps → BB_ColorStyle enum — DONE

Audit: grepping the record mirror for `"color": "<token>"` (2026-06-12)
finds ONLY the enum members; every hand-mapped alias occurs in no game
record. Battery: no new frozen-target drift; snapshot + visual suites
green (the realigned tokens — `Accent2`→5 surface, `Moderate`/`Selected`/
`Contact*`/`MissionObjectives` now enum-true — appear on no frozen target
at rest).

- [x] `src/style/colour_roles.rs` — shared `bb_colour_style_enum_index`
      (the divergence-free enum truth + spot-pin tests).
- [x] `src/bb_brand_apply/colors.rs` `color_style_slot_index` → enum +
      TWO reference-cited divergences (`Bright`→0 surface; `Accent1`
      foreground→0); all dead aliases deleted.
- [x] `src/ir_compose/engine_parts/engine_01.part`
      `resolve_colour_token` → enum + cited foreground divergences
      (`Accent1|Base`→0 with primary fallback; `Background` stays the
      parsed style field); `positive/success` primary-similarity
      heuristic deleted; `resolve_surface_colour_token` = enum
      (`Accent1`→4). Alias-using tests rewritten to real tokens.
- [x] Guard extended: `colour_array_literals_are_not_hardcoded_in_production`
      bans non-neutral 4-element colour-array literals on colour-ish
      production lines (test files/regions exempt; `hardcoding-guard:
      synthetic` annotation for the diagnostic wireframe grey).

Residual (noted, not regressed): `ManufacturerStyle.background` is parsed
from slot 8 (`StyleLoader::parse_buildingblocks_style_record`) while the
enum names slot 9 `Background` — frozen-pinned behaviour; re-derive
against a reference when a screen discriminates the two.

## Phase 2 — font sizing calibration constants — REGISTERED (derivations tracked)

Outcome 2026-06-12: every constant is now an explicit
`docs/ui-fallback-register.md` entry with telemetry + retirement criteria
(the register was also refreshed: stale paths/values fixed; the retired
Drake-amber and ×0.98 all-caps entries moved to Retired). The deep
derivations stay open with their criteria recorded in the register:

- [x] `FIXED_BAND_HEADING_FILL = 0.381` — **RETIRED** (2026-06-12): the
      text-format route landed (T3 resolved: literal widget matches do
      not outrank the named-style table; only text-format-routed
      FontSizes do) and the authored mainmenu bioc `FontSize=40` entry
      now sizes the banner. Fallback deleted; full battery + font
      harness green, zero drift.
- [x] `LONG_HEADING1_PROMPT_FONT_SIZE = 28.7` — registered (new entry);
      same authored-entry derivation attempt queued on the route landing.
- [x] `TEXT_RENDER_SIZE_CALIBRATION = 1.5` /
      `LAYOUT_TEXT_MEASURE_CALIBRATION = 1.5` /
      `SWF_TEXT_RENDER_SIZE_CALIBRATION = 0.84` — registered with the
      shared-metrics retirement plan (extend `pipeline/text_measure.rs`'s
      measure==draw approach to the TTF path; re-derive 0.84 from the SWF
      font em model). Substantial renderer work — own arc.
- [x] `INLINE_NESTED_TEXTFIELD_WORD_GAP = 0.33` — already registered;
      path refreshed.
- [x] `LABEL_CAPTION_PAIR_FLEX_ROW_SPACING = -8.0` — register entry was
      STALE (−5, old path); corrected with the current line-box rationale.
- [ ] MFD content text scale (open item, handoff §13): the ×4/3 gap is
      currently unexplained — derive structurally (candidate: the
      0.9-scaled content stage height), never land as a bare multiplier.
      The route arc landed (07c821a83); derive next power-arc session
      (the ºC glyph is the clean discriminator).

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
      now that the route LANDED (commit 07c821a83) and delete arms the
      data covers — each deletion adjudicated per reference.
- [ ] `src/pipeline/mod.rs` `ScreenNameBackground` tag-name check
      (placeholder-background suppression) — find the structural signal
      (authored background enable/state) instead of the tag name.
- [x] Inventory (2026-06-12): the unique `eq_ignore_ascii_case` target
      set was classified. Data-driven and FINE: DataCore type/enum/field
      strings (`BuildingBlocks_*`, flex/sizing/overflow enum values,
      modifier field names, `@LOC_*`), our own internal node-type/marker
      strings. NAME-keyed survivors needing structural replacements:
      - [ ] `bb_resolve/engine_parts/engine_01.part:250` `"RootGhost"`
            and `:1479` `"base_animatedelements"` (node-NAME matches);
      - [ ] `ui_ir/engine_parts/engine_01.part:496`
            `"root_annunciator_items"` (node-NAME match) and `:1131`
            `"FunctionTitle"`;
      - [ ] tag-name keyed colour/behaviour arms (`Bright`, `Flashing`,
            `Modify`, `Ghost`, `icon`, `Heading1` style names) — the
            Phase 3 audit set above.

## Phase 4 — stage/geometry constants — REGISTERED (one follow-up)

- [x] `src/mfd_view.rs` `HOST_CONTENT_INSET = 44.0` — the derivation
      attempt was already made and documented at the constant: the
      placement is runtime ActionScript, PROVEN absent from records and
      static SWF placement. Now a register entry (measured framework
      constant with provenance + drift telemetry).
- [x] Host-stage text scale already reads the SWF header
      (`pipeline/host_stage.rs` via `SwfAssetLibrary::stage_size`) — no
      constant there. Remaining: `mfd_view.rs` `HOST_STAGE_SIZE`
      could read the same header when next touched (noted in its register
      entry).
- [x] `ADDITIVE_ALTERNATE_REVERSE_POSITION_PHASE_RATIO = 2/3` —
      registered with retirement criteria (derive the at-rest phase from
      the authored timeline instead of a fixed ratio).

## Phase 5 — registry/pins hygiene (pattern is OK, keep it audited)

`data/default_value_registry_v1.json` pins and
`tests/fixtures/ui_ir/brand_palettes_v1.json` are the SANCTIONED shape
for irreducible values (provenance + validator + retirement notes).

- [x] Sweep 2026-06-12: the power at-rest profile pins are SHADOWED by
      `ship_values.rs` derivation (registry copies remain as the
      documented bare-replay fallback — retirement criterion already in
      the notes: registry-less test render + `SB_SHIP_VALUES_DUMP=1`).
      The OUTPUT 2/16 + emissions-formula derivations remain
      APPROVAL-GATED (handoff §8 asks Tom — engine SignatureSystem
      formulas).
- [x] Validator coverage: brand palettes →
      `brand_palette_fixture_matches_live_records`; registry pins →
      the freeze validators + notes-file discipline (validators run in
      `ui_check.sh --full`). New real-value fixtures must ship with a
      validator (rule recorded in `crates/starbreaker-ui/AGENTS.md`).

## Tracking

Work phases top-down; one phase per arc/PR with the full battery at the
boundary. Each completed checkbox cites its commit in this file. New
hard-coding discovered mid-phase: fix in place if small, else append
here in the same change (the self-correction rule).
