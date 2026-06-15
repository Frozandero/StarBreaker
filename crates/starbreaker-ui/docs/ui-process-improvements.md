# UI parity process — improvements and consolidation plan

> **Append-only retrospective ledger** — the dated record of process changes
> and the **append target for future end-of-arc retrospectives**
> (`crates/starbreaker-ui/docs/ui-process-retro-prompt.md`). Many `ledger
> item N` citations in the code/scripts/tests point here for provenance, so
> the file stays even though its plans are all done. File paths in earlier
> Parts reflect the layout *at that time* and some now point at docs/examples
> since deleted or moved (expected, not rot); the CURRENT doc layout is
> **Part G**. The still-open *architecture* directions (former items 16/17/18)
> have moved to `crates/starbreaker-ui/docs/ui-architecture-runbook.md`
> §"Open architecture debt" — that is the live backlog; this is the history.

A retrospective of the Clipper power-screen parity arc (2026-06-10 → 06-11)
turned into concrete process changes, followed by the **phased, actionable
plan** that implements them (§"Phased plan"). The plan is written to be
executed from fresh context: every file path, command, and acceptance
criterion is explicit.

Status: **IMPLEMENTED 2026-06-11** (commits 2c6029f49, 845612154, ace9af280, 6304571ab, 8c4352623, 5a5b51f71, 89d6a4d51 — per-step [done] markers below). The current arc's work-state handoff is separate:
`crates/starbreaker-ui/docs/ui-clipper-parity-handoff.md`.

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
`~/projects/scorg_tools/ships/dcb_canvas/libs/foundry/records`.
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
doc (`crates/starbreaker-ui/docs/ui-clipper-parity-handoff.md`) proved the right vehicle for
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
the same commits that touch the registry; `crates/starbreaker-ui/docs/ui-fallback-register.md`
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
4. **Workspace (outside the repo)**: `~/projects/scorg_tools/docs/`
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
  now** (per the owner's standing feedback) — needed only for non-default data.
- Its validation loop omits `line_count_guard` and the font harness.
- The agent-prompt template embeds a stale export invocation.
- A memory note referenced a non-existent probe (item 7).

**Improvement:** ONE authoritative, self-sufficient documentation set such
that a fresh-context agent (or a brand-new agent) can run the entire process
reading only it. Old documents are rewritten into it or deleted — no
parallel half-truths left to confuse.

**Action — write the following two documents** (do not implement until the
phased plan is executed; full content requirements below so nothing is lost):

#### `crates/starbreaker-ui/docs/ui-workflow.md` — THE process (authoritative)

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
   `crates/starbreaker-ui/docs/ui-architecture-runbook.md`: pipeline stages
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

#### `crates/starbreaker-ui/docs/ui-reference.md` — commands, tools, data (the lookup half)

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
     ~/projects/scorg_tools/ships --kind decomposed` (P4K
     auto-detected; do NOT set `RAYON_NUM_THREADS=1` except when
     benchmarking).
   - Which scene has which bindings: LOD0 = cockpit MFD screens; LOD1 =
     medical/door/annunciator etc. (list them).
3. **Comparison**: `scripts/ui_compare.py` usage + preset list; reference
   image inventory (`~/projects/scorg_tools/reference/in-game/...`
   per screen, with resolution caveats).
3b. **Screen dossier table** — ONE ROW PER KNOWN SCREEN so a per-screen
   prompt needs only the screen name: helper name | scene.json that carries
   the binding (LOD0 cockpit MFDs vs LOD1 medical/door/annunciator) |
   canvas record (name + mirror path) | reference image path | ui_compare
   preset | frozen tier/target id (if any) | open known issues pointer
   (handoff/catalog). Seed with: Screen_Left_Lower_RTT (power),
   Screen_Right_Upper_RTT (target, gold clipper_target_master), medical bed
   (platinum ui_target_a), end-of-bed (platinum ui_target_b), small door,
   annunciator master left (golds), and the remaining Clipper helpers.
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
   `~/projects/scorg_tools/ships/dcb_canvas/libs/foundry/records`
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
| `crates/starbreaker-ui/docs/ui-matching-workflow.md` | **Delete**; content absorbed (corrected) into `crates/starbreaker-ui/docs/ui-workflow.md`. Leave no stub — update every reference. |
| `crates/starbreaker-ui/docs/ui-matching-agent-prompt.md` | **Rewrite** as the SHORT PER-SCREEN PROMPT template (~20 lines): (1) read `crates/starbreaker-ui/docs/ui-workflow.md` then `crates/starbreaker-ui/docs/ui-reference.md`; (2) look up `<SCREEN>` in the reference doc's screen dossier; (3) goal = close the gap to the dossier's reference image — replay-render, run `ui_compare.py`, build/extend the diff catalog, then the TDD loop with `ui_check.sh`, guard-adjudication on trips, audited freezes only with approval; (4) per-arc variables block (`SCREEN=`, optional `HANDOFF=`, optional known-symptom list); ships with one filled-in example. |
| `crates/starbreaker-ui/docs/ui-matching-text-prompt.md` | **Delete** (text-only variant obsolete — agents in use are vision-capable; the rewritten prompt covers both). |
| `docs/ui-regression-baseline-workflow.md` | **Delete**; freeze flows live in `crates/starbreaker-ui/docs/ui-workflow.md` §7; schema details stay in `crates/starbreaker-ui/docs/ir-freeze-schema.md`. |
| `docs/ui-matching-tasks/target-master-findings.md` | **Delete** (stale findings; anything still true is in the runbook/memory). |
| `HANDOFF-item2-medical2.md`, `HANDOFF-medical2-followup.md` (repo root) | **Delete**; superseded by memory + `crates/starbreaker-ui/docs/ui-clipper-parity-handoff.md`; the medical outstanding items are restated there. |
| `crates/starbreaker-ui/docs/gold-platinum-regression-deep-dive.md` | **Keep** as historical analysis; add a header line "background reading; process lives in crates/starbreaker-ui/docs/ui-workflow.md". |
| `crates/starbreaker-ui/docs/ir-freeze-schema.md`, `ir-style-authority-migration-plan.md` | **Keep** (schema + migration state), linked from the new docs. |
| `crates/starbreaker-ui/docs/ui-architecture-runbook.md`, `crates/starbreaker-ui/docs/ui-regression-policy.md`, `crates/starbreaker-ui/docs/ui-font-size-harness.md`, `crates/starbreaker-ui/docs/ui-fallback-register.md` | **Keep** as satellites; dedupe any process text that moved into `crates/starbreaker-ui/docs/ui-workflow.md`; each gets a "process: see crates/starbreaker-ui/docs/ui-workflow.md" header. |
| `crates/starbreaker-ui/AGENTS.md`, `StarBreaker/AGENTS.md`, `.github/copilot-instructions.md` | **Update** required-reads/validation-commands sections to the new docs + `ui_check.sh`. |
| Workspace `~/projects/scorg_tools/docs/` ui plans/research | Out of repo — leave, but the new docs state explicitly: *repo docs are authoritative; workspace docs are archive*. |

**Acceptance for item 11:** a fresh agent given only "read
`crates/starbreaker-ui/docs/ui-workflow.md` and `crates/starbreaker-ui/docs/ui-reference.md`, then continue
`crates/starbreaker-ui/docs/ui-clipper-parity-handoff.md`" can execute a full TDD+review cycle
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

**Action:** state both rules in `crates/starbreaker-ui/docs/ui-workflow.md`; add a lightweight
`tests/docs_reference_guard.rs`-style check IF cheap (greps the two new docs
for `scripts/*.sh` / example names and asserts the files exist — best-effort,
phase 4).

---

## Part C — second retrospective (2026-06-12, annunciator/power arc)

Re-review of the whole process after the annunciator close-out and power
P1/P2/P5, with everything learned. Same format: incident → improvement →
action. Items 13–15 are IMPLEMENTED in this commit; 16–19 are recorded
decisions/directions.

### 13. The wrong-cwd / stale-binary trap [done 2026-06-12]

**Observed:** TWICE in one session a `cargo build` silently ran in the
wrong directory (background shells reset their cwd between commands), and
the following replay render used a STALE binary — the unchanged output
nearly mis-adjudicated a correct fix as ineffective (the styled-ImagePath
override and the EnableBackground flip both "did nothing" until rebuilt).

**Action:** `scripts/ui_render.sh` — resolves the repo root from its own
location, always builds first, prints the binary mtime, then replays.
Documented as the preferred replay entry point in `crates/starbreaker-ui/docs/ui-reference.md`
§2. General rule: wrapper scripts with self-resolved roots beat command
sequences for anything run across shell resets.

### 14. Freeze-cycle friction [done 2026-06-12]

**Observed:** the artifact freeze flow (release build → export → freeze →
two validators → full battery) is seven hand-typed steps and was run FIVE
times in one session (~10 min each). One run also failed spuriously on
stale `*-current.png` comparison leftovers ("undeclared artifact produced
in freeze scope") from an earlier validation mismatch.

**Action:** `scripts/ui_freeze_cycle.sh --approver --reason
[--skip-export]` — the whole cycle in order, with the stale-comparison
cleanup built in. The IR snapshot freeze stays separate on purpose: its
delta must be READ and accounted for, not automated past.

### 15. The photometric review method, promoted to tooling [done 2026-06-12]

**Observed:** the decisive diagnostics of this arc were pixel-ratio
measurements made with ad-hoc PIL snippets: the linear-light compositing
gap (sRGB-space blending renders the chiclet glow (15,9,3) where linear
gives (68,38,8) — matching the reference), the MissionObjectives icon slot
(all five power icons share slot-16 hue under the capture cast, distinct
from Base and Bright ON THE SAME capture), and the annunciator
backplate-vs-bloom adjudication (no-bloom zones are NEUTRAL — hue survives
any tone curve, so a warm plate cannot be hiding there).

**Method (now codified in `crates/starbreaker-ui/docs/ui-reference.md` §3):** judge hue from
R-normalised ratios, never raw values; estimate each capture's cast from a
known anchor (footer text = Base, pip slabs = Bright) before judging an
unknown colour; expect bloom to lift B near bright elements.

**Action:** `ui_compare.py --stats` prints per-region bright/dark means +
ratios for render and reference; presets added for `annunciator` and
`door`.

### 16. Reserved-ID-band fragility (per-host-type ID-band lanes)

→ Moved to `crates/starbreaker-ui/docs/ui-architecture-runbook.md`
§"Open architecture debt" (live backlog, not history). Original observation
in git: a new expanding host type shifted the shared `EXPANSION_ID_BASE`
order and stole a frozen platinum identity.

### 17. ONE brand-context resolver (architecture debt)

→ Moved to `crates/starbreaker-ui/docs/ui-architecture-runbook.md`
§"Open architecture debt". Original observation in git: ≥4 rival
brand-container selection paths exist; one improvising over a shared standard
caused the separator AEGS-divider leak.

### 18. Linear-light compositing (gated workstream, evidence strong)

→ Moved to `crates/starbreaker-ui/docs/ui-architecture-runbook.md`
§"Open architecture debt". Original observation in git: engine composites in
linear light, we blend in sRGB; white-mask glow converted (scoped), the
renderer-wide migration is gated on owner approval.

### 19. What demonstrably worked (keep doing)

- The guard battery caught both P3 blockers and the flag-directive
  medical regression BEFORE landing; every trip named its counterexample.
- The audited freeze deltas + registry `.notes.md` provenance carried six
  re-freezes and two registry flips with every line accounted for.
- The review-phase catalog kept "deferred" explicit (P6 hover artifacts
  EXCLUDED as a recorded decision, not a gap).
- Entry/data archaeology before code (the MRAI explicit-FillColor pattern,
  the misc/orig System-Icon-Color entries, the single-consumer
  EnableBackground binding) repeatedly turned "mystery constant" into
  "authored value".

---

## Part B — phased plan (actionable, fresh-context-ready)

Execute phases in order; each phase is independently committable and ends
with its acceptance check. Work happens in
`~/projects/scorg_tools/StarBreaker` on `feature/ui` (no remote;
self-contained repo). General rules while executing: TDD where code changes
behaviour; `cargo build` debug for iteration; commit per phase (or per item
within a phase) with messages citing this doc's item numbers; do NOT touch
frozen baselines except where a phase explicitly says so.

### Phase 0 — tooling quick wins (items 2, 3a, 4, 6) — no behaviour changes

1. **`scripts/ui_check.sh`** (item 4): [done 2026-06-11 2c6029f49] two tiers exactly as specified in
   item 4. Make it executable; echo each suite as it runs; non-zero exit on
   first failure. Verify: run both tiers green on current HEAD (the `--full`
   tier needs game data + the existing export; document that in `--help`).
2. **`scripts/ui_compare.py`** (item 2): [done 2026-06-11 2c6029f49] CLI as specified; presets stored in
   the script as a dict
   (`{"power": {"emissions": (40,0,1560,170), "columns": (430,170,1430,1030),
   "scrollbar": (430,1000,1430,1080), "output_card": (60,170,560,620),
   "battery_card": (60,600,560,1060), "footer": (0,1060,1600,1200)},
   "target": {...derive from the target screen review...}}`, coordinates in
   the render's 1600×1200 space); reference auto-scaled to render width
   BEFORE cropping. Verify: run against
   `/tmp/` replay output of `Screen_Left_Lower_RTT` vs
   `reference/in-game/Clipper/Screen_Left_Lower_RTT.png`; eyeball one crop.
3. **`font_size_check.py` self-check** (item 3a): [done 2026-06-11 2c6029f49] matched==0 or unexpected
   column count → print `HARNESS ERROR ...` and exit 2 (distinct from drift
   exit 1). Verify: feed it an empty file (expect exit 2) and a real dump
   (expect current behaviour).
4. **`examples/ui_stage_diff.rs`** (item 6): [done 2026-06-11 2c6029f49] generalise
   `repro_emissions.rs`; flags per item 6; prints per matching node
   `id name [ty] parse=(sizing,rect) resolved=(sizing,rect)` and a final
   `FIRST DIVERGENCE: ...` line. Delete `repro_emissions.rs` in the same
   commit. Verify: run on `gen_mc_s_emissions.json` at 1458x141 — it must
   show the (now historical) Percent vs styled sizing values without
   crashing; run on a medical canvas as a second smoke test.
5. Commit. [done 2026-06-11 2c6029f49]

### Phase 1 — documentation consolidation (items 11, 7, 1, 8, 9, 12)

Largest phase; do it in one sitting against THIS doc's item-11 content
spec.

1. [done 2026-06-11] Re-read (for content to absorb/correct):
   `crates/starbreaker-ui/docs/ui-matching-workflow.md`,
   `ui-matching-agent-prompt.md`, `ui-matching-text-prompt.md`,
   `docs/ui-regression-baseline-workflow.md`, `crates/starbreaker-ui/docs/ui-regression-policy.md`,
   `crates/starbreaker-ui/docs/ui-architecture-runbook.md`, `crates/starbreaker-ui/docs/ui-font-size-harness.md`,
   `crates/starbreaker-ui/AGENTS.md`, `StarBreaker/AGENTS.md` (UI parts),
   `.github/copilot-instructions.md` (UI parts),
   `crates/starbreaker-ui/docs/ui-clipper-parity-handoff.md` (mechanisms quick reference),
   and the Claude memory file `power-screen-parity-plan.md`.
2. [done 2026-06-11 845612154] Write `crates/starbreaker-ui/docs/ui-workflow.md` per item 11 spec (sections 1–10). While
   writing, RUN every command (item 12a). Correct as you go: no
   `ui debug`/`ui styles` (use `ui render --dump-ir-dir` + the MCP trio);
   no mandatory `SC_DATA_P4K`; validation = `ui_check.sh`.
3. [done 2026-06-11 845612154] Write `crates/starbreaker-ui/docs/ui-reference.md` per item 11 spec (sections 1–8), including
   the probe registry table (item 7) and the verified MCP tool list (run
   each MCP tool once or cite a this-arc usage).
4. [done 2026-06-11 845612154] Apply the supersede table: deletes, the prompt rewrite, satellite-doc
   headers, AGENTS/copilot updates. Then repo-wide reference sweep:
   `grep -rn "ui-matching-workflow\|ui-matching-text-prompt\|ui-regression-baseline-workflow\|HANDOFF-item2\|HANDOFF-medical2\|ui debug\|ui styles" --include='*.md' --include='*.rs' --include='*.sh' .`
   and fix every live hit.
5. [done 2026-06-11] Update the Claude memory: `power-screen-parity-plan.md` and `MEMORY.md`
   gain a pointer "process docs consolidated → crates/starbreaker-ui/docs/ui-workflow.md +
   crates/starbreaker-ui/docs/ui-reference.md"; fix the `MFD_IR_DUMP_LOG` ghost-probe note
   (correct name: `ui render --dump-ir-dir`).
6. [done 2026-06-11 845612154 — reference check scripted, dry-run via the docs-only check; the guard test (8c4352623) keeps it honest] Acceptance (item 11's, strengthened): the deliverable is the SHORT
   PROMPT — instantiate the rewritten template for one screen (e.g.
   `SCREEN=Screen_Left_Lower_RTT`) and dry-run it from fresh context (or a
   subagent): with ONLY that prompt, the agent must reach a replay render,
   a region comparison, a catalog entry, and the start of a TDD fix (e.g.
   the ignored `column_zero_auto_text_children_stack_at_measured_heights`
   spec) WITHOUT leaving the two docs + dossier for any command or path.
   Any lookup that forces an excursion is a doc bug — fix it before
   closing the phase. Commit:
   `docs: consolidated UI workflow + reference; supersede ui-matching docs
   (process item 11)`.

### Phase 2 — freeze + registry automation (items 5, 10)

1. [done 2026-06-11 ace9af280] **Freeze delta audit** (item 5): extend
   `examples/freeze_ui_snapshot_ir.rs` — load the existing freeze JSON
   first; after computing the new snapshot, print per target each changed
   identity with `field: old -> new` (and ADDED/REMOVED identities); write a
   `delta` array (target, identity, field, old, new) into the freeze JSON
   next to approver/reason; exit non-zero with a clear message when invoked
   with no changes (`--allow-empty` escape hatch). TDD: a unit test against
   two small in-memory snapshots. Update `crates/starbreaker-ui/docs/ui-workflow.md` §freeze flow
   + `ir-freeze-schema.md` (schema gains `delta`); run
   `validate_ui_snapshot_freeze.sh` (extend it to tolerate/check the new
   field).
2. [done 2026-06-11 6304571ab] **Registry notes** (item 10): write
   `crates/starbreaker-ui/data/default_value_registry_v1.notes.md` now,
   seeded from current knowledge: power pins (`piplist*`,
   `pipsLengthMax`, `totalPossiblePower=16`/`availablePower=2` —
   reference-pinned, derivation TODO), `iscast=false` (engine-pushed per
   render-target type; screens=false), medical `Bed/...`/`CloneLocationInfo/...`
   keys (platinum-pinned at pre-composition paths — migrate keys before
   composing relative urlPostfix namespaces), emissions signature paths
   (now derived in ship_values.rs — note they bypass the registry),
   localization-ish entries. Link from `crates/starbreaker-ui/docs/ui-fallback-register.md`.
3. Commit per item.

### Phase 3 — approval-gated baseline refreshes (item 3b + deferred freezes)

Do NOT start without the owner's go-ahead in-session; present the deltas first.

1. [done 2026-06-11 5a5b51f71 — approval: the owner's 'fully implement this plan'; 7 deltas quoted in the commit] **Font baseline TSV** (item 3b): rebuild debug CLI; dump from the LOD1
   scene (`SB_UI_FONT_DUMP=1 ./target/debug/starbreaker ui render --scene
   ".../DRAK Clipper_LOD1_TEX2/scene.json" --out-dir /tmp/fontcheck 2>&1 |
   grep '^FONTDUMP' > /tmp/font_dump.tsv`), present the 7 drifts with the
   responsible (already-approved) changes, then replace
   `font_size_baseline.tsv` with the new dump filtered to the 4 target
   canvases and re-run `font_size_check.py` (expect PASS, 26+ matched).
2. [done 2026-06-11 89d6a4d51 — release re-export produced byte-identical target PNGs (hashes unchanged); freeze metadata refreshed; ui_check --full ALL GREEN] **Power-arc artifact freeze** (deferred from the gold re-freeze): after
   the power screen work wraps — `cargo build --release -p starbreaker`;
   re-export drak_clipper; `bash scripts/generate_ui_regression_artifacts.sh`;
   `bash scripts/freeze_ui_regression_artifacts.sh --approver owner --reason
   "<cite the arc's commits>"`; both validate scripts; `ui_check.sh --full`.
   (Sequencing note: this naturally belongs at the END of the parity arc's
   step 7 in `crates/starbreaker-ui/docs/ui-clipper-parity-handoff.md` — whichever happens first
   carries it.)

### Phase 4 — adoption guard (item 12b, optional)

1. [done 2026-06-11 8c4352623] `crates/starbreaker-ui/tests/docs_reference_guard.rs`: read
   `crates/starbreaker-ui/docs/ui-workflow.md` + `crates/starbreaker-ui/docs/ui-reference.md`; extract
   `scripts/<name>.sh|py` and `examples/<name>.rs` tokens; assert each file
   exists. Keep it dumb and forgiving (only flags vanished files, not prose).
2. [done 2026-06-11 845612154 — covered by the Phase 1 AGENTS.md update] AGENTS.md mentions.

### Execution-state tracking

When executing this plan, track progress IN THIS FILE by appending `[done
<date> <commit>]` to each numbered step, so a context-compacted or fresh
agent can resume mid-phase without re-deriving state. The companion
work-state doc for the parity arc itself remains
`crates/starbreaker-ui/docs/ui-clipper-parity-handoff.md` — do not mix the two.

---

## Part D — third retrospective (2026-06-12, hard-coding remediation + MFD text-size arc)

Evidence base: the 2026-06-12 session (commits 5bf1d7f84 … 891f20725 —
RgbaColor guard/fixture, BB-enum colour realignment, the text-format style
route + freeze, eight empirically-deleted rules, the MFD
`imageSizePercent` host division + artifact re-freeze, the Ruffle
post-mortem). Implementation plan for everything below:
**`docs/ui-improvement-plan.md`** (checkbox-phased; less-capable-agent
detail). Items here record the findings; the plan carries the work.

### 19. Ad-hoc pixel measurement is the top error source → measurement tool + bank

**Observed:** ~15 throwaway python snippets were written in one session to
measure glyph cap heights, row/column runs, and colour ratios. Two of them
produced WRONG conclusions that cost hours: the power footer "ink ×1.4"
claim (the measurement box included the footer bar line — the real glyph
cap was 52, not 81) and a "2"-glyph cap of 218 (box caught the card
underline). The capture-cast colour method (ui-reference §3) also had to be
re-derived as an ADDITIVE-haze model mid-session.

**Improvement:** one measurement tool with contamination guards (single
glyph-run extraction, bar-line exclusion, per-element boxes from IR rects)
and a once-per-capture **measurement bank** checked in as a provenance
fixture, so adjudications become lookups instead of re-measurements.

**Action:** plan Phase 1 (`scripts/ui_measure.py`, measurement-bank
fixtures, rectification — see item 21).

### 20. Stale exported PNGs silently mis-adjudicate → export stamp + hard staleness failure

**Observed:** the whole-image visual guard compares
`ships/Data/UI/Generated/*.png`, which only refresh on export. A rule
deletion (`ScreenNameBackground` suppression) was judged "zero drift"
against WEEKS-old files; a fresh export showed a real regression
(annunciator strip warm haze). The guard gave a wrong-but-plausible PASS —
the exact failure class of item 3.

**Improvement:** the export writes a stamp; the visual guard HARD-FAILS
when the compared PNGs predate the stamp's expectations or the stamp is
missing/stale. Docs already say "re-export before any artifact comparison
(~50s)" — the guard must enforce it.

**Action:** plan Phase 0.

### 21. Reference perspective skew burns sessions → rectification helper

**Observed (owner):** captures are manual, often low-res and
non-perpendicular, compensated by hand in GIMP. The power-screen padding
investigation burned hours inside ±10px skew ambiguity; several layout
models were UNDECIDABLE against the capture.

**Improvement:** `ui_compare.py --rectify` — a homography from four
screen-corner points (clicked once per capture, persisted next to the
reference) rectifies the capture to render dimensions before comparison.

**Action:** plan Phase 1.

### 22. The empirical disable→adjudicate audit — codify with its limits

**Observed:** disabling a suspect rule and letting the frozen pins
adjudicate proved EIGHT hard-coded rules deletable and THREE load-bearing
in one session — far cheaper than per-rule reference archaeology. But its
verdict is only "no FROZEN PIN references this" (five screens, one ship),
and item 20 shows it can lie when comparison inputs are stale.

**Improvement:** document the method in `crates/starbreaker-ui/docs/ui-workflow.md` §5 with its
two preconditions (fresh export; lib+IR+visual suites all consulted) and
its scope caveat.

**Action:** plan Phase 6.

### 23. IR-dump query helpers lived in /tmp → promote

**Observed:** `/tmp/irq.py` (per-node rect/payload/tag queries over a
`--dump-ir-dir` JSON) and `/tmp/irtree.py` (ancestor-chain printer) were
rewritten from memory during the session and used dozens of times.

**Action:** plan Phase 1 (`scripts/ui_ir_query.py`).

### 24. Engine-model knowledge discovered this arc is not in the runbook

**Observed:** four hard-won models live only in code comments / the
handoff: (a) authored TRBL padding scales with the canvas geometry scale
(×2 on the 800×600 MFD content canvas — pinned by pip-top/stride
measurements); (b) `Parent(...)`-wrapped style entries select the
textfield's implicit TEXT-FORMAT child (the text-format route, with the
literal-match precedence caveat from T3); (c) the GFx-host
`imageSizePercent` division applies to ALL size classes on the host path;
(d) the additive-haze photometric model for capture colour adjudication.
Plus the Ruffle/AVM1 facts (now in the runbook §"Why Ruffle…").

**Action:** plan Phase 6 (runbook "engine model" entries with their
citations).

### 25. Probe output channels are inconsistent

**Observed:** `BB_A3_STYLE_PROBE` logs via `log::info!` (invisible without
`RUST_LOG=info`, which cost a blank-output round trip); `BB_TEXT_FORMAT_PROBE`
prints via `eprintln!` (always visible). The probe registry doesn't say
which is which.

**Improvement:** standardize env-gated probes on `eprintln!`; note the
channel per probe in the registry table.

**Action:** plan Phase 0.

### 26. Examples are outside the check battery → freeze runs break late

**Observed:** the IR snapshot freeze failed mid-flow because two EXAMPLES
(`freeze_ui_snapshot_ir.rs`, `dump_ui_ir_targets.rs`) still called the
deleted `drake_amber_fallback` — `ui_check.sh` never compiles examples.

**Improvement:** `cargo check -p starbreaker-ui --examples` joins the
battery (TDD tier — it is fast).

**Action:** plan Phase 0.

### 27. Truth mining beats capture calibration — AVM1 facts verified

**Observed:** the costliest derivations (44px content inset, slider
formula, text scale) are runtime ActionScript. VERIFIED this session:
`BuildingBlocks_root.swf` is SWF v8 / **AVM1** (127 `DoInitAction` tags,
no DoABC); the already-vendored `swf = "0.2"` crate exposes
`avm1::types::{Action, ConstantPool, Push}`; the file extracts in <1s via
`starbreaker p4k extract --output /tmp/bbswf --filter
"**/BuildingBlocks_root.swf"`.

**Improvement:** a static AVM1 constant/bytecode dumper turns measured
framework constants into data-derived ones.

**Action:** plan Phase 2.

---

## Part E — fourth retrospective (2026-06-12, plan-execution session: P0–P2, P4.1, P5.1/2/4, P6)

Evidence base: the second 2026-06-12 session (commits bc38ec8bd …
9d140f254 — the improvement plan's phases 0/1/2/6 complete, P4.1, the
P5 derivation items). Items 28–31 were implemented in the same session;
nothing here extends the phased plan (the remaining plan work is
`docs/ui-improvement-plan.md` P3/P4.2–4.4/P5.3).

### 28. A commit cannot cite its own hash → checkbox convention

**Observed:** the plan's discipline said mark checkboxes `[x] (<date>
<commit>)` in the same commit as the work — impossible for the commit's
own hash. The session settled on `[x] (<date>, commit "plan PX.Y")` with
every commit message citing the step, making the mapping greppable
(`git log --grep "plan P1.2"`).

**Improvement:** future plan templates should specify the step-citation
form, not a hash.

**Action:** [done 2026-06-12 — this entry records the convention; the
improvement plan's checkboxes all use it.]

### 29. AVM1 mining outcomes: one confirm, two C++-side bounds

**Observed:** the item-27 dumper worked first try (127 classes), but the
mined targets mostly were NOT in the bytecode: the 44px content-view
inset and the scrollbar `_SizeRatio` are engine-pushed (C++); the CLIK
thumb formula CONFIRMED our viewport/content model; AS2 applies text
sizes verbatim (negative-confirms the imageSizePercent host division).
A miss with a recorded bound is a real result — both pins now carry
"PROVEN absent from the bytecode" provenance.

**Improvement:** when proposing future truth-mining, scope expectations:
BB runtime values live mostly C++-side; the SWF carries framework
FORMULAS (CLIK components), not layout constants.

**Action:** [done 2026-06-12 — recorded in the runbook's "AVM1 mining
results" + the fallback register entries.]

### 30. The staleness guard fires mid-battery → preflight visibility

**Observed:** the P0.2 guard (correctly) failed `--full` runs twice this
session, but only AFTER minutes of suites had already run, because the
test binary rebuild happened >30min after the last export.

**Improvement:** `ui_check.sh --full` prints the export-stamp age up
front and warns when >30min, so the ~50s re-export happens before the
battery, not after a failed one. Warning only — the in-guard hard-fail
stays authoritative.

**Action:** [done 2026-06-12 — ui_check.sh preflight, this session.]

### 31. Helper-less bindings render to GUID-named PNGs → unfindable

**Observed:** mapping the medical-bed replay output to its binding
required a pixel content-match hunt: bindings without `helper_name`
(the bed) wrote `<canvas-guid>_TEX0.png`.

**Improvement:** `png_name_for_binding` falls back helper_name →
content_canvas_record_name → canvas_record_name → guid, so replay
outputs are human-findable.

**Action:** [done 2026-06-12 — cli/src/ui.rs + unit test; replay-only
naming, exports unaffected.]

---

## Part F — fifth retrospective (2026-06-13, P3 text-calibration + P4 cascade-unification session)

Evidence base: the back-to-back P3/P4 session (commits 109a15afb …
through the P4.4 close). The arcs themselves landed cleanly (five text
constants retired for derived models; the cascade unified on one
`bb_style_engine` with byte-identical output); the process finding below
is the one that cost real time.

### 32. A test silently stopped running for two commits → orphaned-#[test] guard

**Observed:** inserting a new test ABOVE an existing one (P3.2) placed the
new test's doc-comment between the existing `#[test]` and its function,
orphaning that attribute. The OUTPUT-card spec test
(`auto_text_children_flow_at_measured_widths`) thereby stopped running and
shipped dead in the P3 commit. `cargo` emitted only a non-fatal
"duplicated attribute" + "never used function" warning, and the battery
does not fail on warnings, so it was invisible until a `--tests` build was
eyeballed during P4.3 cleanup. (Re-enabling it confirmed it still passed —
no regression hid behind it — but that was luck, not process.)

**Improvement:** `tests/test_attribute_guard.rs` — a deterministic,
cache-independent guard: a `#[test]` line whose next non-blank line is a
`///` doc comment is orphaned. Wired into `ui_check.sh`'s TDD tier so it
runs every cycle (warnings-based detection is unreliable because cargo
only re-emits them on recompile).

**Action:** [done 2026-06-13 — guard + battery wiring this session; the
detector's own unit test pins the exact pattern that bit us.]

### 33. P4.4 audit honoured the scope caveat over the literal instruction

**Observed:** P4.4 said "delete what the engine now covers." The
disable→adjudicate showed NO frozen pin depends on the RootGhost
name-pluck — literally "nothing trips, so delete." But it serves
non-frozen brands (aegs/bioc/crus/orig/crlf with RootGhost radii) and no
frozen target has a ghost button to verify the engine's condition-matched
application reproduces it. The §5 caveat (a clean audit proves "no frozen
pin", not "correct everywhere", item 22/P6.2) correctly overrode the
literal "delete" — KEPT with a documented deletion criterion.

**Improvement:** none needed — the caveat worked as written. Recorded as a
worked example: when disable→adjudicate is clean BUT the rule serves
out-of-frozen-set cases, the verdict is keep-with-criterion, not delete.

**Action:** [done 2026-06-13 — RootGhost kept + documented; criterion at
the function and in the plan's P4.4 checkbox.]

### 34. The battery ran a hand-picked test subset → run the whole crate suite

**Observed (2026-06-13):** `ui_check.sh` ran a hand-picked `--test` list
(`manifest_live_ir_guard`, `line_count_guard`, `test_attribute_guard`,
`--lib`, and in `--full` the snapshot/visual suites). ~14 integration
targets — `pipeline_ir`, `swf_*`, `brand_palette_resolution`,
`pipeline_mfd_frame`, `regression_hashes`, `source_hardcoding_guards`,
`docs_reference_guard`, `ui_ir_representative`, `visual_diff` — were NEVER
run by the battery. That gap hid two real defects until a manual
`--tests` build: `swf_phase5_wiring` had not COMPILED since the
`colour_overlay_enabled` field landed, and `pipeline_ir`'s style-override
test silently REGRESSED at the P5.3 slot-8→9 change. Both are exactly the
class the battery exists to catch.

**Improvement:** `ui_check.sh` runs the WHOLE crate suite
(`cargo test -p starbreaker-ui`) in both tiers — auto-covering every
current and future target, no fragile enumeration. The only export-coupled
guards (whole-image colour + custom-shape) are gated by
`UI_SKIP_VISUAL_GUARD=1` in the TDD tier (they need a fresh export) and run
authoritatively in `--full`.

**Action:** [done 2026-06-13 — ui_check.sh restructure + the skip hook in
manifest_visual_regression.rs.]

---

## Part G — UI documentation consolidation (2026-06-13)

All UI documentation now lives under **`crates/starbreaker-ui/docs/`** (one
location). This supersedes Part C's doc-disposition table where it said to
"keep" `gold-platinum-regression-deep-dive.md` and the
`ir-style-authority-migration-plan.md` / artifact plans — those are now
removed (their work is complete and captured here / in git).

**Deleted (implemented or superseded; git preserves them):**
`ui-improvement-plan.md` (all phases done), `ir-style-authority-migration-plan.md`
(Phase C complete — IR is the styling authority), the two
`docs/StarBreaker/ui-rework-artifacts/` plans (target screen frozen, hybrid
rendering shipped), `gold-platinum-regression-deep-dive.md` (historical;
policy lives in `ui-regression-policy.md`), and
`medical-snapshot-baseline-workflow.md` (folded into `ui-regression-policy.md`
§"Gold-standard targets and tier selection").

**Relocated `docs/` → `crates/starbreaker-ui/docs/`:** ui-workflow, ui-reference,
ui-architecture-runbook, ui-fallback-register, ui-regression-policy,
ui-font-size-harness, ui-cascade-passes, ui-process-improvements (this file),
ui-clipper-parity-handoff, and the perf baseline (→ `ui-perf-baseline.md`).
The agent prompts (`ui-matching-agent-prompt.md`, `ui-process-retro-prompt.md`)
and `ir-freeze-schema.md` were already there.

**Convention going forward:** new UI docs go in `crates/starbreaker-ui/docs/`
and are referenced by full repo-relative path (`crates/starbreaker-ui/docs/
ui-X.md`) so `docs_reference_guard` validates them. Repo-level `docs/` holds
only cross-cutting / non-UI docs (the decomposed-export contract, blender
material/shader docs, animation + lighting research). The `*.notes.md`
provenance sidecars stay co-located with their fixtures/data (registry
pattern), not in docs/.

---

## Part H — sixth retrospective (2026-06-13, power SpaceBetween + colour-gate session)

Evidence base: the `Screen_Left_Lower_RTT` (power MFD) parity arc — landed
`SpaceBetween`/`SpaceAround`/`SpaceEvenly` flex justification (`d2d26297e`,
P13 position) and the shared-tier background suppression that made the header
side bars render their authored red (`2c2f50b72`, P13 colour). Both went through
the normal TDD/guard flow and landed clean (no frozen regression). The findings
below cost real time; #35 produced a wrong verdict the owner had to overturn.

### 35. The rectified reference silently smears thin-feature colour → wrong "faithful" verdict

**Observed:** I judged the header side bars' colour with the photometric anchor
method (item 27) but ran it on the RECTIFIED reference (item 21's homography
output). The bars are 2px wide; the warp interpolates them with the orange
screen-grid behind, lifting the measured G/R to 0.64 — indistinguishable from
the same-capture Base anchor (footer text, 0.63). I recorded the orange bars as
FAITHFUL. They are not: the nodes AUTHOR `background.color` = Accent1/Accent2,
the CRISP original measures G/R 0.52 / B/R 0.31 = Accent1, and the owner saw red
at a glance. The anchor method was sound; it ran on the wrong image.
Rectification is right for POSITION and wrong for THIN-FEATURE COLOUR, and
nothing flagged the dilution.

**Improvement:** (a) `ui-workflow.md` §10 don't-retry rule + `ui-reference.md`
§3 caveat: a thin feature's (≤~4px: bars, strokes, dotted separators) COLOUR is
judged on the CRISP ORIGINAL, never the rectified capture. (b)
`scripts/ui_measure.py` reports `feature_width` and warns (JSON + stderr) when it
is ≤4px.

**Action:** [done 2026-06-13 — ui_measure feature_width + warning; workflow §10
+ reference §3/§7 caveats; verified the warning fires on the 2px bar and stays
quiet on the 220px OUTPUT text.]

### 36. IR descendant dumps were re-typed all session → `ui_ir_query.py children`

**Observed:** `ui_ir_query.py` had `query` (flat regex) and `tree` (ancestor
chain) but no DESCENDANTS view. Tracing the pip-column clip, the heat-bar
overflow, the battery container stack and the four separators, I hand-wrote
inline `parent→children with rect/clip/overflow/is_active` python at least six
times — the most-retyped diagnostic of the session.

**Improvement:** `ui_ir_query.py children <ir.json> <node_id> [--depth N]
[--fields a.b,c]` — the descendant subtree (rect, `right`=x+w, is_active, a
non-Visible overflow mode), the mirror of `tree`.

**Action:** [done 2026-06-13 — subcommand added; verified on the power IR
(canvas_PowerSystems subtree); reference §7 row added.]

### 37. Ad-hoc rectify-and-crop was re-typed all session → `ui_compare.py --box`

**Observed:** `ui_compare.py` compared only the fixed PRESET regions. To compare
an arbitrary rectangle (header bars, right pip area, battery/output cards) of
render vs rectified-reference I imported the module, called `rectify_reference`
and hand-cropped with PIL ~ten times.

**Improvement:** `ui_compare.py --box x0,y0,x1,y1` (repeatable) — a stacked
render|rectified-ref crop of an ad-hoc region, honouring `--rectify`/`--stats`,
reusing the existing rectify+stack+stats helpers.

**Action:** [done 2026-06-13 — `--box` added; verified on the header region;
reference §3 usage line added.]

### 38. Cascade-tier + brand-index facts re-derived → runbook engine model

**Observed:** re-derived from scratch the facts the colour fix turned on: the
emissions header bars AUTHOR Accent1/Accent2 (the shared `mfd_g_emissions`
"New Style" overrode them to Base); the emissions brandStyles index→brand map
(`brandStyles[1]` = `s_drak_hud` = DRAK, which authors NO separator colour — the
Accent1/visibility separator entries live in `s_argo_hud`/`s_grin_hud`, never
selected for the Clipper); and the cascade-tier override semantics.

**Improvement:** `ui-architecture-runbook.md` engine-model bullet — a shared-tier
`BackgroundColor` does not override a custom shape's authored colour
(`shared_background_override_suppressed`), with the separator authored accents
and the brand-index note. The "emissions header bars are Base/orange" premise is
recorded as WRONG (authored Accent).

**Action:** [done 2026-06-13 — runbook engine-models bullet added.]

### 39. The dossier pointed at a non-rectifiable reference → corners-variant note

**Observed:** the dossier lists `Screen_Left_Lower_RTT.png`, but only the `_dark`
capture carries a `.corners.json` sidecar; a fresh agent following the dossier
gets the un-rectifiable legacy capture (and the per-screen prompt overrode it to
`_dark.png` without the dossier explaining why).

**Improvement:** `ui-reference.md` §3 dossier intro — prefer the straight-on
capture with a `<name>.corners.json` sidecar (auto-rectified); the power screen's
is `Screen_Left_Lower_RTT_dark.png`.

**Action:** [done 2026-06-13 — reference §3 dossier note.]

### 40. MCP style tools rejected the mirror file path → GUID note

**Observed:** `ui_scene_style_probe` / `ui_canvas_style_inventory` returned
`canvas_not_found` when given the dcb_canvas mirror file PATH; they want the
record GUID/name (resolvable from the file's `_RecordId_`/`_RecordName_`).

**Improvement:** `ui-reference.md` §4 — the `canvas` arg is the record GUID/name,
not the mirror path; get it from the file's top-of-file `_RecordId_`/`_RecordName_`.

**Action:** [done 2026-06-13 — reference §4 note.]

### 41. The cascade probe showed entry NAMES but not what they SET → modifier summary

**Observed:** the single biggest time-sink was tracing WHY a node resolved a
colour. `BB_A3_STYLE_PROBE` printed `matches=["New Style"]` (the entry name) but
not its modifier, so confirming the Sep1/Sep4 override meant separately
re-reading the `mfd_g_emissions` record to find "New Style" = `BackgroundColor=Base`.

**Improvement:** the probe appends each matched entry's key modifiers
(`probe_modifier_summary`): colour modifiers as `Field=Token`, IsActive / Size* /
Anchor* as `Field=value` — `New Style[BackgroundColor=Base]`,
`Vertical Separator 1[IsActive=false]`. Probe-gated (render-neutral).

**Action:** [done 2026-06-13 — bb_brand_apply probe + reference §6 row; verified
`New Style[BackgroundColor=Base]` on the power render.]

### Phase H — implementation (all done 2026-06-13, this session)

Render-neutral tooling + docs only (no freeze/baseline touched; the SpaceBetween
and colour fixes themselves went through the arc's TDD/guard flow, not this
retro). `ui_check.sh` green after the doc edits (docs_reference_guard included).
1. `ui_ir_query.py children` (item 36). [done]
2. `ui_compare.py --box` (item 37). [done]
3. `ui_measure.py` feature_width + thin-feature warning (item 35 tooling). [done]
4. `BB_A3_STYLE_PROBE` matched-entry modifier summary (item 41). [done]
5. Docs: workflow §10 + reference §3/§4/§6/§7 (items 35–37, 39–41); runbook
   engine model (item 38). [done]

### Phase H — deferred follow-ups (items A–C, all done 2026-06-13)

Three retro findings needed code (not just docs/tooling) and were deferred from
the Phase-H batch above so the gaps could be researched first. All three are
render-neutral (a new opt-in probe field, an MCP label, a script selector — no
freeze/baseline touched). `ui_check.sh` green after each.

- **Item C — `ui_render.sh` scene selection.** The replay script always took an
  explicit `--scene`; the LOD0 (cockpit `_RTT`) vs LOD1 (interior usable) split
  was tribal knowledge. Gap research: confirmed the helper-name `*_RTT` → LOD0
  rule against the dossier. Now `--scene` wins, else `--lod 0|1`, else derived
  from the helper. [done — commit `264d0330c`; reference §2.]
- **Item B — brand per `brandStyles[N]`.** `ui_canvas_style_inventory` listed
  `brandStyles[]` by index only, so confirming "is this the s_drak_hud brand?"
  meant a separate `datacore_record` read. Gap research: the brand is the
  `brandIdentifier` path basename on each entry. `brand_style_label(brand, idx)`
  now renders `brandStyles[1] s_drak_hud`. [done — commit `010df3214`; unit test
  `brand_style_label_uses_brand_identifier_basename`; reference §4.]
- **Item A — winning style-cascade source per field.** The single biggest parity
  time-sink (item 41's sibling): a node resolved a colour and you had to infer
  WHICH pass/entry set it. Gap research: where to record without polluting the
  freeze — recording must sit AFTER `apply_modifier` and inside the
  non-suppressed branch, or a shared override that LOST the colour gate is
  falsely credited (caught and fixed: Sep1 stamps null, not `New Style/Base`).
  `SB_UI_STYLE_PROVENANCE=1` stamps `node.raw["__StyleProvenance"][field] =
  "pass/entry"`; surfaced as the always-None-in-normal-compiles
  `UiIrNode.style_provenance`, queryable with `ui_ir_query.py … --fields
  style_provenance`. [done — commit `e788b1470`; probe §6, IR field §7; verified
  80 genuine fields on the power render, e.g. `PipBox_Fill BackgroundColor =
  s_drak_hud/PipBox_Fill_Unpowered`.]

## Arc — MFD aspect / content-scaling + step-3 hand-off (2026-06-14)

Power-screen card width + battery icon (landed/frozen: commits `1177002ff`,
`58e5b574b`, `d50afa34f`) via the data-driven AspectRatioToTag → "Content Canvas
Scaling" mechanism; step-3 (square-screen aspect) scoped + handed off. The retro
findings below are the friction THIS arc paid for.

### 42. A slow diagnostic harness read as an infinite loop → near-reverted a correct fix

**Observed:** the single biggest time-sink of the arc. `mfd_ir_dump` takes ~94s
because its `Fs` fetcher walks + parses the ENTIRE decompiled record mirror at
startup before compiling anything. With no progress output, a run sat at ~99% CPU
/ ~198MB for >60s and I read it as an infinite LAYOUT loop from the new
`PercentOfY` content width — killed it repeatedly, tried three "fixes" for a
non-existent cycle, and almost shelved the (correct) change as unshippable. Ground
truth came only from timing a run to completion (94s, exit 0) and from the real
export path (`ui render` / `entity export`, DataCore fetcher) rendering the same
screen in **9s**. There was never a loop.

**Improvement:** `mfd_ir_dump` prints a startup banner (record count + mirror
load time) and the IR compile time, so slow≠hang is unmistakable; a §10
don't-retry entry codifies "high CPU/RSS on a mirror-backed example is harness
load, not a pipeline loop — time it to completion or use the real fetcher."

**Action:** [done 2026-06-14 — banner (Phase I). SPEEDUP follow-up 2026-06-14:
`mfd_ir_dump` ~94s→**5s** — index only the UI subtrees (`ui/`, `tagdatabase/`,
`scitemdisplayscreenpreset/`: 60k→5.7k files), parallel head-scan for
`_RecordId_`/`_RecordName_` (index 90s→0.0s), and a memoising `Fs` fetcher with a
shared-`Rc` path (the uncached 6.2MB TagDatabase re-parse was the compile cost:
~2min→4.1s). Docs steer routine IR inspection to the indexed tools (`ui_ir_query`,
`ui render --dump-ir-dir`); mfd_ir_dump is the no-P4K/no-MCP fallback. rayon added
as a dev-dependency.]

### 43. `ui_ir_query` (MCP) silently renders the PRE-content-scaling layout

**Observed:** `ui_ir_query` could not verify the content-scaling change because the
MCP canvas fetcher (`mcp/src/tools.rs :: find_by_name`) searches ONLY
`BuildingBlocks_Canvas`, so `AspectRatioToTag_MFD` (a
`BuildingBlocks_AspectRatioLibrary`) does not resolve and
`apply_mfd_content_canvas_scaling` no-ops — `ui_ir_query` returns the unscaled
(narrow-card) layout with no error. Wrong-but-plausible: it looks like the change
"didn't take" when the tool simply can't see it. (The EXPORT fetcher,
`starbreaker-3d/src/ui_pipeline.rs :: datacore_ui_lookup_type_names`, WAS extended
to index the library; the MCP fetcher was not.)

**Improvement:** mirror the family index into the MCP fetcher so `ui_ir_query`
exercises the same pipeline as the export; until then a §10 note warns that
`ui_ir_query` does not exercise the aspect-tag content-scaling path.

**Action:** [done 2026-06-14 — §10 note + MCP `P4kCanvasFetcher` now indexes
`BuildingBlocks_AspectRatioLibrary` (`mcp/src/tools.rs`, `find_by_guid`/
`find_by_name` search a `lookup_struct_ids` Vec); MCP rebuilt + redeployed
(`mcp/starbreaker-mcp`). Phase I.]

### 44. Registering a NEW frozen target is an undocumented multi-step sequence

**Observed:** freezing the power screen as gold took a confusing detour.
`ui_freeze_cycle.sh` froze the IMAGE artifact fine but then HARD-FAILED validation
with "snapshot freeze ids do not match manifest ids" — because adding a manifest
entry also requires the SEPARATE `freeze_ui_snapshot_ir.sh` (the cycle
deliberately omits it), and the `manifest_contains_expected_visual_targets` test
hard-codes the target COUNT (`== 5`), so a 6th target fails it. The full sequence
(manifest entry → `ui_freeze_cycle` → `freeze_ui_snapshot_ir` → bump the count
assert) was tribal.

**Improvement:** document the "register a new gold/platinum target" sequence in
the reference freeze section; note the hard-coded count bump. (Optional follow-up:
`ui_freeze_cycle` could detect a manifest-vs-snapshot id delta and tell you to run
the snapshot freeze instead of failing opaquely.)

**Action:** [done 2026-06-14 — reference freeze sequence + `ui_freeze_cycle.sh`
pre-check that names any manifest target lacking an IR-snapshot baseline and
points at `freeze_ui_snapshot_ir.sh` (exits before the opaque validator). Phase I.]

### 45. Dim (alpha-0.2) glyph width is not measurable from the PNG

**Observed:** measuring the battery icon (its card renders at alpha 0.2,
"depleted" at rest) by pixel-scanning the export PNG gave garbage — colour-
deviation thresholds caught only the dense core (35–53px of a real ~67px glyph),
and the glyph also MOVES as the card width changes, so fixed crop boxes missed it.
The reliable signal was the icon's DRAW RECT (`iw`/`ih`) from a temporary
render-time `eprintln` in the custom-shape path, swept across candidate values in
one build via an env factor.

**Improvement:** a permanent, env-gated custom-shape draw-rect probe (the
throwaway `BB_ICON_PROBE2` made durable) so element width is read from layout, not
scraped from dim pixels; a §10 note steers future measuring to the probe.

**Action:** [done 2026-06-14 — §10 note + durable env-gated `BB_DRAW_RECT_PROBE`
in the custom-shape draw path (`ir_compose/.../engine_01.part`); verified
`shape_BatteryIcon rect=(76,909,69,80)` — the icon's laid-out width is 69px,
matching the reference. Phase I.]

### 46. The aspect-tag / content-scaling engine model was re-derived cold

**Observed:** the whole AspectRatioToTag → "Content Canvas Scaling" mechanism, the
per-screen aspect sources (display-entity `aspectRatioOverride` / `screenPreset` /
auto-from-mesh), the Clipper screen→entity loadout mapping, and the mesh aspects
were researched from scratch this arc — none of it was in the docs.

**Improvement:** the mechanism + step-3 plan was captured in a hand-off that
bootstrapped the next session; step 3 LANDED 2026-06-14 (`cc67d79e2`) — cockpit
screens render at their true screen-mesh aspect — and the hand-off doc was
deleted as complete. The reference dossier §3 now pairs every screen to its
capture and records the power row as FROZEN GOLD.

**Action:** [done 2026-06-14 — hand-off (commit `1d7eaffdb`) bootstrapped step 3,
which landed `cc67d79e2`; hand-off doc removed on completion — Phase I.]

### Phase I — implementation (2026-06-14, this retro)

Render-neutral tooling + docs only (no freeze/baseline touched; the power-screen
fixes themselves went through the arc's TDD/freeze flow above, not this retro).
`ui_check.sh` green after the edits.
1. `mfd_ir_dump` startup banner + compile timing (item 42). [done]
2. Docs: workflow §10 don't-retry entries (items 42 slow-harness, 43 ui_ir_query
   blind spot, 45 measure-via-probe); reference freeze sequence (item 44) +
   dossier power row → gold + step-3 hand-off pointer (items 44, 46). [done]
3. MCP fetcher library index (item 43) — implemented + redeployed 2026-06-14;
   verified `ui_ir_query` now returns the power cards at 438px (was 399px). [done]
4. `ui_freeze_cycle` manifest-delta pre-check (item 44) + durable
   `BB_DRAW_RECT_PROBE` (item 45) — implemented + verified 2026-06-14 (both were
   initially deferred). [done]

All Phase-I items landed; no `[planned]` remainder.

## Arc — step-3 per-screen aspect implementation (2026-06-14)

The step-3 hand-off (above) was executed: cockpit screens now render at their
true screen-mesh aspect (`cc67d79e2`; door + annunciator L/R re-frozen
`404e4fa23`/freeze; hand-off doc deleted `36cd8a8b0`). Findings from the work.

### 47. The default export LOD silently hid the fix → `--lod 0` + probe + docs
**Observed:** ~6 release-rebuild+export cycles (~70s each) were burned believing
the square HUD screens' geometry was "unreachable" — `loaded.mesh` had no
`RTT_Screen` submesh for them. Real cause: plain `entity export` defaults to
**LOD1**, which CULLS the small HUD screens (g-force, velocity ball, …); their
aspect resolved to `None` and they rendered 16:9. Compounded by inspecting a
STALE `LOD0_TEX0` scene.json while the export wrote `LOD1_TEX2`, and by the
shared `Generated/*.png` being per-canvas + stale. `--lod 0` fixed it instantly.
**Improvement:** durable `SB_SCREEN_ASPECT_PROBE` (flags empty-mesh/LOD), a
reference §5 *screen-mesh → render aspect* note (LOD0 requirement + staleness
trap + freeze-uses-`--lod 0`), and a workflow §10 don't-retry bullet.
**Action:** [done 2026-06-14 — this retro: probe + docs.]

### 48. The hand-off's root cause was wrong (DataCore research ≠ export bindings)
**Observed:** the step-3 hand-off (from record research) asserted the square
screens were `mfd`/4:3 sharing `M_MFD_Screen`, reusable via `aspect_tag.rs`. The
exported `scene.json` showed them `physical` on `M_Physical_Screen` (16:9) with
NO mfd path and radar a hardcoded 1024². Overturning this (a spike) cost real
time before any code was written.
**Improvement:** workflow §9 — a handoff making binding-kind/canvas/aspect
claims MUST verify them against the exported `scene.json` `ui_bindings` first
(the export is ground truth, not the records).
**Action:** [done 2026-06-14 — this retro: workflow §9.]

### 49. Screen geometry → aspect data model re-derived cold; AABB snippet a trap
**Observed:** had to discover that submesh `material_name` is EMPTY in the
export (identify the RTT material by `material_id` → `MtlFile` name), that the
screen quad maps via `node_parent_index` → `nmc` node name == helper, and that
the aspect must be **PCA** in-plane — the hand-off's `plane_aspect` snippet used
an AABB, which collapsed the TILTED annunciator to 1.96 instead of 5.58.
**Improvement:** reference §5 captures the data model + a PCA Blender snippet
(replacing the AABB one); the mechanism lives in `screen_aspect.rs` (PCA + 5
unit tests).
**Action:** [done 2026-06-14 — code `cc67d79e2`; docs this retro.]

### 50. Diagnosing a `None` aspect needed ad-hoc probes rebuilt 3× → durable probe
**Observed:** temporary `eprintln` probes (mesh verts, RTT-submesh count, node
match, result) were added/removed across several slow release cycles to find
why the aspect was `None` (LOD culling + empty `material_name`).
**Improvement:** `SB_SCREEN_ASPECT_PROBE=1` is now permanent in `child_payload`
(`SCREEN_ASPECT helper=… kind=… mesh_verts=… aspect=…`), registered in
reference §6 with the None-diagnosis decode (verts=0 ⇒ LOD; verts>0 ⇒ no RTT
submesh on node).
**Action:** [done 2026-06-14 — this retro.]

### 51. `ui_render.sh` routed the HUD gauges to LOD1 (culled) — acceptance bug
**Observed:** the retro acceptance dry-run (render a dossier screen from the docs
alone) caught it: the wrapper's auto-LOD keyed only on `*_RTT`, so the cockpit
HUD gauges (`Screen_Small_Radar*`, `Countermeasures_Screen`,
`Screen_Central_Compass`, `screen_flight_hud*`, `Screen_Annunciator_*`) fell to
LOD1 and rendered culled/16:9 — a fresh agent would inspect a blank screen.
**Improvement:** extend the wrapper's LOD0 case to the cockpit dashboard
families; fix the §2/§3 LOD-derivation text (the annunciator is LOD0, not LOD1).
**Action:** [done 2026-06-14 — this retro; verified `Screen_Small_Radar2` →
LOD0 → 1920×1920.]

### Phase J — implementation (2026-06-14, this retro)

Render-neutral tooling + docs only (the screen-aspect fix + freezes went through
the arc's TDD/freeze flow above — `cc67d79e2` / `404e4fa23` / `36cd8a8b0` — not
this retro). `ui_check.sh` green after the edits; `SB_SCREEN_ASPECT_PROBE`
verified on a `--lod 0` export.
1. `SB_SCREEN_ASPECT_PROBE` in `child_payload` + reference §6 registry row
   (items 47, 50). [done]
2. Docs: reference §5 *screen-mesh → render aspect* (LOD0 / material_id / PCA /
   freeze-LOD), workflow §9 (verify handoff vs scene.json) + §10 LOD
   don't-retry + handoff-deletion expectation (items 47–49). [done]
3. `ui_render.sh` cockpit-LOD0 routing + §2/§3 text (item 51 — acceptance fix).
   [done]

All Phase-J items landed; no `[planned]` remainder.

## Arc — g-force / velocity ball parity to platinum (2026-06-15)

### 52. `cover_fit_recentre` drift is INVISIBLE to the TDD-tier guards — only `--full` catches it
**Observed:** relaxing the `cover_fit_recentre` clamp (to centre the g-force ball,
which the origin-clamp left ~63px left) silently shifted the aspectOverrides
door + annunciator too — they get `cover_fit=true` from the screen-mesh override,
and with `canvas==target` the OLD clamp collapsed to a no-op while the relaxed one
centred their largest content node (whole-image guard: door 56.9%, annunc 43-57%).
The fast `ui_check` (IR snapshot + live-IR guards) PASSED — the snapshot freeze
pipeline does not apply `cover_fit`, so any post-layout rect shift in
`cover_fit_recentre` is invisible to it. Cost a full release+export+`--full` cycle
to discover, and a second to confirm the fix.
**Improvement:** (a) scoped `cover_fit_recentre` to shift ONLY on an axis where the
cover-scaled canvas OVERFLOWS the target, so aspectOverrides screens (canvas==target)
are inherently a no-op (code, `e27368f41`); (b) workflow §10 don't-retry bullet:
anything touching `cover_fit_recentre` / `bb_layout` post-layout rect shifts needs a
`--full` re-export+visual guard — the TDD tier will not see it.
**Action:** code fix landed; doc bullet [done — this retro].

### 53. Stale LOCAL visual artifacts masquerade as a regression
**Observed:** the `--full` guard flagged `clipper_power_master` (1.38%) and
`clipper_target_master` (<0.5%) — neither touched by this arc. Burned time proving
it: their IR snapshots were unchanged, neither has a full-circle node (cubic) nor a
cover-fit path, and `BB_SHRINK_PROBE` showed the shrinkProportion fix doesn't fire
on them. The diff was a ~2px emissions-header text-top shift baked into the
*untracked* `test-artifacts/ui/*.png`, which only refresh on freeze, so they had
drifted from the current render (and from the already-approved IR snapshot, which
doesn't capture those text nodes).
**Improvement:** workflow §10 note — a visual-guard failure on a screen your change
provably can't reach (IR snapshot unchanged + no relevant code path, confirmed with
the disable→adjudicate discriminators + `BB_SHRINK_PROBE`) is a STALE LOCAL
artifact, not a regression; re-freeze to sync (owner-gated), don't hunt a phantom
root cause.
**Action:** doc note [done — this retro].

### 54. Circular-gauge geometry re-measured with one-off numpy >2x → `ui_gauge_measure.py`
**Observed:** the arc hand-wrote ≥5 throwaway numpy snippets — centre-dot position +
circularity (circle vs squircle), cardinal-ring 2D centroids + on-axis perpendicular
offsets, cross-arm V/H symmetry, and a centre-aligned render|reference montage — all
generic across the circular HUD gauges.
**Improvement:** `scripts/ui_gauge_measure.py <render> [reference] [--montage out]`
emits all of these as JSON (white-dot offset+circularity, cross V/H, per-cardinal
perp offset + radius fraction) plus the centre-aligned montage. Caveat in its
docstring: the per-cardinal window can catch an adjacent diagonal marker (the cross
V/H band metric is the robust one).
**Action:** tool added [done — this retro]; reference §7 row [done].

### 55. Full-circle corner radius rendered a SQUIRCLE (quadratic corner arcs)
**Observed:** `rounded_rect_path` used quadratic Bezier corner arcs, which bulge
toward the corner; a full-radius rect (the centre dot, corner_radius 100 clamped to
half of a 114.86² node) therefore filled ~8% more than a circle — a visible rounded
square the owner flagged. Only a circularity-ratio measurement (not eyeballing)
separated it cleanly.
**Improvement:** cubic corner arcs (k≈0.5523) for FULL ellipses only (radius consumes
both half-extents), gated so partial card/border corners keep the quadratic path and
stay byte-identical to the frozen MFD baselines (`69bb4ffb0`, scoped `e27368f41`).
**Action:** code fix landed; recorded here.

### Phase K — implementation (2026-06-15, this retro)
1. `scripts/ui_gauge_measure.py` (item 54) — added + verify-on-write run; reference §7
   diagnostics row. [done]
2. Docs: workflow §10 `cover_fit_recentre` `--full` bullet (item 52) + workflow §10
   stale-local-artifact note (item 53, both in §10). [done]
3. Code fixes (items 52/54/55) landed during the arc (commits cited above); this phase
   is tooling + docs only and does not alter render behaviour.

## Arc — velocity-num HUD parity (2026-06-15)

### 56. `ui_check.sh --full` does NOT re-export → STALE-EXPORT failure reads as breakage
**Observed:** after a code change I ran `ui_check.sh --full` to measure the blast
radius on the frozen baselines. It failed with `STALE EXPORT: Generated PNGs
predate the current build` — the export-coupled visual guard compares
`ships/Data/UI/Generated/*.png`, which `--full` does NOT refresh, so it fails the
P0.2 staleness guard until you manually re-export. Reference §1 describes `--full`
as running "those export-coupled visual guards **authoritatively**", which reads as
"it exports first" — it does not. Cost a full cycle + a grep through
`generate_ui_regression_artifacts.sh` to recover the canonical export invocation
(`entity export drak_clipper <ws>/ships --kind decomposed --lod 0 --mip 0
--materials all` — note `--lod 0`, or the cockpit HUD screens are culled).
**Improvement:** reference §1 — state that `--full` requires a FRESH export first
and give the exact command inline; reference §2 — promote the canonical guard-export
command (with `--lod 0 --mip 0 --materials all`) as "the export the visual guard
compares against", not buried in the freeze script.
**Action:** doc fix [done — this retro].

### 57. `FONTDUMP` column doc was stale (missing `width_px`) → misread a font's em as a position
**Observed:** chasing the blank render I read a `FONTDUMP` line `… 52.00 3.91
21560.0 18.21 -` and briefly read `21560.0` as an off-screen draw coordinate. The
emitter (`swf_draw.rs`) actually prints `size_px visible_px units_per_em width_px
text` (21560 = the Slug font's em units), but `ui-font-size-harness.md` documented
the columns as `… size_px visible_px em text` — missing `width_px`. Reading the
emitter settled it, but the doc drift cost a wrong first hypothesis.
**Improvement:** `ui-font-size-harness.md` — fix the FONTDUMP column list to
`FONTDUMP \t canvas \t node \t font \t size_px \t visible_px \t units_per_em \t
width_px \t text` (matches the `eprintln!` in `swf_draw.rs`).
**Action:** doc fix [done — this retro].

### 58. Text-screen position/size diagnosis re-typed >3× → `ui_measure.py --text-bands`
**Observed:** diagnosing a text-only screen (is it blank? where do the glyphs land?
what size vs the reference?) I hand-wrote the same numpy three times: the bright-
pixel bbox + centre-x of the rendered text, and the per-line cap-height BANDS as a
PERCENT of screen height for render vs reference (the measurement that proved the
velocity-num font is ~7× under the reference — 1.9% vs 20.6%/16.2%). Generic across
every text/readout screen, none of it covered by `ui_gauge_measure.py` (circular
gauges) or the per-box `ui_measure.py` crop mode.
**Improvement:** `ui_measure.py --text-bands <image> [--ref <reference>]` — emits the
bright-text bbox + centre-x and the per-line band heights as % of image height (and
a render-vs-ref ratio when `--ref` is given). Reuse for any text-screen parity.
**Action:** tool added [done — this retro]; reference §7 row [done].

### 59. Centring an anchored label via the text-draw rule is a CROSS-SCREEN trap (regressed medical)
**Observed:** to centre the velocity-num readouts I broadened `ir_compose`'s
`center_anchored_heading` from `label_style=="Heading1"` to ANY style (the rule
keys on label `anchorToParent` 0.5/0.5 + node anchor/pivot 0,0). It REGRESSED the
medical-bed (`ui_target_a`) titles/descriptions by 1.5% — they share the identical
anchor pattern but must render LEFT. The owner caught it live; the `--full` visual
guard had already flagged it. The Heading1 gate is load-bearing: centring is NOT
purely anchor-driven. The correct fix centred the readout via `bb_layout` CARD
cross-axis content-fit + `crossAxisJustification=Center` (scoped to center columns,
which no frozen screen has), leaving the text-draw rule untouched.
**Improvement:** workflow §10 don't-retry bullet — do NOT broaden
`center_anchored_heading` (the anchored-label text-draw centring) to non-Heading1;
the medical title/desc share the anchor pattern and stay left. Centre stacked
readouts via center-column card cross-fit instead.
**Action:** doc bullet [done — this retro]; the trap is also in the dossier
velocity-num row and [[velocity-num-hud-parity]].

### Phase L — implementation (2026-06-15, this retro)
1. `scripts/ui_measure.py --text-bands` (item 58) — added + verify-on-write run;
   reference §7 row. [done]
2. Docs: reference §1 `--full` needs-fresh-export note + §2 canonical guard-export
   command (item 56); `ui-font-size-harness.md` FONTDUMP columns (item 57); workflow
   §10 don't-broaden-center_anchored_heading bullet (item 59). [done]
3. Render-behaviour code fixes (registry pins `1f25845d2`, centred-column content-fit
   `0cdf6d526`) landed during the arc; this phase is tooling + docs only and does not
   alter render behaviour.

### 60. "Undecoded per-screen text-scale blocker" was UNDER-RESEARCH — the authored FontSize was in the data
**Observed (velocity-num arc, 2026-06-15):** the prior arc deferred the ~9× font gap
as a PROVEN blocker — "no data-derived scale reaches 6×, per-screen canvas→screen
text-scale model undecoded, and the frozen annunciator counterexample rules out a
global scale." The whole framing was wrong: it was NEVER a scale problem. The DRAK
velocity SCREEN variant (`drak_hc_hud_cutlass_velocity_num`, the `(Screen)` variant
the Clipper instantiates) authors FontSize **500/420** directly; the render fell back
to the Heading2 standard (52) because those authored sizes weren't being APPLIED (two
upstream gaps: the no-brand-match `defaultStyles` fallback, and the text-format route
rejecting `Type(Text)+Parent[…]`). `SB_UI_FONT_DUMP` showed 52 (effective), and the
authored 500/420 was one grep away in the instantiated variant record. The "blocker"
was a paper estimate that never checked the authored value reached the node.
**Improvement:** workflow §10 don't-retry bullet — font reads wrong → read the
INSTANTIATED variant's authored FontSize FIRST (find it via the rendered node names;
`SB_UI_FONT_DUMP` = effective, `BB_A3_STYLE_PROBE`/`BB_TEXT_FORMAT_PROBE` = which
entries matched). A "no derivable scale" conclusion is valid only after confirming the
authored size is reaching the node. Generalises the skill's "undecoded = under-research,
not a proven blocker" rule to the font path.
**Action:** workflow §10 bullet [done — this retro]; dossier velocity-num row +
[[velocity-num-hud-parity]] updated to overturn the wrong diagnosis. Code fixes
`6c1343abf`/`c7931fb4b` landed during the arc.

### 61. `check_ui_hardcoding.sh` silently stopped guarding — targets renamed to directories
**Observed:** the guard greps `ir_compose.rs` / `compose.rs` / `ui_ir.rs`, all
decomposed into `<dir>/` long ago. `rg` printed "No such file or directory" to stderr
and returned non-zero (= "marker not found"), so every check on those files PASSED
VACUOUSLY — a re-introduced hard-code in the decomposed dirs would sail through. A
guard that can't find its target is worse than no guard (false assurance).
**Improvement:** resolve `<file>.rs` → its `<dir>/` and search recursively; a target
that resolves to NEITHER a file nor a dir is now a LOUD failure (silent-failure →
loud), so the next rename can't quietly disable a check.
**Action:** `scripts/check_ui_hardcoding.sh` fixed + re-run clean (no stderr IO
errors) [done — this retro].

### 62. `ui_freeze_cycle.sh` exported without `--lod 0` — would cull cockpit HUD screens on re-freeze
**Observed:** onboarding the (cockpit) velocity-num target, `ui_freeze_cycle.sh`'s
export line lacked `--lod 0`, while `generate_ui_regression_artifacts.sh` and the
canonical guard-export both use it. LOD1 CULLS the small HUD screens (ledger 47), so a
re-freeze of any cockpit target run through `ui_freeze_cycle` (without `--skip-export`)
would have frozen a stale/missing PNG. The §7 onboard flow happens to use the `generate`
script, so this was a latent trap, not hit this arc — but a future cockpit re-freeze via
`ui_freeze_cycle` would silently regress.
**Improvement:** `ui_freeze_cycle.sh` export now matches the canonical guard-export
(`--lod 0 --mip 0 --materials all`) with an inline comment citing the LOD-cull reason.
**Action:** `scripts/ui_freeze_cycle.sh` fixed [done — this retro].

### Phase M — implementation (2026-06-15, velocity-num arc retro)
Tooling + docs only; no render-behaviour change (the arc's render fixes `6c1343abf`/
`c7931fb4b` and the gold onboarding `a91cec377` landed in the loop). `ui_check.sh`
green per commit.
1. `scripts/check_ui_hardcoding.sh` — decomposed-dir resolution + loud-on-missing
   (item 61). [done]
2. `scripts/ui_freeze_cycle.sh` — `--lod 0` export (item 62). [done]
3. Docs: workflow §10 read-the-authored-FontSize-first bullet (item 60); dossier
   velocity-num row updated (font/colour/boxes landed, GOLD onboarded, background =
   capture characteristic). [done]

### 63. "White where nothing should draw" is an OVER-PAINTER, not a missing asset — dump every node's fill first
**Observed (compass arc, 2026-06-15):** the compass rendered a white sheet over the
top ~75%. The first instinct ("the dark `DRAK_Background_compass` texture failed to
load / is mis-fit") was wrong — the draw-rect probe showed it rasterising at the full
`(0,0,1920,587)` with the correct `.dds`. The white was a DIFFERENT node painting OVER
it: `CanvasProxyRoot`, an authored `rendererType:"None"` DisplayWidget with
`background.enable=true` white, covering exactly the canvas-visible region. Since the
texture max value is ~50, white (255) provably could not come from it — the over-painter
was findable in one query. I lost time theorising about texture fit before dumping fills.
**Engine fact:** a node with `rendererType:"None"` is a non-rendering proxy/group — the
engine paints nothing for it; only `"Flash"` nodes draw their `background`/svg. Fixed in
`node_background_enabled` (`a47759308`).
**Improvement:** (a) workflow §10 don't-retry bullet — white/wrong-colour where a
background should be is almost always an OVER-PAINTER; dump EVERY node's fill with
`python3 scripts/ui_ir_query.py query <ir.json> '.*' --fields background_fill_colour,stroke_colour`
(verified: it flat-lists all nodes incl. the flat `nodes[]` dump IR) BEFORE theorising
about asset load/fit. A fill that can't come from the suspect texture's value range
proves a later element is painting over it. (b) Record the `rendererType:"None"` =
non-rendering rule in the architecture runbook engine-models section.
**Action:** docs below [done — this retro]; unit test
`compile_ir_does_not_draw_background_on_renderertype_none_container` guards it.

### 64. Top-level (namespace-less) engine-state `arrayVariable` lists were skipped → stray template item at rest
**Observed (compass arc):** `apply_array_variable_lists` resolves a relative
`arrayVariable` only under a list namespace; a namespace-less one returned `None`
(skip), leaving the list's lone authored child — a per-entry CLONE template — rendering
as a stray item. The compass `list_Ticks` (`FlightController/Compass/Ticks`) hit this:
a stray orange tick at the strip edge. Many HUD lists are namespace-less engine-state
paths (`WeaponController/Countermeasures/Launchers`, `FlightController/ScmTicks`, …).
**Improvement:** a namespace-less arrayVariable that is a MULTI-segment path (`A/B/C`)
is an absolute engine-state reference (the authored leading slash is inconsistent across
the data) — resolve it directly; a BARE single-segment name (the power outer `pipList`)
is UI-local and still needs a namespace (the structural discriminator that kept the
power pip stacks byte-identical, confirmed by `--full`). The registry then pins the
at-rest count (compass `=0`, provenance-noted) so the template deactivates: the faithful
empty list. Recorded in workflow §10 + reference glossary so the next HUD-list arc
doesn't re-derive it.
**Action:** `apply_array_variable_lists` (`d83bdcf27`) + registry pin + docs [done].

### 65. `ui_check.sh` / `entity export` redirected output is FULLY BUFFERED — silence ≠ hang
**Observed (compass arc):** running `ui_check.sh --full` and the export in the
background, the output file stayed 0 bytes until the very end, so progress-polling
(`tail`/`grep` the file) showed nothing for minutes — easy to misread as a stalled/hung
run and kill it. The whole battery flushes only at process exit when stdout is a file.
**Improvement:** reference §1 note — when `ui_check.sh`/`--full`/`export` output is
redirected to a file, it is fully buffered (no incremental lines); wait on process
EXIT (the background-task completion notification or an `until` exit-status loop), do not
read a 0-byte file as a hang. (Same lesson as ledger 42 "slowness ≠ a loop", applied to
buffered output.)
**Action:** reference §1 buffered-output note [done — this retro].

### 66. "Compass live ticks = proven blocker" was UNDER-RESEARCH — the config was one DataCore search away
**Observed (compass arc, 2026-06-15):** I declared the compass tick/label data a
PROVEN blocker — "the projection config (FOV/spacing/intervals) is engine C++ only;
searched canvas JSON, DataCore records, P4K config; reproducing it needs measuring
off the reference = banned hard-coding." The owner pushed "do more research"; a
deeper DataCore search found `SVehicleHudParams.VehicleHudDefault.compassTape`
(`hudparams/vehiclehuddefault.xml`): `range=90`° / `mainTickIncrement=20`° /
`subTicks=4` — the EXACT projection model, matching the reference (majors 20° apart,
minors 5°, FOV≈90°). My "search" had only tried `search_records("compass")` (→ UI
canvases) and `("ifcs")` (→ flight-control); I never searched the HUD-params family
(`search_records("vehiclehud")` → the record immediately). The blocker was a
not-exhaustive search dressed as a proof. Resolved by DERIVING the at-rest tick
array from `compassTape` at heading 0 (`ship_values.rs::derive_compass_ticks`) —
fully data-driven, no hard-coding.
**Improvement:** this is the THIRD arc where "not in the data / undecoded = blocked"
was actually under-research (cf. ledger 60 velocity-num font, and the m/s SIUnit
loc-family). Reinforced the *Default to fixing* bar in the skill: a "demonstrated
absent" proof must name the record FAMILIES searched (not just keyword "X"); for a
config value, search the params/global record types (`*Params`, `*Global`,
`hudparams/…`), not only the feature name. Before declaring an engine value
underivable, run `search_records` across the plausible STRUCT families
(`SVehicleHudParams`-style), not one keyword.
**Action:** skill *Default to fixing* / red-flag reinforcement (recommendations.md);
dossier + registry-notes + memory overturned the wrong "blocker" verdict; derivation
landed in the loop (`ship_values.rs`). [done — this retro]

### Phase N — implementation (2026-06-15, compass arc retro)
The arc's render fixes landed in the loop: `a47759308` (rendererType:None
background), `d83bdcf27` (empty-compass arrayVariable resolution), and the
compass-tick DERIVATION in `ship_values.rs` (item 66, overturning the wrong
blocker). Retro docs:
1. Reference §6/§7: over-painter probe one-liner (item 63). [done]
2. Architecture runbook: `rendererType:"None"` non-rendering rule (item 63). [done]
3. Workflow §10: namespace-less multi-segment engine-state list bullet (item 64);
   reference §1 buffered-output note (item 65). [done]
4. Item 66 (under-research blocker) — dossier/registry-notes/memory corrected;
   skill *Default to fixing* search-the-record-families reinforcement. [done]

### 67. Compass follow-up: `auto`≠`useRaw` fill, height-driven font, and a too-broad font gate (medical trap)
**Observed (compass arc round 2, 2026-06-15, owner feedback "font/colour/tick-height wrong, minors missing"):** three findings.
(a) **Canvas fill by `coordinateMethod`.** The compass ticks clipped because `bb_layout` lumped `coordinateMethod:"auto"` with `useRaw` (uniform cover/contain). The compass master UNIQUELY authors `auto` among cockpit screens (others `useRaw`; target `aspectOverridesWidth`); `auto` FILLS the target like `aspectOverrides*` (non-uniform sx/sy). Moved `auto` to that branch (`4f352b429`) — no frozen screen is `auto`. Lesson: check the authored `coordinateMethod` before treating a wide-screen-vs-16:9-canvas mismatch as a cover/contain problem.
(b) **Height-driven text sizes its font to the field.** Labels with no FontSize fell to the Heading1 default (60, ~6% cap vs ref 19%). A text field whose box is height-driven (width `PercentOfY`, height `Percent`) has the engine size the glyph to fill the field; `resolve_effective_font_size` now does this before the named-style default (`ca5ec0b25`).
(c) **A "Percent height → field-fit" gate is TOO BROAD — it hit medical PLATINUM.** The first cut keyed only on a Percent height and regressed `ui_target_a` titles (25/30/60 → field heights); the live-IR guard caught it. The discriminator is the `PercentOfY` WIDTH (a height-DRIVEN box) — medical's titles have Percent heights but normal widths (width-laid-out), so they keep their named size. Lesson: when a font-model gate trips a frozen text screen, the disable→adjudicate delta names the exact baseline; find the structural property that separates the motivating case (here: width derived FROM height) rather than widening the gate.
(d) **Colour = proven blocker.** The labels need the `heading-cardinal-dir` tag for `CardinalLabel→Base`, applied at runtime via `BindingsTagFromBoolean(maintick→tag)`. The generic `hud_generic_headingtapecomponent` has 4 such ops; the rendered DRAK compass variant has 0 (master refs only it). The rendered canvas data has the STYLE but not the tag-application, so faithfully the labels are the Heading1 default (Bright) — reproducing orange needs the absent binding or name-gating. Recorded as a deferral, not hacked.
**Improvement/Action:** all landed in the loop (`4f352b429`, `ca5ec0b25`); dossier + [[compass-hud-parity]] updated. Owner-gated checkpoints honoured (asked before the frozen-shared cover_fit change; user chose "investigate more" → found the cleaner `coordinateMethod` discriminator). [done — this round]
