# ui-screen-parity — review & recommendations

Companion to `SKILL.md`. Per the skill's creation decision (2026-06-14), the
skill is **not** pressure-tested by re-writing it in a RED→GREEN loop; instead,
findings land here in two states, with a lifecycle:

- **Open recommendations** — ideas not yet applied. End-of-arc retrospectives
  (`crates/starbreaker-ui/docs/ui-process-retro-prompt.md`) and reviews append
  here rather than editing `SKILL.md` mid-arc.
- **Change log** — applied changes. When a `writing-skills` session implements
  an open recommendation (or an owner-requested edit), it records the applied
  change in the Change log and clears the open item.

## Validation status

- **De-facto tested through real use.** No formal subagent baseline/green test
  has been run, but the skill has been exercised on real invocations (Drake
  Clipper) and that surfaced 8 issues — all fixed (see Change log). The recurring
  class was "agent shortcuts the discipline" (presume / batch / estimate-defer),
  now consolidated into `SKILL.md`'s **Operating posture** section.
- **Freeze gate held on the first full real arc (2026-06-15, g-force/velocity
  ball to platinum).** The agent stopped at the freeze checkpoint, presented the
  per-identity deltas, and the owner explicitly authorised the re-freezes — the
  one invariant gate worked as designed, not just by assumption. The end-of-arc
  retrospective also fired on BOTH destinations: the skill finding → Open
  recommendations (now applied), and process/tool/doc findings →
  `crates/starbreaker-ui/docs/ui-process-improvements.md` ledger items 52–55 +
  `scripts/ui_gauge_measure.py` (committed `0daf35e8e`). The self-improvement
  loop is validated end-to-end.
- **Recommended later validation** (lower priority now, given real use):
  1. **Compliance dry-run (cheap).** Dispatch a fresh subagent with
     "Drake Clipper / Screen_Right_Upper_RTT" and confirm, from the skill + docs
     alone, it: (a) resolves the reference to
     `reference/in-game/Clipper/Screen_Right_Upper_RTT.png` and CONFIRMS it with
     the user; (b) reads the four required docs in order; (c) finds the dossier
     row (canvas `MC_S_Target_Master`, preset `target`, GOLD
     `clipper_target_master`); (d) respects the freeze/commit/final stop points
     rather than barrelling through; (e) knows to run the retro at the end. Any
     miss is a skill gap — fix it in `SKILL.md` and note it here.
  2. **Full execution test (expensive).** Same scenario, subagent actually runs
     the loop; compare behaviour with vs without the skill.

## Open recommendations (review / decide)

- **Formal compliance dry-run** — still un-run. Decide whether real-use coverage
  supersedes it, or run it once for the paths real use hasn't exercised:
  multi-ship disambiguation, and a screen absent from the dossier.
- **Nothing consumes this file.** Open recommendations accumulate but no step
  reads them back. Decide a trigger — e.g. a `writing-skills` session on this
  skill (or arc start) skims the open items first.
- **Red-flag table length.** Now ~13 rows with near-duplicates; the new
  Operating-posture principle may let several be merged. Optional cleanup,
  deferred to avoid churn while the skill is still settling.
- **Multi-ship generalization unverified** (see Known assumptions) — exercise
  when a second `reference/in-game/` folder exists.
- **`--helper` ≠ reference stem (2026-06-15, velocity-num arc).** The skill's
  worked example and the gather-inputs step say "the chosen SCREEN is
  simultaneously the dossier row, the render `--helper`, and the reference file
  stem." That held for the target/power examples but NOT for velocity-num: the
  reference stem is `ship_velocity_num_master` while the render `--helper` is
  `screen_flight_hud_left_upper` (the dossier's Helper column). I had to read the
  dossier to get the right helper. Recommend softening the wording: the render
  `--helper` comes from the dossier's **Helper/scene column**, which is usually
  but not always the reference file stem — when they differ, the dossier is
  authoritative. (Low risk; wording only.)
- **Partial-parity terminal state is fine, but call it out (2026-06-15).** The
  velocity-num arc landed real structural+content wins yet two diffs (an
  unimplemented engine unit-suffix table; an undecoded HUD content-view zoom) are
  genuine engine-mechanism blockers that BOUND achievable parity this arc — the
  correct terminal state is "deferred with proof," not "clean." The skill already
  permits this (defer-on-proven-blocker), but a one-line acknowledgement that some
  screens have intrinsic mechanism-blockers (so the closing review ends
  deferred-not-clean) would reassure a fully-automated run it isn't failing to
  converge. (Optional.)

## Known assumptions & risks

- **Ship→folder mapping** is only exercised for Drake Clipper → `Clipper`; no
  other reference folder exists yet. The skill auto-discovers folders under
  `reference/in-game/` and confirms the match with the user, which should
  generalize, but multi-folder disambiguation is unverified.
- **Reference-variant selection** (prefer the `corners.json` straight-on
  capture) is heuristic; the mandatory user confirmation is the backstop.
- **Restated rules can drift.** `SKILL.md` repeats a few rule headlines from
  `ui-workflow.md` §1 for strict adherence. If those docs change, re-check the
  skill's restated rules; verify-on-write applies to the command lines it cites.
- **Discoverability.** The skill lives in `StarBreaker/.claude/skills/`; it
  auto-loads as a project skill when Claude runs with StarBreaker as the project
  root. From the `scorg_tools` workspace cwd, invoke it explicitly.
- **Autonomy boundary.** Two launch-selected modes: **semi-automated** gates
  commits and freezes (+ final parity); **fully automated** auto-commits and
  drops the final-parity gate but STILL gates every freeze. The freeze gate is
  invariant across modes by design — it mutates frozen regression baselines, the
  docs' hard approval gate (workflow §6/§7). Auto-approving freezes is NOT an
  offered mode; if ever wanted, record the decision here first.

## Change log (applied changes, append-only)

Dated, sourced entries for changes already made. Open, not-yet-applied ideas go
under **Open recommendations** above.

- **2026-06-14 (first real invocation).** The skill auto-presumed SHIP = Drake
  Clipper instead of asking, rationalising "only one populated folder / the ship
  this session was on." Root cause: step 1 said "resolve by matching," which
  reads as a licence to auto-select. Fix landed same day: step 1 now mandates an
  `AskUserQuestion` ship confirmation (discovered folders as options + "Other"
  free-text), explicitly "even when only one folder exists," and a red-flag row
  counters the presume-the-single-folder rationalisation.
- **2026-06-14 (authoring refinement, owner-requested).** Front-loaded
  interaction model firmed up: SHIP, REFERENCE, and SCOPE are now each confirmed
  via `AskUserQuestion` before the autonomous loop starts — ship pick, reference
  image confirmation (yes / different file), then "work automatically vs. name
  specific issues." Keeps the loop hands-off while making the three decisions
  that steer the whole arc explicit user choices.
- **2026-06-14 (second real invocation).** On a re-run the skill reused the
  previously-confirmed SCREEN and gave no chance to re-select. Same root-cause
  category as the SHIP presumption: carrying session/prior-arc state into a
  fresh run. Fixed at the category level rather than per-input — added an
  "Every invocation starts cold" lead-in (SHIP, SCREEN, REFERENCE, SCOPE all
  asked fresh, never inherited), rewrote step 2 to always ask SCREEN via
  `AskUserQuestion` (lists the folder's screens for re-select + "Other", with a
  prose-list fallback when screens exceed the 4-option limit), and added a
  red-flag row. Watch for the next variant of this class (reusing a reference
  variant or scope) — the cold-start lead-in should now pre-empt it.
- **2026-06-14 (checkpoint UX, owner-requested).** Checkpoints (freeze, commit,
  final parity) now request permission via `AskUserQuestion` with an explicit
  approve/decline choice, not a prose stop — consistent with the input
  questions. Added a red-flag row against acting on a presumed "yes." Reference
  confirmation already used the tool.
- **2026-06-15 (automatic-mode autonomy).** In automatic SCOPE mode the skill
  was asking which catalog item to tackle next. Clarified that automatic mode
  orders the catalog by priority (workflow §4) and works top-down without
  per-item check-ins — when an item is fixed/deferred/blocked it pulls the
  next-highest itself; the four checkpoints remain the only interruptions. Added
  a red-flag row. (Named-issues mode still works the user's list, also in
  priority order.)
- **2026-06-15 (two autonomy modes).** Split the single automatic mode into
  **semi-automated** (gates commits + freezes + final parity — the prior
  behaviour) and **fully automated** (auto-commits, drops the final-parity gate,
  but STILL gates freezing). Chosen at launch via the SCOPE & MODE question
  (now two questions in one prompt: scope = full review vs named issues; mode =
  semi vs fully). Checkpoints section is now a mode × checkpoint table; freeze
  is the invariant gate. Added red-flag rows against auto-freezing in fully
  automated mode.
- **2026-06-15 (fix without asking).** The skill was pausing to ask permission
  after tracing a root cause. Clarified that applying a structural fix is the
  loop's job, not a checkpoint — once the owning stage is identified, edit source
  directly (TDD: failing test first). Only freeze/commit/final gate. Updated the
  Fix step and added a red-flag row.
- **2026-06-15 (sequential dependent questions).** The skill batched SCREEN and
  REFERENCE into one prompt, so the reference options were pre-computed from a
  presumed/default screen instead of the one the user actually picked. Root
  cause: REFERENCE options depend on the SCREEN answer. Fixed by mandating
  separate, SEQUENTIAL `AskUserQuestion` calls for the dependent chain
  (SHIP → SCREEN → REFERENCE), discovering reference candidates only after the
  screen answer returns; only SCOPE & MODE (mutually independent) may share one
  prompt. Added a lead-in rule, a step-3 "only after SCREEN is answered" clause,
  and a red-flag row. General lesson for this skill: never pre-compute a later
  question's options from a guessed earlier answer.
- **2026-06-15 (don't defer on estimated risk).** The skill was deferring items
  on a self-assessed "large blast radius / deserves its own change" estimate
  instead of doing them. Raised the deferral bar to a PROVEN concrete blocker;
  size/risk now triggers research + (if large) a short plan + execution. Points
  at the empirical disable→adjudicate audit (workflow §5) + `ui_check.sh --full`
  to MEASURE blast radius rather than estimate it (matches the arc-memory lesson
  "measure, don't estimate" — paper analysis repeatedly mis-called "blocked").
  Reassures that making the change is autonomous and only the resulting freeze is
  gated. Added "Default to fixing, not deferring" paragraph + two red-flag rows.
- **2026-06-15 (review consolidation).** Review of the log surfaced that 6 of 8
  findings are one class — agent substitutes a guess for an ask or a
  measurement. Added an **Operating posture** section to `SKILL.md` (ASK inputs /
  MEASURE judgments / JUST-DO the work / GATE hard-to-reverse actions) so the
  next novel shortcut is caught without its own red-flag row. Also fixed this
  doc's purpose drift: it had become a changelog of done edits though it was
  meant for open ideas — split into **Open recommendations** (lifecycle: retro
  appends → writing-skills applies → moves here) and **Change log**, and
  refreshed the stale "Not yet executed" validation status.
- **2026-06-15 (subagents for research).** Added a "Use subagents for read-only
  research" section: delegate parallel-safe, READ-ONLY work (per-item root-cause
  research via the MCP trio + dcb_canvas mirror, guard-trip archaeology,
  measuring an existing render+reference pair, doc/data lookups) to subagents
  that report back; keep everything sharing the cargo target — `cargo
  build`/`test`, `ui_check.sh`, `ui_render.sh`, `entity export`, freezes — plus
  the fix/TDD test and gated actions sequential in the main agent (AGENTS.md:
  avoid broad parallelism for shared compilation targets). Subagents are told
  explicitly not to build/test/render/export/edit. Added a red-flag row + a
  pointer from the "Default to fixing" paragraph.
- **2026-06-15 (retrospective was skipped).** The skill ended without running
  the self-improvement retrospective — it was a soft pointer to the external
  `ui-process-retro-prompt.md` in the last section, easy to drop (especially in
  fully-automated mode, which stops after the final-parity report). Fixes:
  reframed it as a MANDATORY closing step ("arc done = retro done", both modes);
  inlined the 7 sweep categories so the skill is self-contained instead of
  leaning on an external prompt; split destinations (process/tool/doc findings →
  `ui-process-improvements.md` ledger; skill findings → Open recommendations);
  instructed tracking it as a TodoWrite item from arc start; added a "loop ends,
  arc doesn't" clause and a red-flag row. External prompt kept as the canonical
  option for the verbatim sweep.
- **2026-06-15 (catalog accuracy + confirm gate).** Two real defects in review:
  (a) findings were mostly right but occasionally outright wrong (square seen as
  circle); (b) the background/backplate layer (stretch, misalignment) was not
  considered. Added a "Build & confirm the diff catalog" phase at arc start
  (both modes, fresh or previously-worked screen): build → self-verify each
  finding by looking AGAIN (guard shape/count/offset-direction misreads, measure
  when unsure) → explicitly check the background layer → confirm the catalog with
  the user via `AskUserQuestion` (with free-text comments) before any fixing.
  Added a launch checkpoint row, an Operating-posture MEASURE clause, a loop
  Compare-bullet note (same checks on newly-surfaced diffs; re-confirm only on
  material change), and three red-flag rows.
- **2026-06-15 (closing re-review).** Added a mandatory closing re-review before
  the retrospective (both modes): re-render fresh + re-compare/self-verify
  against the reference exactly as the opening catalog phase (look-again +
  background layer), building a fresh catalog of what remains. Fully automated
  feeds remaining fixable diffs back into the loop and repeats until clean (or
  proven deferred/blocked); semi-automated surfaces the fresh re-review at the
  Final-parity `AskUserQuestion` gate (accept vs another pass). Updated the
  checkpoints table/bullet (fully-auto Final parity is now "re-review → fix until
  clean", not "report only"), the loop closing line, and added two red-flag rows.
- **2026-06-15 (inherit-verdicts clause; applied from Open recommendations).**
  The g-force arc's retro found the opening catalog UNDER-called the screen by
  carrying prior "faithful / owner-confirmed / not-flagged" verdicts forward
  (missed marker desync, at-rest "0", right-edge line; the user expanded the
  catalog at the confirmation gate — backstop worked). Applied to the catalog
  phase: "**Inherited verdicts are hints, not conclusions**" — re-derive each
  region from the reference at high zoom (small features hide in a naked
  side-by-side), with the scoping refinement that a frozen-baseline or
  registered-known-outlier region routes any re-opened verdict through the
  §5/§6/§7 audited flow, not a silent fix (so it doesn't collide with
  settled-numbers-are-lookups / frozen-baseline discipline). Added a red-flag row.
