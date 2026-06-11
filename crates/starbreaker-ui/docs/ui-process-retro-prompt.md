# UI process retrospective — end-of-arc prompt

The companion to `ui-matching-agent-prompt.md`: that one starts a parity
arc fresh; this one is pasted **at the end of a working session, to the
agent that did the work** — the retrospective runs on lived context (the
friction, dead ends, and retyped commands only that session knows), so it
is NOT handed off to a fresh agent. Generic: no variables to fill in. The
ledger of findings and their implementation state is
`docs/ui-process-improvements.md` — retros APPEND to it, they don't start
new documents.

## Prompt (paste as-is)

```text
Consider the work you have done in this session/arc: what worked, what
didn't, and how the process can be improved — updating tools, creating new
ones, and updating documentation with enough information to bootstrap the
process so the next session needs less research at the start.

First read docs/ui-process-improvements.md (the ledger — you will APPEND
to it, matching its Observed/Improvement/Action format and its phased-plan
style) and skim docs/ui-workflow.md + docs/ui-reference.md so proposals
amend the current process rather than reinvent it. Use this session's own
experience as the primary evidence; use the arc's git log, handoff, and
memory file to recall anything context compaction has blurred.

Sweep these categories against what actually happened:
1. Repeated manual work -> tooling. Anything you typed more than twice
   (ad-hoc crops, hand-rolled diffs, probe greps, command batteries)
   becomes or extends a script/example. Check scripts/ and the examples/
   diagnostics first — extend before creating.
2. Silent failure modes -> loud ones. Any harness/checker/guard that gave
   you a wrong-but-plausible answer (zero matches reported as data drift,
   format rot as MISSING) gets a distinct hard failure.
3. Documentation drift. Every doc claim you relied on that was wrong or
   stale gets fixed with verify-on-write (run the commands; repo-wide grep
   for references to anything renamed/deleted, in the same commit). Never
   leave parallel half-truths; the docs_reference_guard test must cover any
   new doc that cites files.
4. Bootstrap cost. List everything you had to RE-DERIVE this session that
   the docs should have carried (data locations, screen mappings,
   engine-model rules, don't-retry traps, probe names). Land each in
   docs/ui-reference.md (dossier rows, probe registry, glossary) or
   docs/ui-workflow.md §10 (pain points / don't-retry) so the next session
   starts warm.
5. Guard/freeze friction. Adjudications that took detours, deltas you
   audited by hand, outliers that should have been registered earlier —
   improve the flow or the docs that teach it.
6. Memory/handoff quality. Would the handoff you wrote (or inherited) have
   been enough to resume cold? What was missing after compaction? Fix the
   handoff expectations in docs/ui-workflow.md §9, not just this arc's file.

Then:
- APPEND the findings as new numbered items to
  docs/ui-process-improvements.md and EXTEND its phased plan (same style:
  per-step files, commands, acceptance; approval-gated items marked).
- IMPLEMENT the plan: quick tooling wins first, then docs, then automation;
  one commit per coherent item citing its ledger number; verify-on-write
  for every doc change; bash scripts/ui_check.sh green per commit. Process
  changes must not alter render behaviour — anything that would goes
  through the normal TDD/guard flow of docs/ui-workflow.md instead.
- Baseline-affecting actions (TSV/freeze re-captures) are APPROVAL-GATED:
  present the deltas and stop unless approval was given up front.
- Mark executed steps [done <date> <commit>] in the ledger; update the
  session memory pointers.

Acceptance (the bootstrap test): after implementing, dry-run the
per-screen prompt (ui-matching-agent-prompt.md) from the docs alone —
every command, path, and mapping a fresh agent needs must resolve without
leaving docs/ui-workflow.md + docs/ui-reference.md + the dossier. Any
excursion YOU needed during this retro is a doc bug: fix it before
closing.
```

## Example (the 2026-06-11 retrospective)

Run at the end of the power-screen arc, in-session. Findings → ledger items
1–12 (review phase, ui_compare, harness self-checks, ui_check battery,
self-auditing freezes, ui_stage_diff, probe registry, guard adjudication
method, memory discipline, registry provenance, doc consolidation,
verify-on-write); implemented as the ledger's Phases 0–4 (commits
`2c6029f49` … `89d6a4d51`), including superseding the old ui-matching docs
and the per-screen dossier that now bootstraps fresh sessions.
