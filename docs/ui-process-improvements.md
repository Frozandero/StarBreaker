# UI parity process — improvement plan

A retrospective of the Clipper power-screen parity arc (2026-06-10 → 06-11),
turned into concrete process changes. Each item states the observed problem
(with the incident that exposed it), the improvement, and the action that
implements it. Items are ordered by expected payoff.

## 1. Make the render→compare→catalog review a standard phase

**Observed:** fixes were driven by a hand-written symptom list; a full
region-by-region comparison only happened when explicitly requested (Step 9R)
— and immediately found defects the symptom list missed (emissions header
collapsed to 2 px, OUTPUT title mis-flowed) plus one latent engine-model bug
(`SizeY` style modifiers converting Percent→Fixed) whose fix also resolved the
long-open A7 backdrop item.

**Improvement:** every workstream ends with a mandatory review phase:
re-render → region compare → diff catalog (severity + root-cause hypothesis +
fix/defer decision) → ordered fix plan → fixes. Defects found by review get
catalog numbers so "deferred" is an explicit, recorded decision, not a gap.

**Action:** add the review phase to `docs/ui-matching-workflow.md` as a named
step; catalog tables live in the working memory/plan file for the arc.

## 2. Build a reusable region-compare tool

**Observed:** every comparison was a hand-written PIL crop snippet; mismatched
reference resolutions (1959×1513 vs 1600×1200) and bad crop boxes wasted
several iterations.

**Improvement:** one script with named region presets per screen and
auto-scaling that emits a labelled contact sheet.

**Action:** `scripts/ui_compare.py <render.png> <reference.png>
[--regions <screen-preset>]` — scales the reference to the render width,
crops named regions (presets checked in per screen, e.g.
`power: emissions, columns, scrollbar, output_card, battery_card, footer`),
writes `cmp_<region>.png` side-by-sides. Review phases use it exclusively.

## 3. Self-checking harnesses — a zero-match run is a harness FAILURE

**Observed:** `font_size_check.py` silently reported every baseline element
MISSING because the dump format had gained a column after the baseline was
captured. Separately, `font_size_baseline.tsv` had been stale for some time (7
real drifts from earlier approved changes) because the harness wasn't run when
those changes landed.

**Improvement:** harness checkers must fail loudly when they match *nothing*
(that is parser/format drift, not data drift), and harness runs belong to the
per-change checklist, not to memory.

**Action:**
- `font_size_check.py`: exit with a distinct "harness error" when matched
  elements == 0 or the dump column count is unexpected.
- Re-capture `font_size_baseline.tsv` (needs approval — 7 known drifts are
  from approved work: caps-reduction removal, annunciator/door changes).
- Add the harness (and the guards, item 5) to a single check script (item 4)
  so "run it whenever text size changes" stops relying on discipline.

## 4. One command for the standard check battery

**Observed:** the loop "ui lib tests → manifest_live_ir_guard →
line_count_guard → (font harness)" was retyped dozens of times, and at least
once a file went over the 500-line cap unnoticed for several edits because
`line_count_guard` is an integration test that `--lib` runs don't include.

**Improvement:** a single entry point that runs the whole battery, so every
TDD cycle ends with the same command and nothing is forgotten.

**Action:** `scripts/ui_check.sh` = `cargo test -p starbreaker-ui --lib` +
`--test manifest_live_ir_guard --test line_count_guard` + (optional flag)
the font harness against the current export. Reference it from
`docs/ui-regression-policy.md`.

## 5. Self-auditing freezes

**Observed:** re-freezing the gold target required a hand-written JSON diff to
prove only the intended element changed. That audit is exactly what makes a
re-freeze trustworthy, and it currently depends on the operator remembering to
do it.

**Improvement:** the freeze script prints (and records) a structured delta
report — per target, each changed identity with old→new field values — and
refuses to write when the `--reason` doesn't account for every changed target.

**Action:** extend `freeze_ui_snapshot_ir.rs` to diff against the existing
freeze file and emit the delta; store the delta summary alongside
`approver`/`reason` in the freeze JSON.

## 6. A pipeline-stage bisection tool for layout/geometry bugs

**Observed:** the single most effective diagnostic this arc was a throwaway
example that parsed a canvas standalone and laid it out, proving the emissions
collapse came from the resolve cascade, not parse or layout (sizing was
`Percent(1.0)` standalone, `Fixed(1.0)` after resolve). The scratch example
(`examples/repro_emissions.rs`) was hand-written under time pressure.

**Improvement:** keep that capability permanently: run any canvas through
(a) parse-only and (b) full resolve, lay both out at a given size, and print
the first node where typed sizing / rects diverge.

**Action:** generalise the scratch example into
`examples/ui_stage_diff.rs -- <canvas.json> [WxH] [--records-root <dir>]`;
delete `repro_emissions.rs`.

## 7. A probe registry instead of folklore

**Observed:** env-gated probes (`BB_A3_STYLE_PROBE`, `BB_SHRINK_PROBE`,
`SB_UI_GEOM_PROBE`, `SB_SHIP_VALUES_DUMP`, `SB_UI_FONT_DUMP`,
`BB_A3_TEXT_PROBE`) are discovered by grepping, and a memory note referenced a
probe (`MFD_IR_DUMP_LOG`) that does not exist.

**Improvement:** one documented list; new probes are added to it in the same
commit that introduces them.

**Action:** `docs/ui-probes.md` — name, where it prints, what it shows, one
example invocation. Add a line to the AGENTS.md UI section pointing at it.

## 8. Codify the guard-adjudication method

**Observed (working well — codify it):** four times this arc a generic rule
tripped a frozen baseline, and the fix was the same method each time: read the
frozen counterexample, find the *structural* property separating it from the
motivating case, scope the rule by that property — never by name:
- flex shrink → only flex-managed children (Fixed/Percent/Auto∈(0,1]);
- Auto-hint textfield intrinsic sizing → only when anchored beyond the parent
  edge (anchor > 1.0 on that axis);
- `urlPostfix` namespace composition → only absolute (leading-slash)
  postfixes;
- inline-style FontSize → marked and ranked explicitly in the resolver.

**Improvement:** write the method down so it's the default move, and record
each scoped rule's counterexample in the code comment (mostly already done).

**Action:** add a "When a guard trips" section to
`docs/ui-matching-workflow.md`: (1) identify the frozen counterexample node,
(2) read its authored source, (3) find the structural discriminator, (4) scope
the rule and cite the counterexample in a comment, (5) if the baseline is the
thing that's wrong, re-freeze with the per-identity delta in the reason
(item 5).

## 9. Memory updates at discovery time, before each commit

**Observed (working well — codify it):** the emissions mechanism decode was
written to the project memory *before* implementation started, which is what
allowed the work to survive context compaction; conversely, earlier diagnoses
written "when context ran low" were rushed.

**Action:** standing rule in the working plan file: a non-trivial diagnosis is
written to the arc's memory file when it is *made*; every commit is preceded
by a memory-file status update for the arc.

## 10. Registry data hygiene for at-rest engine values

**Observed:** values the engine pushes at runtime (e.g. `iscast` per render
target type, signature emissions, OUTPUT 2/16) end up either pinned in
`default_value_registry_v1.json` or derived in `ship_values.rs`, and the
boundary is only documented in commit messages. JSON carries no comments, so
each pinned value's provenance ("reference-verified", "derivation TODO") is
invisible at the data file.

**Action:** a sibling `default_value_registry_v1.notes.md` mapping each pinned
path → provenance + sunset condition (move-to-derivation), kept in the same
commits that touch the registry; the fallback-register doc links to it.
