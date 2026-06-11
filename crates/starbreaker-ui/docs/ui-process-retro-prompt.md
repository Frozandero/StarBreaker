# UI process retrospective — agent prompt

The companion to `ui-matching-agent-prompt.md`: that one runs a parity arc;
this one runs the **retrospective that improves the process afterwards** —
the exercise that produced `docs/ui-process-improvements.md` and the
consolidated docs/tooling. Run it at arc boundaries (or whenever friction
accumulated). The ledger of findings and their implementation state is
`docs/ui-process-improvements.md` — retros APPEND to it, they don't start
new documents.

## Template

```text
You are working in /home/tom/projects/scorg_tools/StarBreaker (branch
feature/ui).

Read, in order:
1. docs/ui-workflow.md and docs/ui-reference.md  (the current process)
2. docs/ui-process-improvements.md               (the ledger: prior findings,
   their format, and the phased-plan style; you will APPEND to this)
3. ARC's handoff doc and the arc's session memory file
4. git log for ARC's commit range (read the messages — they record what
   was fixed, re-frozen, scoped, and reverted)

ARC=<the work to retrospect, e.g. docs/ui-clipper-parity-handoff.md +
  commit range abc1234..def5678>
FOCUS=<optional: a specific pain to dig into, else sweep everything>

Goal: evidence-based process improvements, implemented — not a essay. Each
finding uses the ledger's format: **Observed** (the concrete incident from
THIS arc — name the commit/file/failure), **Improvement**, **Action**
(file paths, commands, acceptance check).

Sweep these categories against the arc's evidence:
1. Repeated manual work → tooling. Anything typed more than twice
   (ad-hoc crops, hand-rolled diffs, probe greps, command batteries) becomes
   or extends a script/example. Check scripts/ and the examples/ diagnostics
   first — extend before creating.
2. Silent failure modes → loud ones. Any harness/checker/guard that
   reported wrong-but-plausible results (zero matches as data drift, format
   rot as MISSING) gets a distinct hard failure.
3. Documentation drift. Every doc claim touched by the arc gets the
   verify-on-write treatment: run the commands, grep for references to
   anything renamed/deleted (repo-wide, same commit), fix or delete stale
   docs — never leave parallel half-truths. The docs_reference_guard test
   must cover any new doc that cites files.
4. Bootstrap cost. List everything this arc had to RE-DERIVE that the docs
   should have carried (data locations, screen mappings, engine-model rules,
   don't-retry traps, probe names). Each item lands in docs/ui-reference.md
   (dossier rows, probe registry, glossary) or docs/ui-workflow.md §10
   (pain points / don't-retry) so the next session starts warm.
5. Guard/freeze friction. Adjudications that took detours, deltas that
   needed manual auditing, outliers that should have been registered earlier
   — improve the flow or the docs that teach it.
6. Memory/handoff quality. Was the handoff sufficient to resume cold? What
   was missing when context compacted? Fix the handoff TEMPLATE expectations
   in docs/ui-workflow.md §9, not just this arc's file.

Then:
- APPEND the findings as new numbered items to
  docs/ui-process-improvements.md and EXTEND its phased plan (same style:
  per-step files, commands, acceptance; approval-gated items marked).
- IMPLEMENT the plan: quick tooling wins first, then docs, then automation;
  one commit per coherent item citing its ledger number; verify-on-write for
  every doc change; bash scripts/ui_check.sh green per commit (process
  changes must not alter render behaviour — anything that would goes through
  the normal TDD/guard flow of docs/ui-workflow.md instead).
- Baseline-affecting actions (TSV/freeze re-captures) are APPROVAL-GATED:
  present the deltas and stop unless approval was given up front.
- Mark executed steps [done <date> <commit>] in the ledger; update the
  session memory pointers.

Acceptance (the bootstrap test): after implementing, dry-run the per-screen
prompt (ui-matching-agent-prompt.md) from the docs alone — every command,
path, and mapping a fresh agent needs must resolve without leaving
docs/ui-workflow.md + docs/ui-reference.md + the dossier. Any excursion you
needed during THIS retro is a doc bug: fix it before closing.
```

## Example (the 2026-06-11 retrospective)

`ARC=docs/ui-clipper-parity-handoff.md` (the power-screen arc). Findings →
ledger items 1–12 (review phase, ui_compare, harness self-checks, ui_check
battery, self-auditing freezes, ui_stage_diff, probe registry, guard
adjudication method, memory discipline, registry provenance, doc
consolidation, verify-on-write); implemented as the ledger's Phases 0–4
(commits `2c6029f49` … `89d6a4d51`), including superseding the old
ui-matching docs and the per-screen dossier that now bootstraps fresh
sessions.
