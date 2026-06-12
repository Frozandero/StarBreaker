# UI parity workflow (authoritative)

THE process for UI-matching work in `starbreaker-ui`: getting a rendered
screen to match its in-game reference, engine-faithfully and generically.
Commands, tools, data locations, and the per-screen dossier live in the
companion **`docs/ui-reference.md`** — this doc is the *how to work*, that
doc is the *what to type*. Supersedes the former
`crates/starbreaker-ui/docs/ui-matching-workflow.md`,
`ui-matching-text-prompt.md`, and `docs/ui-regression-baseline-workflow.md`.

Starting fresh? Read this doc, then `docs/ui-reference.md`, then the current
arc's handoff doc (see the dossier's "open issues" column). The short
per-screen prompt template is
`crates/starbreaker-ui/docs/ui-matching-agent-prompt.md`.

## 1. Non-negotiable rules

1. **Engine-faithful and generic.** No hard-coding, no name-matching, no
   ship-/screen-/manufacturer-specific branches in production code. No
   heuristic blend factors, hand-tuned percentages, or magic offsets unless
   derived from source data. Hard-coded game-data VALUES (palette
   `RgbaColor` literals, font sizes, brand font lists — fallbacks included)
   are hard-coding: derive from DataCore/P4K, or use a provenance-noted
   extracted fixture for offline tests (registry pattern; guard:
   `tests/hardcoding_guard.rs` + `scripts/check_ui_hardcoding.sh`). The rule
   is SELF-CORRECTING: encountering pre-existing hard-coding obliges you to
   replace it or flag it in the same change — never extend it because it is
   already there. If one asset misbehaves, find the structural property of
   its category and fix the rule for the whole category. Ask: *how does the
   engine handle this?* — mirror its model.
2. **TDD.** When a bug is found, write a failing test that reproduces it
   BEFORE changing code; verify it fails; fix; verify it passes.
3. **Frozen platinum/gold = regressions in source behaviour.** Never silence
   them by editing tests or baselines first. Baselines change only through
   the audited freeze flow (§7) with explicit approver + reason, or via a
   reference-anchored known-outlier (§6).
4. **IR is the sole styling authority.** The renderer consumes explicit IR
   fields; it never invents semantics from style tags, widget names, parent
   context, or palettes at draw time. If an effect isn't in IR yet, add it
   to `ui_ir` preprocessing/schema first, then consume it. Compose-time
   effects must be represented in IR/snapshot semantics so guards can see
   drift.
5. **3000-line cap** on every `.rs`/`.part` under `src/` (enforced by
   `line_count_guard`). Split by responsibility well before the cap —
   target chunks of ~2500 lines or less; the cap is the hard stop, not
   the goal.
6. **Remove no-effect experiments immediately.** A patch that doesn't
   measurably change queried IR/draw values or rendered output is a failed
   hypothesis: revert it and record what was falsified before trying the
   next idea. Don't stack overlapping fallbacks — one structural rule per
   behaviour.
7. **Verify-on-write documentation.** Every command line that enters a doc
   is run once at writing time. Renaming/removing a CLI subcommand, script,
   probe, or doc file includes a repo-wide grep for references in the same
   commit.
8. **"Overrides" means freeze-system known-outliers only** — never
   hard-coded value overrides in code.

## 2. The architecture in one page

Pipeline stages and the bug classes they own (deep dive:
`docs/ui-architecture-runbook.md`):

| Stage | Files | Owns |
|---|---|---|
| Source resolution | `bb_resolve/` (Pass-1 canvas-ref entries, Pass-2 WidgetCanvas urls, widget-standard expansion, list/array materialisation, namespaces), `bb_state_filter.rs`, `bb_brand_apply/` (style cascade), `bb_scene/` (parse, WidgetClone expansion) | which nodes exist, their authored fields, applied styles |
| Bindings | `bb_bindings/` (op-graph eval vs `DefaultValueRegistry`, bound geometry/state-tags/text) | values: text, numbers, booleans, bound SizeX/Y + AnchorX/Y, state tags |
| Layout | `bb_layout/` | rects: flex (order, grow/no-grow, shrink, intrinsic text), overlay anchors/pivots |
| IR compile | `ui_ir/` | the canonical `UiIrDocument`: which authored metadata survives, effective font sizes, clip rects, payload classification |
| Render | `ir_compose/` (+ `text/`, `bb_svg`) | final draw: glyphs, fills, tints, contain-fit, clipping |
| Regression | `ui_snapshot/`, manifest tests, freeze fixtures | structural + whole-image comparison vs frozen baselines |

Style cascade order (lowest first): canvas `style` record link < `sharedStyles`
< selected `brandStyles[]` < `embeddedStyles` < node `inlineStyles` (applied
last in every pass; an inline `FontSize` is marked `__InlineFontSize` and
outranks the brand-table standard in font resolution).

Per-ship values: derived at export in
`starbreaker-3d/src/ui_pipeline/ship_values.rs` → `PipelineInputs::derived_values`;
the CLI replay derives from the scene's root entity so **replay == export**.
At-rest engine-pushed values that can't be derived yet are pinned in
`crates/starbreaker-ui/data/default_value_registry_v1.json` — every pin gets a
provenance entry in `default_value_registry_v1.notes.md`.

## 3. The working loop (per catalog item)

1. **Identify the owning stage before editing.** For style/tag questions run
   the MCP trio in order: `ui_canvas_style_inventory` (which authored
   container/entry) → `ui_scene_style_probe` (which entries matched the
   node) → `ui_ir_query` (what the renderer will consume). For geometry,
   `ui render --dump-ir-dir` + `examples/ui_stage_diff` (parse vs resolve)
   + the probes (reference doc §6). Read the authored canvas in the
   dcb_canvas mirror — it is grep-able JSON.
2. **One falsifiable hypothesis, one focused change**, with the failing test
   first (rule 2). Don't chain speculative layout changes between
   measurements.
3. **Validate:** `bash scripts/ui_check.sh` (every cycle), then re-render
   via the ~1-minute replay (`bash scripts/ui_render.sh --helper <name>` —
   it rebuilds first and prints the binary mtime, so a stale binary can't
   masquerade as "the fix had no effect") and compare with
   `scripts/ui_compare.py`.
4. **Update the catalog** (fixed / still open / new finding) and the arc's
   memory/handoff notes for any non-trivial diagnosis — at discovery time,
   not at session end.
5. **Commit** per coherent fix; message cites the catalog item. Before each
   commit, the arc's memory/handoff state is current.
6. At a workstream boundary: `bash scripts/ui_check.sh --full`, re-export,
   full reference comparison, handoff/memory update.

User gives relative feedback ("move it ~20px up")? Treat it as a calibration
target: measure current values first, trace the mismatch to authored
metadata / layout math / IR loss / draw-time adjustment, fix structurally,
re-measure numerically, only then ask for visual confirmation.

## 4. The review phase (mandatory at every workstream boundary)

1. Re-render the screen (replay; full export only when refreshing canonical
   PNGs).
2. `python3 scripts/ui_compare.py <render> <reference> --regions <preset>`
   and READ each crop with vision. For any colour question add `--stats`
   (bright/dark means + R-normalised ratios per region): judge hue from
   ratios, calibrate the capture's cast from a known anchor on the SAME
   reference (footer text = Base, pip slabs = Bright) — method details in
   the reference doc §3. Pixel adjudications consult the **measurement
   bank FIRST** (`crates/starbreaker-ui/tests/fixtures/ui_ir/
   reference_measurements_v1.json` + its `.notes.md`): settled reference
   numbers are lookups, not re-measurements; new measurements use
   `scripts/ui_measure.py` and get appended to the bank with provenance.
3. Build/extend the numbered **diff catalog**: region | difference |
   severity | root-cause hypothesis | fix-or-defer decision. Deferrals are
   explicit entries, not omissions. Reference screenshots are imperfect
   (skew, bloom, resolution, capture artifacts — e.g. the power screen's
   red/white pip outlines are mouse-hover artifacts): compare structurally,
   not pixel-naively.
4. Order the fixes (structural/layout before styling; shared-root-cause
   items together).
5. Execute via §3; re-run the review until parity or only documented
   deferrals remain.

## 5. When a guard trips

The platinum/gold guards failing after your change is the system working.
Method (never name-gate your way out):

1. Read the failure: it names the target and `<node_id>:<node_type>` with
   field deltas.
2. Find that element in the freeze JSON
   (`crates/starbreaker-ui/tests/fixtures/ui_ir/ui_snapshot_freeze.json`)
   and its authored source in the dcb_canvas mirror.
3. Find the **structural discriminator** separating the frozen
   counterexample from your motivating case (sizing behaviour class, anchor
   range, absolute-vs-relative path, value range — never a name).
4. Scope the rule by that property and cite the counterexample in a code
   comment.
5. If instead the **baseline is wrong** (your change moves the render toward
   the in-game reference): verify visually against the reference, then
   re-freeze (§7) quoting the per-identity delta in the reason and commit
   message.
6. If the deviation is known but the true fix is deferred: register a
   known-outlier (§6) instead of freezing the miss.

**The empirical disable→adjudicate audit** (ledger item 22): to test whether
a suspect rule (hard-coded constant, name match, special pass) is
load-bearing, disable it and let the frozen pins adjudicate — one session
proved eight rules deletable and three load-bearing this way, far cheaper
than per-rule reference archaeology. Preconditions, both mandatory:
(1) **fresh export first** — the whole-image guard compares
`ships/Data/UI/Generated/*.png`, which only refresh on export; the P0.2
staleness guard (`.export_stamp.json`) now hard-fails stale comparisons,
but don't lean on it: re-export (~50s) before judging "zero drift";
(2) consult ALL THREE suites — lib tests, the live-IR guard, AND the
visual/snapshot suites — a rule can be invisible to two and pinned by the
third. Scope caveat: a clean pass proves "**no frozen pin references
this**" (five screens, one ship today), NOT "correct everywhere" — record
the deletion in the fallback register's retired table with that bound.

## 6. Known-outlier overrides (reference-anchored)

For elements knowingly off-reference where the true fix is deferred:
register in `crates/starbreaker-ui/tests/fixtures/ui_ir/ui_known_outliers.json`
(`{target, identity, field, frozen_value, reference_target, confidence,
reason, source}`; numbers only — no reference image enters the repo). The
comparator treats the field one-sided: movement toward `reference_target`
passes with a `✅ IMPROVEMENT` note (**re-freeze, don't revert**); away
fails; within `confidence` → graduate to a strict freeze. Field-generic
(geometry, alpha, font_size, text tops, tint rgba/token). Gotcha: freeze
pipeline font metrics differ from full renders — anchor text-top targets as
`frozen_snapshot_value − measured_render_delta`. Full policy:
`docs/ui-regression-policy.md`.

## 7. Freezes, target onboarding, tier changes

Baselines are data-driven from
`crates/starbreaker-ui/tests/fixtures/ui_ir/ui_snapshot_manifest.json`.
Flows (commands in the reference doc §2/§7):

- **Onboard a target** (only after the output is explicitly approved as a
  standard): `add_ui_regression_target.sh` → `freeze_ui_snapshot_ir.sh
  --approver <name> --reason "..."` → re-export →
  `generate_ui_regression_artifacts.sh` → `freeze_ui_regression_artifacts.sh`
  → both validators → `ui_check.sh --full`.
- **The artifact cycle as one command**: `bash scripts/ui_freeze_cycle.sh
  --approver <name> --reason "..."` runs release build → export → stale
  `*-current.png` cleanup → artifact freeze → both validators →
  `ui_check.sh --full`. The IR snapshot freeze stays a separate, deliberate
  step: its printed delta must be read and accounted for.
- **Re-freeze after an intentional improvement**: same freeze command; the
  tool prints the per-identity delta — the reason and the commit message
  must account for every changed identity. AUDIT RULE: a re-freeze whose
  delta contains identities you can't explain is rejected.
- **Artifact freezes** source the EXPORTED PNGs: re-export first; the export
  writes those PNGs near the END of its run — never diff or freeze until
  the process has exited.
- **Tier changes**: never to silence drift; re-freeze with reason, run both
  validators + the full battery.
- No image binaries in freeze commits (`test-artifacts/ui/*.png` stays
  untracked).
- Every UI defect fix lands with a regression guard in the same change
  (category guidance: `docs/ui-regression-policy.md`). Pixel regression is
  the content-agnostic whole-image guard — do NOT add per-screen ROI pixel
  checks.

## 8. Derivation policy

- Ship-specific values (pips, temps, battery counts, signatures…) derive
  from DataCore in `ship_values.rs`, with unit tests asserting the
  reference-verified numbers.
- Engine-runtime values with no decoded formula yet are pinned
  (registry or derivation constants) with a TODO + provenance note —
  documented IOUs, not silent magic.
- New fallback logic of any kind requires an entry in
  `docs/ui-fallback-register.md` (trigger signal + retirement target).

## 9. Memory, handoffs, prompts

- Non-trivial diagnoses go into the arc's memory file **when made**.
- Every commit is preceded by a memory/handoff status update.
- An arc that pauses (or at any major milestone) maintains a repo handoff
  doc — pattern `docs/ui-<arc>-handoff.md` — with: landed commits, catalog
  status, next-step spec (ideally an `#[ignore]`d failing test), parked
  work with full diagnosis, approval-gated items.
- To launch fresh-context work on a screen, instantiate
  `crates/starbreaker-ui/docs/ui-matching-agent-prompt.md` with the screen
  name — the dossier (reference doc §3) supplies everything else.
- At the END of an arc — in the SAME session, before clearing context —
  paste the process retrospective prompt
  (`crates/starbreaker-ui/docs/ui-process-retro-prompt.md`): it runs on the
  session's lived experience (the friction only that session knows);
  findings append to the `docs/ui-process-improvements.md` ledger and get
  implemented, so each arc lowers the next one's bootstrap cost.

## 10. Known pain points & don't-retry list

- **State-bound visibility**: live-session elements are IsActive-gated
  chains false at static rest; visibility gating lives in
  `bb_state_filter`'s capture-tested heuristics. A generic IsActive-binding
  pass over the merged scene breaks medical platinum — don't retry.
- **Relative `urlPostfix` composition**: medical canvases author bindings
  pre-qualified and the platinum registry keys pin that resolution; only
  ABSOLUTE (leading-slash) postfixes compose. Composing relative ones needs
  a registry key migration first.
- **Overlay-default tint** ("enableColorOverlay + null colour → Base" for
  shapes/images): regressed target-screen chevrons. Entry-driven FillColor
  is the engine model. (The narrow WidgetIcon case is implemented and
  scoped.)
- **Column-wide intrinsic text sizing**: medical baselines pin the
  fill/auto_main placement for non-zero Auto hints in columns; only
  Auto == 0.0 (pure content hint) is in scope for column intrinsics.
- **Caps font-size fudges**: the 0.98 all-caps reduction was removed; styled
  text renders the full brand nominal. Don't reintroduce nominal-scale
  constants — the data-backed font model owns sizing
  (`docs/ui-font-size-harness.md` guards it; run it via `ui_check.sh
  --full` whenever text size could change).
- **`.tif` in canvas JSON = `.dds` in P4K.**
- **Stale generated PNGs**: `ships/Data/UI/Generated/...` only refreshes on
  full export. Iterate via replay; export before freezing artifacts.
