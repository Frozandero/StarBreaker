# UI parity process — improvements and consolidation plan

A retrospective of the Clipper power-screen parity arc (2026-06-10 → 06-11)
turned into concrete process changes, followed by the **phased, actionable
plan** that implements them (§"Phased plan"). The plan is written to be
executed from fresh context: every file path, command, and acceptance
criterion is explicit.

Status: **reviewed and planned, NOT yet implemented** (per Tom's
instruction). The current arc's work-state handoff is separate:
`docs/ui-clipper-parity-handoff.md`.

---

## Part A — process findings

Each item: the observed problem (with the incident that exposed it), the
improvement, and the action. Items 1–10 are the original retrospective
(reviewed and corrected); items 11–12 were added during review.

### 1. Make the render→compare→catalog review a standard phase

**Observed:** fixes were driven by a hand-written symptom list; a full
region-by-region comparison only happened when explicitly requested (Step
9R) — and immediately found defects the symptom list missed (emissions
header collapsed to 2 px, OUTPUT title mis-flowed) plus one latent
engine-model bug (`SizeY` style modifiers converting Percent→Fixed) whose
fix also resolved the long-open A7 backdrop item.

**Improvement:** every workstream ends with a mandatory review phase:
re-render → region compare → diff catalog (severity + root-cause hypothesis
+ fix/defer decision) → ordered fix plan → fixes. Defects found by review
get catalog numbers so "deferred" is an explicit, recorded decision, not a
gap.

**Action (corrected):** the review phase becomes a named step in the NEW
consolidated workflow doc (item 11) — *not* in
`crates/starbreaker-ui/docs/ui-matching-workflow.md`, which is being
superseded. (The original action here cited `docs/ui-matching-workflow.md`,
a path that does not exist — itself evidence for item 11.) Catalog tables
live in the arc's memory/handoff file.

### 2. Build a reusable region-compare tool

**Observed:** every comparison was a hand-written PIL crop snippet;
mismatched reference resolutions (1959×1513 vs 1600×1200) and bad crop boxes
wasted several iterations.

**Improvement:** one script with named region presets per screen and
auto-scaling that emits labelled side-by-side crops.

**Action:** `scripts/ui_compare.py <render.png> <reference.png>
[--regions <screen-preset>] [--out-dir <dir>]` — scales the reference to the
render width, crops named regions (presets checked in per screen, e.g.
`power: emissions, columns, scrollbar, output_card, battery_card, footer`;
`target: status_band, chevrons, footer`), writes `cmp_<region>.png`
side-by-sides plus a full-frame pair. Review phases use it exclusively.
Renders for iteration come from the ~1-minute replay
(`ui render --scene .../scene.json --helper <name>`), not a full export.

### 3. Self-checking harnesses — a zero-match run is a harness FAILURE

**Observed:** `scripts/font_size_check.py` silently reported every baseline
element MISSING because the dump format had gained a `width_px` column after
the baseline was captured. Separately,
`crates/starbreaker-ui/tests/fixtures/font_size_baseline.tsv` had been stale
for some time (7 real drifts from earlier *approved* work) because the
harness wasn't run when those changes landed.

**Improvement:** harness checkers must fail loudly when they match *nothing*
(that is parser/format drift, not data drift), and harness runs belong to
the standard check battery (item 4), not to operator memory.

**Action:**
- `font_size_check.py`: exit with a distinct "harness error" code/message
  when matched elements == 0 or the dump column count is unexpected.
- Re-capture `font_size_baseline.tsv` (**approval-gated** — the 7 known
  drifts are from approved work: caps-reduction removal, annunciator/door
  changes: PWR/WPN/THR +11.8%, SHLD +3.2%, CLOSED −10.7%,
  TierLevel/TitleText +3.4%). Capture from the **LOD1** scene
  (`ships/Packages/DRAK Clipper_LOD1_TEX2/scene.json`) — the LOD0 scene
  lacks the medical/door/annunciator bindings the baseline covers.
- Fold the harness into `ui_check.sh --full` (item 4).

### 4. One command for the standard check battery

**Observed:** the loop "ui lib tests → manifest_live_ir_guard →
line_count_guard" was retyped dozens of times; a file went over the 500-line
cap unnoticed for several edits because `line_count_guard` is an integration
test that `--lib` runs don't include. Separately, the superseded workflow
doc's "required full-scope path" lists additional suites
(`manifest_snapshot_regression`, `manifest_visual_regression`,
`validate_ui_snapshot_freeze.sh`, `validate_ui_regression_artifacts.sh
--quick`) that the TDD loop never ran — two divergent "standard" batteries
existed.

**Improvement:** one entry point with two tiers, so every TDD cycle ends
with the same command and workstream boundaries run the full set.

**Action:** `scripts/ui_check.sh`:
- default (TDD tier): `cargo test -p starbreaker-ui --lib` +
  `cargo test -p starbreaker-ui --test manifest_live_ir_guard --test
  line_count_guard`.
- `--full` (boundary tier): adds `--test manifest_snapshot_regression`,
  `--test manifest_visual_regression`,
  `bash scripts/validate_ui_snapshot_freeze.sh`,
  `bash scripts/validate_ui_regression_artifacts.sh --quick`,
  `cargo test -p starbreaker-3d --lib`, and the font harness (item 3)
  against a named scene.
The new workflow doc names `ui_check.sh` as THE validation command.

### 5. Self-auditing freezes

**Observed:** re-freezing the gold target required a hand-written JSON diff
to prove only the intended element changed (it was exactly one:
`clipper_target_master 40:widget_custom_shape h 0.18 → 194.4`). That audit
is what makes a re-freeze trustworthy, and it currently depends on the
operator remembering to do it.

**Improvement:** the freeze tooling prints (and records) a structured delta
report — per target, each changed identity with old→new field values — and
refuses to write when nothing would change or when run without
`--approver`/`--reason`.

**Action:** extend `crates/starbreaker-ui/examples/freeze_ui_snapshot_ir.rs`
(driven by `scripts/freeze_ui_snapshot_ir.sh`) to diff against the existing
`crates/starbreaker-ui/tests/fixtures/ui_ir/ui_snapshot_freeze.json` and
emit the per-identity delta to stdout; store the delta summary alongside
`approver`/`reason` in the freeze JSON. The commit message for a freeze must
quote the delta.

### 6. A pipeline-stage bisection tool for layout/geometry bugs

**Observed:** the single most effective diagnostic this arc was a throwaway
example that parsed a canvas standalone and laid it out, proving the
emissions collapse came from the resolve cascade, not parse or layout
(sizing was `Percent(1.0)` standalone, `Fixed(1.0)` after resolve). The
scratch example (`crates/starbreaker-ui/examples/repro_emissions.rs`,
committed as-is) was hand-written under time pressure.

**Improvement:** keep the capability permanently: run any canvas through
(a) parse-only and (b) full resolve, lay both out at a given size, and
print per-node typed sizing + rects, flagging the first divergence.

**Action:** generalise into `crates/starbreaker-ui/examples/ui_stage_diff.rs`
(`cargo run -p starbreaker-ui --example ui_stage_diff -- <canvas.json>
[WxH] [--records-root <dir>] [--filter <name-substring>]`), defaulting
`--records-root` to the decompiled mirror
`/home/tom/projects/scorg_tools/ships/dcb_canvas/libs/foundry/records`.
Delete `repro_emissions.rs`.

### 7. A probe registry instead of folklore

**Observed:** env-gated probes are discovered by grepping, and a memory note
referenced a probe (`MFD_IR_DUMP_LOG`) that does not exist. Verified-real
probes this arc: `BB_A3_STYLE_PROBE`, `BB_A3_TEXT_PROBE`, `BB_SHRINK_PROBE`,
`SB_UI_GEOM_PROBE`, `SB_SHIP_VALUES_DUMP`, `SB_UI_FONT_DUMP`; plus the
`ui render --dump-ir-dir <dir>` IR dump flag and the `mfd_ir_dump` example.

**Improvement:** one documented list; new probes are added to it in the same
commit that introduces them.

**Action:** the probe table becomes a section of the consolidated reference
doc (item 11) rather than a standalone `docs/ui-probes.md` — one fewer
document to drift. Each entry: env var / flag, owning module, what it
prints, one copy-paste invocation.

### 8. Codify the guard-adjudication method

**Observed (working well — codify it):** four times this arc a generic rule
tripped a frozen baseline, and the fix was the same method each time: read
the frozen counterexample, find the *structural* property separating it from
the motivating case, scope the rule by that property — never by name:
- flex shrink → only flex-managed children (Fixed/Percent/Auto∈(0,1]);
- Auto-hint textfield intrinsic sizing → only when anchored beyond the
  parent edge (anchor > 1.0 on that axis);
- `urlPostfix` namespace composition → only absolute (leading-slash)
  postfixes;
- column intrinsic text sizing (pending) → only Auto value == 0.0.
And once the *baseline* was the wrong side: the A7 backdrop sliver — which
is the audited re-freeze path (item 5).

**Improvement:** write the method down so it's the default move, and record
each scoped rule's counterexample in the code comment (mostly already done).

**Action (corrected):** a "When a guard trips" section in the NEW workflow
doc (item 11): (1) identify the frozen counterexample node (the guard names
`<node_id>:<node_type>` and the target), (2) read its authored source in the
dcb_canvas mirror, (3) find the structural discriminator, (4) scope the rule
and cite the counterexample in a comment, (5) if the baseline is the thing
that's wrong, perform the audited re-freeze (item 5), (6) if the deviation
is known and intentionally unfixed, register a reference-anchored
known-outlier (`crates/starbreaker-ui/tests/fixtures/ui_ir/ui_known_outliers.json`).

### 9. Memory updates at discovery time, before each commit

**Observed (working well — codify it):** the emissions mechanism decode was
written to the project memory *before* implementation started, which is what
allowed the work to survive context compaction; conversely, earlier
diagnoses written "when context ran low" were rushed. The end-of-arc handoff
doc (`docs/ui-clipper-parity-handoff.md`) proved the right vehicle for
session-spanning state.

**Action:** standing rules in the new workflow doc: a non-trivial diagnosis
is written to the arc's memory file when it is *made*; every commit is
preceded by a memory-file status update; an arc that pauses produces/updates
a repo handoff doc (pattern: `docs/ui-<arc>-handoff.md`).

### 10. Registry data hygiene for at-rest engine values

**Observed:** values the engine pushes at runtime (`iscast` per render
target type, signature emissions, OUTPUT 2/16, ambient temps) end up either
pinned in `crates/starbreaker-ui/data/default_value_registry_v1.json` or
derived in `crates/starbreaker-3d/src/ui_pipeline/ship_values.rs`, and the
boundary is only documented in commit messages. JSON carries no comments, so
each pinned value's provenance ("reference-verified", "derivation TODO") is
invisible at the data file.

**Action:** a sibling
`crates/starbreaker-ui/data/default_value_registry_v1.notes.md` mapping each
pinned path → provenance + sunset condition (move-to-derivation), updated in
the same commits that touch the registry; `docs/ui-fallback-register.md`
links to it.

### 11. Consolidated process documentation (NEW — the centrepiece)

**Observed:** the process knowledge is scattered, duplicated, and partly
wrong, across FOUR places:

1. **Repo `docs/`**: `ui-architecture-runbook.md`, `ui-regression-policy.md`,
   `ui-regression-baseline-workflow.md`, `ui-font-size-harness.md`,
   `ui-fallback-register.md`, `ui-matching-tasks/target-master-findings.md`,
   plus this doc and the arc handoff.
2. **Crate `crates/starbreaker-ui/docs/`**: `ui-matching-workflow.md` (the
   *actual* workflow rules — 328 lines), `ui-matching-agent-prompt.md`,
   `ui-matching-text-prompt.md`, `gold-platinum-regression-deep-dive.md`,
   `ir-freeze-schema.md`, `ir-style-authority-migration-plan.md`; plus
   `crates/starbreaker-ui/AGENTS.md`.
3. **Repo root**: stale one-off handoffs `HANDOFF-item2-medical2.md`,
   `HANDOFF-medical2-followup.md`.
4. **Workspace (outside the repo)**: `/home/tom/projects/scorg_tools/docs/`
   (ui-plan2.md and archives, `docs/StarBreaker/*` research notes) — and the
   Claude session memory.

Concrete drift found during this review:
- This doc and the arc handoff referenced `docs/ui-matching-workflow.md` —
  **a path that does not exist** (the real file is under the crate).
- `ui-matching-workflow.md` instructs CLI fallbacks `starbreaker ui debug`
  and `starbreaker ui styles` — **subcommands that no longer exist** (the
  current CLI has `ui render` and `ui mfd` only; `ui render --dump-ir-dir`
  is the IR dump path now).
- It mandates `SC_DATA_P4K=...` on every command; the P4K is **auto-detected
  now** (per Tom's standing feedback) — needed only for non-default data.
- Its validation loop omits `line_count_guard` and the font harness.
- The agent-prompt template embeds a stale export invocation.
- A memory note referenced a non-existent probe (item 7).

**Improvement:** ONE authoritative, self-sufficient documentation set such
that a fresh-context agent (or a brand-new agent) can run the entire process
reading only it. Old documents are rewritten into it or deleted — no
parallel half-truths left to confuse.

**Action — write the following two documents** (do not implement until the
phased plan is executed; full content requirements below so nothing is lost):

#### `docs/ui-workflow.md` — THE process (authoritative)

Must contain, in this order:

1. **Mission & non-negotiable rules** (carried from the old workflow doc,
   corrected): engine-faithful + generic (no per-asset hacks, no name
   gating); TDD (failing test first, verify it fails); frozen
   platinum/gold = regressions in source behaviour, never silenced by
   baseline edits; baselines change only via the audited freeze flow with
   explicit approver+reason; 500-line file cap; remove no-effect experiments
   immediately; IR is the sole styling authority (renderer consumes explicit
   IR fields, never invents semantics from tags/names at draw time);
   compose-time effects must be represented in IR/snapshot semantics.
2. **Required reads** (corrected list): `StarBreaker/AGENTS.md`,
   `crates/starbreaker-ui/AGENTS.md`, this doc, the satellite docs index
   (below), the arc's current handoff doc, and the Claude memory file for
   the arc when resuming.
3. **The architecture in one page** + pointer to
   `docs/ui-architecture-runbook.md`: pipeline stages
   (bb_resolve Pass-1/Pass-2 → brand/style cascade → bb_bindings →
   bb_layout → ui_ir → ir_compose), where each class of bug lives, and the
   style cascade order (style-link < sharedStyles < brand < embedded <
   node inlineStyles; inline FontSize marked `__InlineFontSize`).
4. **The working loop**: pick a catalog item → write the failing test →
   fix at the owning stage → `scripts/ui_check.sh` → re-render via replay →
   region-compare → update catalog/memory → commit (message cites catalog
   item). Includes the investigation order for style/tag questions
   (MCP `ui_canvas_style_inventory` → `ui_scene_style_probe` →
   `ui_ir_query` → only then edit; no-effect changes are reverted and the
   failed hypothesis recorded).
5. **The review phase** (item 1) as a numbered procedure with
   `scripts/ui_compare.py` invocations and the catalog table format.
6. **When a guard trips** (item 8, full method incl. audited re-freeze and
   known-outlier registration).
7. **Freeze / target onboarding / tier change flows** (absorb the still
   correct parts of the old workflow doc + `ui-regression-baseline-workflow.md`):
   `add_ui_regression_target.sh`, `freeze_ui_snapshot_ir.sh` (+ delta
   audit), `generate_ui_regression_artifacts.sh`,
   `freeze_ui_regression_artifacts.sh` (note: artifact freeze sources the
   EXPORTED PNGs — re-export first; export writes PNGs near the END of the
   run), `validate_*` scripts, repo-only CI path
   (`validate_ui_regression_repo_only.sh`), approval checklist, "no image
   binaries in freeze commits".
8. **Derivation policy**: per-ship values derive in
   `starbreaker-3d/src/ui_pipeline/ship_values.rs` flowing through
   `PipelineInputs::derived_values`; replay derives from scene root entity
   so replay == export; at-rest engine-pushed values that cannot be derived
   yet are pinned in the registry WITH a notes entry (item 10); "overrides"
   means ONLY freeze-system known-outliers, never hard-coded values in code.
9. **Memory & handoff conventions** (item 9).
10. **Known pain points and don't-retry list** (carried + current): static
    state-bound visibility; relative `urlPostfix` composition (breaks
    medical platinum — needs registry key migration first); the
    "enableColorOverlay+null → Base" overlay default (regressed target
    chevrons); column-wide intrinsic text sizing (medical pins fill
    placement; only Auto==0.0 is in scope); reference screenshots are
    imperfect (skew/bloom/resolution — structural comparison, not naive
    pixel matching; power-screen pip outlines are mouse-hover artifacts).

#### `docs/ui-reference.md` — commands, tools, data (the lookup half)

Must contain (every entry verified against the current code before
writing):

1. **Build & test**: `cargo build` (debug is fine for iteration; release
   only for deploy/export); `scripts/ui_check.sh` (+ `--full`); full
   workspace test; the individual test names for targeted runs.
2. **Render & export**:
   - Replay (iteration, ~1 min): `./target/debug/starbreaker ui render
     --scene "<ships>/Packages/DRAK Clipper_LOD0_TEX0/scene.json"
     --out-dir /tmp/out [--helper Screen_Left_Lower_RTT]
     [--dump-ir-dir /tmp/out/ir] [--mip N]`.
   - Full export (canonical PNGs, ~6 min, needed before artifact freezes):
     `./target/release/starbreaker entity export drak_clipper
     /home/tom/projects/scorg_tools/ships --kind decomposed` (P4K
     auto-detected; do NOT set `RAYON_NUM_THREADS=1` except when
     benchmarking).
   - Which scene has which bindings: LOD0 = cockpit MFD screens; LOD1 =
     medical/door/annunciator etc. (list them).
3. **Comparison**: `scripts/ui_compare.py` usage + preset list; reference
   image inventory (`/home/tom/projects/scorg_tools/reference/in-game/...`
   per screen, with resolution caveats).
4. **MCP tools** (server `starbreakerMcp`), with WHEN-to-use guidance:
   - UI: `ui_canvas_style_inventory`, `ui_scene_style_probe`, `ui_ir_query`
     (the style investigation order), `ui_regression_registry`,
     `ui_regression_validate`.
   - Data: `search_entities`, `search_records`, `datacore_record`,
     `datacore_query` (e.g.
     `Components[SEntityComponentDefaultLoadoutParams]`), `entity_loadout`
     (resolved tree), `p4k_list`/`p4k_read`/`p4k_search` (CryXML
     auto-decode), `image_preview` (vision on P4K DDS), `chunk_list`/
     `chunk_read`.
   - Policy: MCP-first for data archaeology; CLI for renders/exports;
     local PNGs are read directly with the Read tool (vision), never via
     MCP; MCP server redeploy procedure (kill, rebuild, copy — from
     AGENTS.md).
5. **Local data locations**: the decompiled record mirror
   `/home/tom/projects/scorg_tools/ships/dcb_canvas/libs/foundry/records`
   (grep-able authored canvases — the workhorse of this arc), the export
   tree `ships/Packages/<Ship>_LODn_TEXn/scene.json` + generated PNGs
   `ships/Data/UI/Generated/ship/<mfr>/<Ship>/...`, fixtures
   (`ui_snapshot_manifest.json`, `ui_snapshot_freeze.json`,
   `ui_known_outliers.json`, `font_size_baseline.tsv`), the registry +
   notes, key engine-parts files by responsibility (the handoff doc's
   "mechanisms quick reference" table, generalised).
6. **Probe registry** (item 7 table).
7. **Diagnostic examples**: `ui_stage_diff` (item 6), `mfd_ir_dump`
   (canvas+content GUIDs, name-substring filter), `bb_layout_wireframe`,
   `freeze_ui_snapshot_ir`.
8. **Glossary**: platinum/gold tiers, known-outlier, derivation vs pin,
   relay/slot-broadcast, widget-standard expansion, host-stage scale,
   BB_ColorStyle slot order, etc. — one line each with a source-of-truth
   pointer.

#### Supersede list (explicit dispositions)

| Document | Disposition |
|---|---|
| `crates/starbreaker-ui/docs/ui-matching-workflow.md` | **Delete**; content absorbed (corrected) into `docs/ui-workflow.md`. Leave no stub — update every reference. |
| `crates/starbreaker-ui/docs/ui-matching-agent-prompt.md` | **Rewrite** as a ~20-line prompt that points at `docs/ui-workflow.md` + the arc handoff and states only the per-arc variables (images, ship, goal). |
| `crates/starbreaker-ui/docs/ui-matching-text-prompt.md` | **Delete** (text-only variant obsolete — agents in use are vision-capable; the rewritten prompt covers both). |
| `docs/ui-regression-baseline-workflow.md` | **Delete**; freeze flows live in `docs/ui-workflow.md` §7; schema details stay in `crates/starbreaker-ui/docs/ir-freeze-schema.md`. |
| `docs/ui-matching-tasks/target-master-findings.md` | **Delete** (stale findings; anything still true is in the runbook/memory). |
| `HANDOFF-item2-medical2.md`, `HANDOFF-medical2-followup.md` (repo root) | **Delete**; superseded by memory + `docs/ui-clipper-parity-handoff.md`; the medical outstanding items are restated there. |
| `crates/starbreaker-ui/docs/gold-platinum-regression-deep-dive.md` | **Keep** as historical analysis; add a header line "background reading; process lives in docs/ui-workflow.md". |
| `crates/starbreaker-ui/docs/ir-freeze-schema.md`, `ir-style-authority-migration-plan.md` | **Keep** (schema + migration state), linked from the new docs. |
| `docs/ui-architecture-runbook.md`, `docs/ui-regression-policy.md`, `docs/ui-font-size-harness.md`, `docs/ui-fallback-register.md` | **Keep** as satellites; dedupe any process text that moved into `docs/ui-workflow.md`; each gets a "process: see docs/ui-workflow.md" header. |
| `crates/starbreaker-ui/AGENTS.md`, `StarBreaker/AGENTS.md`, `.github/copilot-instructions.md` | **Update** required-reads/validation-commands sections to the new docs + `ui_check.sh`. |
| Workspace `/home/tom/projects/scorg_tools/docs/` ui plans/research | Out of repo — leave, but the new docs state explicitly: *repo docs are authoritative; workspace docs are archive*. |

**Acceptance for item 11:** a fresh agent given only "read
`docs/ui-workflow.md` and `docs/ui-reference.md`, then continue
`docs/ui-clipper-parity-handoff.md`" can execute a full TDD+review cycle
without consulting any superseded doc; `grep -rn "ui-matching-workflow"`
across the repo returns only historical mentions in git log; every command
in the new docs has been executed once during writing (no untested
commands).

### 12. Keep documentation honest — verify-on-write

**Observed:** every stale-doc instance above (dead subcommands, dead env
vars, dead paths, dead probes) was a *written-but-never-rechecked* claim.

**Improvement:** two cheap rules: (a) every command line that enters a doc
is run once at writing time; (b) renames/removals of CLI subcommands,
scripts, probes, or doc files include a repo-wide grep for references in the
same commit.

**Action:** state both rules in `docs/ui-workflow.md`; add a lightweight
`tests/docs_reference_guard.rs`-style check IF cheap (greps the two new docs
for `scripts/*.sh` / example names and asserts the files exist — best-effort,
phase 4).

---

## Part B — phased plan (actionable, fresh-context-ready)

Execute phases in order; each phase is independently committable and ends
with its acceptance check. Work happens in
`/home/tom/projects/scorg_tools/StarBreaker` on `feature/ui` (no remote;
self-contained repo). General rules while executing: TDD where code changes
behaviour; `cargo build` debug for iteration; commit per phase (or per item
within a phase) with messages citing this doc's item numbers; do NOT touch
frozen baselines except where a phase explicitly says so.

### Phase 0 — tooling quick wins (items 2, 3a, 4, 6) — no behaviour changes

1. **`scripts/ui_check.sh`** (item 4): two tiers exactly as specified in
   item 4. Make it executable; echo each suite as it runs; non-zero exit on
   first failure. Verify: run both tiers green on current HEAD (the `--full`
   tier needs game data + the existing export; document that in `--help`).
2. **`scripts/ui_compare.py`** (item 2): CLI as specified; presets stored in
   the script as a dict
   (`{"power": {"emissions": (40,0,1560,170), "columns": (430,170,1430,1030),
   "scrollbar": (430,1000,1430,1080), "output_card": (60,170,560,620),
   "battery_card": (60,600,560,1060), "footer": (0,1060,1600,1200)},
   "target": {...derive from the target screen review...}}`, coordinates in
   the render's 1600×1200 space); reference auto-scaled to render width
   BEFORE cropping. Verify: run against
   `/tmp/` replay output of `Screen_Left_Lower_RTT` vs
   `reference/in-game/Clipper/Screen_Left_Lower_RTT.png`; eyeball one crop.
3. **`font_size_check.py` self-check** (item 3a): matched==0 or unexpected
   column count → print `HARNESS ERROR ...` and exit 2 (distinct from drift
   exit 1). Verify: feed it an empty file (expect exit 2) and a real dump
   (expect current behaviour).
4. **`examples/ui_stage_diff.rs`** (item 6): generalise
   `repro_emissions.rs`; flags per item 6; prints per matching node
   `id name [ty] parse=(sizing,rect) resolved=(sizing,rect)` and a final
   `FIRST DIVERGENCE: ...` line. Delete `repro_emissions.rs` in the same
   commit. Verify: run on `gen_mc_s_emissions.json` at 1458x141 — it must
   show the (now historical) Percent vs styled sizing values without
   crashing; run on a medical canvas as a second smoke test.
5. Commit: `tooling: ui_check battery, ui_compare regions, harness
   self-check, ui_stage_diff (process items 2,3a,4,6)`.

### Phase 1 — documentation consolidation (items 11, 7, 1, 8, 9, 12)

Largest phase; do it in one sitting against THIS doc's item-11 content
spec.

1. Re-read (for content to absorb/correct):
   `crates/starbreaker-ui/docs/ui-matching-workflow.md`,
   `ui-matching-agent-prompt.md`, `ui-matching-text-prompt.md`,
   `docs/ui-regression-baseline-workflow.md`, `docs/ui-regression-policy.md`,
   `docs/ui-architecture-runbook.md`, `docs/ui-font-size-harness.md`,
   `crates/starbreaker-ui/AGENTS.md`, `StarBreaker/AGENTS.md` (UI parts),
   `.github/copilot-instructions.md` (UI parts),
   `docs/ui-clipper-parity-handoff.md` (mechanisms quick reference),
   and the Claude memory file `power-screen-parity-plan.md`.
2. Write `docs/ui-workflow.md` per item 11 spec (sections 1–10). While
   writing, RUN every command (item 12a). Correct as you go: no
   `ui debug`/`ui styles` (use `ui render --dump-ir-dir` + the MCP trio);
   no mandatory `SC_DATA_P4K`; validation = `ui_check.sh`.
3. Write `docs/ui-reference.md` per item 11 spec (sections 1–8), including
   the probe registry table (item 7) and the verified MCP tool list (run
   each MCP tool once or cite a this-arc usage).
4. Apply the supersede table: deletes, the prompt rewrite, satellite-doc
   headers, AGENTS/copilot updates. Then repo-wide reference sweep:
   `grep -rn "ui-matching-workflow\|ui-matching-text-prompt\|ui-regression-baseline-workflow\|HANDOFF-item2\|HANDOFF-medical2\|ui debug\|ui styles" --include='*.md' --include='*.rs' --include='*.sh' .`
   and fix every live hit.
5. Update the Claude memory: `power-screen-parity-plan.md` and `MEMORY.md`
   gain a pointer "process docs consolidated → docs/ui-workflow.md +
   docs/ui-reference.md"; fix the `MFD_IR_DUMP_LOG` ghost-probe note
   (correct name: `ui render --dump-ir-dir`).
6. Acceptance (item 11's): fresh-context dry run — open ONLY the two new
   docs + handoff and walk one full cycle (replay render → compare → pick
   the ignored `column_zero_auto_text_children_stack_at_measured_heights`
   spec → confirm every needed command/path is present in the docs WITHOUT
   leaving them). Commit:
   `docs: consolidated UI workflow + reference; supersede ui-matching docs
   (process item 11)`.

### Phase 2 — freeze + registry automation (items 5, 10)

1. **Freeze delta audit** (item 5): extend
   `examples/freeze_ui_snapshot_ir.rs` — load the existing freeze JSON
   first; after computing the new snapshot, print per target each changed
   identity with `field: old -> new` (and ADDED/REMOVED identities); write a
   `delta` array (target, identity, field, old, new) into the freeze JSON
   next to approver/reason; exit non-zero with a clear message when invoked
   with no changes (`--allow-empty` escape hatch). TDD: a unit test against
   two small in-memory snapshots. Update `docs/ui-workflow.md` §freeze flow
   + `ir-freeze-schema.md` (schema gains `delta`); run
   `validate_ui_snapshot_freeze.sh` (extend it to tolerate/check the new
   field).
2. **Registry notes** (item 10): write
   `crates/starbreaker-ui/data/default_value_registry_v1.notes.md` now,
   seeded from current knowledge: power pins (`piplist*`,
   `pipsLengthMax`, `totalPossiblePower=16`/`availablePower=2` —
   reference-pinned, derivation TODO), `iscast=false` (engine-pushed per
   render-target type; screens=false), medical `Bed/...`/`CloneLocationInfo/...`
   keys (platinum-pinned at pre-composition paths — migrate keys before
   composing relative urlPostfix namespaces), emissions signature paths
   (now derived in ship_values.rs — note they bypass the registry),
   localization-ish entries. Link from `docs/ui-fallback-register.md`.
3. Commit per item.

### Phase 3 — approval-gated baseline refreshes (item 3b + deferred freezes)

Do NOT start without Tom's go-ahead in-session; present the deltas first.

1. **Font baseline TSV** (item 3b): rebuild debug CLI; dump from the LOD1
   scene (`SB_UI_FONT_DUMP=1 ./target/debug/starbreaker ui render --scene
   ".../DRAK Clipper_LOD1_TEX2/scene.json" --out-dir /tmp/fontcheck 2>&1 |
   grep '^FONTDUMP' > /tmp/font_dump.tsv`), show Tom the 7 drifts with the
   responsible (already-approved) changes, then replace
   `font_size_baseline.tsv` with the new dump filtered to the 4 target
   canvases and re-run `font_size_check.py` (expect PASS, 26+ matched).
2. **Power-arc artifact freeze** (deferred from the gold re-freeze): after
   the power screen work wraps — `cargo build --release -p starbreaker`;
   re-export drak_clipper; `bash scripts/generate_ui_regression_artifacts.sh`;
   `bash scripts/freeze_ui_regression_artifacts.sh --approver tom --reason
   "<cite the arc's commits>"`; both validate scripts; `ui_check.sh --full`.
   (Sequencing note: this naturally belongs at the END of the parity arc's
   step 7 in `docs/ui-clipper-parity-handoff.md` — whichever happens first
   carries it.)

### Phase 4 — adoption guard (item 12b, optional)

1. `crates/starbreaker-ui/tests/docs_reference_guard.rs`: read
   `docs/ui-workflow.md` + `docs/ui-reference.md`; extract
   `scripts/<name>.sh|py` and `examples/<name>.rs` tokens; assert each file
   exists. Keep it dumb and forgiving (only flags vanished files, not prose).
2. Add `ui_check.sh` mention + the two doc paths to the SessionStart
   knowledge: one line each in `StarBreaker/AGENTS.md` (if not already from
   phase 1).

### Execution-state tracking

When executing this plan, track progress IN THIS FILE by appending `[done
<date> <commit>]` to each numbered step, so a context-compacted or fresh
agent can resume mid-phase without re-deriving state. The companion
work-state doc for the parity arc itself remains
`docs/ui-clipper-parity-handoff.md` — do not mix the two.
