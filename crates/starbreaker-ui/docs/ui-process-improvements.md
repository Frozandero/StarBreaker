# UI parity process — improvements and consolidation plan

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

### 16. Reserved-ID-band fragility (recorded; fix rides P3)

**Observed:** adding a NEW expanding host type (WidgetSeparator) shifted
the shared `EXPANSION_ID_BASE` allocation order and STOLE a frozen
platinum identity (the medical close-button X became a separator
instance). The guards caught it before landing — the system worked — but
the design couples baseline identity to expansion ORDER.

**Direction:** band lanes per host type (or a second band for new types)
in `merge_child_scene`; concrete plan parked with P3 in
`crates/starbreaker-ui/docs/ui-clipper-parity-handoff.md` item 12.

### 17. ONE brand-context resolver (architecture debt, recorded)

**Observed:** at least four independent brand-container selection
implementations exist (`resolve_brand_style`'s manufacturer-prefix scan,
`collect_standard_text_styles`' selected_style_name family mapping, the
body-background preferred chain, the separator hud↔env sibling swap) —
and the P3 AEGS-divider leak came exactly from one of them improvising.
Every new modularkit standard re-derives this logic.

**Direction:** extract a single brand-context resolver (canvas style-link
→ `s_<mfr>_{hud|env}` by canvas family → sibling swap; identity matching
only, no prefix scans over shared standards) and migrate call sites one
at a time under the guards.

### 18. Linear-light compositing (gated workstream, evidence strong)

The engine composites in linear light; our renderer blends in sRGB space.
The white-mask glow path now converts (scoped, landed); the renderer-wide
migration would change every alpha blend including text antialiasing —
full re-freeze + re-adjudication of all targets, but the evidence says it
moves EVERYTHING toward the references. Candidate for a dedicated arc;
numbers in `crates/starbreaker-ui/docs/ui-clipper-parity-handoff.md` items 10/11.

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
