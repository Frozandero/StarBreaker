---
name: ui-screen-parity
description: Use when getting a StarBreaker ship UI screen's render to match its in-game reference capture — e.g. "make Drake Clipper Screen_Right_Upper_RTT match its reference". Triggers: UI parity arc, screen render vs reference/in-game/<Ship>/<Screen>.png, closing visual gaps on cockpit/MFD/HUD/medical/door screens.
---

# UI Screen Parity

## Overview

Get ONE ship UI screen's generated render as close to its in-game reference as
the reference's known limits allow, **engine-faithfully and generically**. This
skill ORCHESTRATES the authoritative process; it does not replace it. The
how-to-work is `crates/starbreaker-ui/docs/ui-workflow.md`; the what-to-type +
per-screen dossier is `crates/starbreaker-ui/docs/ui-reference.md`. Follow them
exactly.

**Core principle:** render wrong → fix the owning *upstream* stage so IR is
correct. Never name-gate, never hard-code a value, never tune a magic offset.
Reference captures are imperfect (resolution, bloom, CRT/skew, mouse-hover
artifacts) — match **structurally**, not pixel-naively. The target is "as close
as the reference allows," not pixel-identity.

Run **autonomously** through the TDD diff-catalog loop. How many gates interrupt
it is chosen at launch (SCOPE & MODE step): **semi-automated** gates commits and
freezes; **fully automated** auto-commits and gates only freezes. **Freezing a
baseline is ALWAYS gated, in every mode.**

**Worked example:** SHIP `Drake Clipper`, SCREEN `Screen_Right_Upper_RTT` →
folder `Clipper`, reference `reference/in-game/Clipper/Screen_Right_Upper_RTT.png`;
the dossier maps it to the LOD0 scene, canvas `MC_S_Target_Master`, preset
`target`, GOLD `clipper_target_master`.

## Operating posture

Nearly every way this skill goes wrong is the same shortcut: substituting a
guess for an ask or a measurement. Default behaviour by category:

- **Inputs / user-facing choices → ASK.** Ship, screen, reference, scope & mode:
  fresh `AskUserQuestion` every run, sequentially. Never presume from session or
  prior-arc state; never pre-compute one question's options from a guessed
  earlier answer.
- **Technical judgments → MEASURE.** Blast radius, "is this a blocker?", root
  cause, colour/position: prove it with the probes, `ui_check.sh --full`, and
  the disable→adjudicate audit. Never estimate-then-defer. Visual findings must
  survive a SECOND look (guard square-vs-circle misreads, miscounts, wrong offset
  direction) and must include the background/backplate layer (stretch,
  alignment), not just foreground widgets.
- **The work itself → JUST DO IT.** Applying a structural fix (TDD, failing test
  first) is the loop's job, not a checkpoint.
- **Hard-to-reverse actions → GATE.** Freeze (always), plus commit & final
  parity (semi-automated): stop and ask permission via `AskUserQuestion`.

Asking the user and measuring are always safe; presuming and estimating are the
failure mode.

## Gather inputs (in order)

**Every invocation starts cold.** Ask SHIP, SCREEN, REFERENCE, and SCOPE fresh
on each run via `AskUserQuestion` — never inherit a choice from earlier in this
session or a prior arc, even when you already "know" the answer. Prior context
is a hint to surface, never a substitute for asking.

**Ask them as separate, SEQUENTIAL questions — never batch dependent ones into
one prompt.** Each answer determines the next question's options: the ship picks
the screen list, and **the chosen screen determines which reference images are
discovered.** Do NOT pre-compute a later question's options from a presumed
earlier answer — the classic bug is offering the reference image for a default
screen instead of the screen the user just picked. Wait for each answer before
building the next question. Only SCOPE & MODE (independent of each other) may
share one prompt.

1. **SHIP — always confirm via `AskUserQuestion`; never presume.** List the
   folders under `~/projects/scorg_tools/reference/in-game/` (each is one ship's
   reference set, e.g. `Clipper` = Drake Clipper) and present them as the
   options; the tool's automatic "Other" choice lets the user free-text a ship
   whose folder doesn't exist yet. Ask **even when only one folder exists** — a
   single populated folder (today only `Clipper`) is NOT a licence to
   auto-select, no matter what ship the session was previously working on. The
   chosen ship resolves to `reference/in-game/<folder>/`.
2. **SCREEN — always ask via `AskUserQuestion`; never reuse a prior choice.**
   Ask fresh even if a screen was confirmed earlier in the session or a previous
   arc — that is NOT a licence to skip the question. List the screens available
   in the confirmed ship's reference folder (distinct file stems, e.g.
   `Screen_Right_Upper_RTT`, `self_master`, `compass_master`) so the user can
   re-select or switch, and rely on the automatic "Other" choice for a screen
   not listed (no capture yet → adding its dossier row is part of the work). If
   the folder holds more screens than the tool's 4-option limit, show the full
   list in the message and offer the most relevant candidates + "Other". The
   chosen SCREEN is simultaneously the dossier row, the render `--helper`, and
   the reference file stem.
3. **REFERENCE — resolve, show, then CONFIRM via `AskUserQuestion` (hard stop).**
   **Only after SCREEN (step 2) is answered**, list files in the ship folder
   matching that CHOSEN screen — never a presumed/default screen. Prefer the
   straight-on capture carrying a `<name>.corners.json` sidecar over a legacy
   name
   (reference §3 — e.g. power uses `Screen_Left_Lower_RTT_dark.png`). **Read the
   chosen image and show it to the user**, then confirm with `AskUserQuestion`
   that it is the right screen (options: "Yes, use this" / "Pick a different
   file" — the automatic "Other" choice lets them point at another path or
   variant). Do NOT continue until confirmed.
4. **SCOPE & MODE — ask via `AskUserQuestion` (two questions, one prompt).**
   Once the reference is confirmed, ask:
   - **Scope:** "Full review of every region" vs "Specific issues I'll name"
     (the "Other"/free-text captures the user's observed symptoms). Named issues
     seed the diff catalog; the review still surfaces anything else it finds.
   - **Mode:** "Semi-automated — gate commits and freezes" vs "Fully automated —
     commit automatically, gate only freezes." Freezing is gated in BOTH modes
     (see Checkpoints); the mode only changes whether commits and the final
     parity check pause.
   Build/seed the catalog, then **order it by priority (workflow §4:
   structural/layout before styling, shared-root-cause items together) and work
   it top-down — never pause to ask which item to do next.** A HANDOFF doc named
   in the dossier's open-issues column is read either way.

## Required reads (before any fix)

In order: `StarBreaker/AGENTS.md` → `crates/starbreaker-ui/AGENTS.md` →
`crates/starbreaker-ui/docs/ui-workflow.md` (the process) →
`crates/starbreaker-ui/docs/ui-reference.md` (find SCREEN in the §3 dossier:
scene/LOD, canvas, compare preset, frozen tier, open issues). If SCREEN is NOT
in the dossier, adding its row is part of the work.

## Build & confirm the diff catalog (before any fixing — both modes)

Do this once at arc start, after the required reads, whether the screen is fresh
or has been worked before.

**Inherited verdicts are hints, not conclusions.** A prior arc's open issues, or a
dossier/memory/handoff label of "faithful / owner-confirmed / not flagged", SEEDS
this pass but never replaces it — such a label is a prior judgment that may have
under-scrutinized, or that the owner's view has since outgrown. Re-derive each
region's verdict from the reference at high zoom (small features — circle vs
squircle, a few-dozen-px marker offset — hide in a naked side-by-side). For a
region under a frozen baseline or a registered known-outlier, route any re-opened
verdict through the §5/§6/§7 audited flow (adjudicate → re-freeze / known-outlier),
not a silent fix.

Steps:

1. **Render + compare, build the catalog.** Use the loop's render + compare
   commands below; build the numbered diff catalog (region | difference |
   severity | root-cause hypothesis | fix-or-defer).
2. **Self-verify every finding — look AGAIN before trusting it.** Re-open the
   crops and re-check each item against the reference: is the difference real and
   correctly described? Findings are mostly right but occasionally outright wrong
   — guard specifically against misreading SHAPE (square vs circle vs ring),
   COUNT, presence/absence, and the DIRECTION of an offset. Drop or correct any
   finding that doesn't survive the second look; measure (`ui_measure.py`) when
   unsure rather than eyeball.
3. **Check the BACKGROUND / backplate layer explicitly** — not just foreground
   widgets. Is the background image stretched, wrong-aspect, scaled, offset,
   mis-aligned, or cropped versus the reference? Add any background diffs to the
   catalog; they are easy to miss and skew everything layered on top.
4. **Confirm with the user via `AskUserQuestion`.** Present the self-verified
   catalog, ask them to confirm or amend it, and invite free-text comments (the
   "Other" choice). Do NOT start fixing until confirmed — this gate runs in BOTH
   modes (semi- and fully automated).

## The autonomous loop (workflow §3–§4)

**Track the arc with TodoWrite from the start — include the mandatory closing
retrospective (below) as a todo so it is never dropped.**

Replay-render → compare → catalog → TDD fix → check → re-render. Per cycle:

- **Render:** `bash scripts/ui_render.sh --helper <SCREEN> [--ir]` (cockpit/HUD
  screens need LOD0 — the wrapper picks it; ledger 47).
- **Compare:** `python3 scripts/ui_compare.py <render> <reference> --regions
  <preset> [--stats]` and **READ every crop with vision**. Rectify for POSITION
  via `corners.json`; judge a **thin feature's COLOUR on the crisp original**
  (ledger 35). Settled reference numbers come from the measurement bank first.
  Apply the catalog phase's look-again accuracy check and background-layer check
  to any newly-surfaced diff; re-confirm with the user only if the issue set
  changes materially.
- **Catalog:** extend the numbered diff catalog (region | difference | severity
  | root-cause hypothesis | fix-or-defer) in the arc memory/handoff — deferrals
  are explicit entries, not omissions.
- **Fix (TDD):** failing test first → ONE structural fix at the owning stage
  (`bb_layout` rects / `ui_ir` preserved metadata / `ir_compose` draw / `text`
  metrics — workflow §2 table) → `bash scripts/ui_check.sh` every cycle →
  re-render + compare → update catalog + memory. **Once you've traced the root
  cause, apply the fix directly — never ask permission to fix it. Editing source
  to correct the IR is the loop's job, not a checkpoint.**
- **Guard trips:** that is the system working — adjudicate via workflow §5
  (structural discriminator, never a name). Baseline genuinely wrong → that's a
  freeze (a STOP). Known deferred miss → register a §6 known-outlier.

Work the catalog in priority order, highest first. When an item is fixed,
deferred, or blocked, take the next-highest open item yourself — **in automatic
mode, do not ask the user which to tackle next.** Don't stop the loop until
every catalog item is fixed, explicitly deferred with a reason, or a concrete
blocker is proven; the only interruptions are the active checkpoints below.
Resolving the catalog ends the loop, not the arc — a closing re-review and then
the mandatory retrospective (below) are the final steps in every mode.

**Default to fixing, not deferring.** "Large blast radius," "touches many frozen
nodes," "deserves its own deliberate change," or any size/risk estimate is NOT a
blocker — it is a signal to RESEARCH and PLAN, then fix within this arc (fan the
read-only research across subagents — see *Use subagents for read-only
research*). De-risk
a wide change empirically: run the disable→adjudicate audit (workflow §5) and
`bash scripts/ui_check.sh --full` so the frozen pins MEASURE the real blast
radius instead of you estimating it; find the one structural discriminator; for
a genuinely large change, write a short plan in the arc memory/handoff (affected
identities, sequence, the single rule) and execute it. **Making the change is
autonomous — only the resulting baseline FREEZE is gated** (a wide change that
moves many frozen baselines toward the reference is the workflow §5 "baseline is
wrong → re-freeze" path; present that delta at the freeze gate). Defer ONLY on a
PROVEN concrete blocker (a data signal you demonstrated absent, a dependency on
un-landed work) and record that proof, not a guess. Effort or risk alone never
justifies deferral.

## Use subagents for read-only research (never for builds)

Fan work out to subagents where it CANNOT conflict; keep anything touching the
shared cargo target or source single-threaded.

**Delegate — parallel-safe, READ-ONLY:**
- Root-cause research for independent catalog items: one subagent per item to
  trace it through the dcb_canvas mirror + the MCP trio
  (`ui_canvas_style_inventory` → `ui_scene_style_probe` → `ui_ir_query`) and
  report back the owning stage + structural discriminator.
- Guard-trip archaeology across many frozen nodes (workflow §5).
- Measurement / catalog-building from an EXISTING render+reference pair
  (`ui_compare.py`, `ui_measure.py`, reading crops with vision).
- Doc / dossier / DataCore / P4K lookups.

Each reports findings back; you synthesize and own the fix decision.

**Never delegate — sequential in the main agent:**
- Anything that runs `cargo build`/`test`, `ui_check.sh`, `ui_render.sh`,
  `entity export`, or a freeze: they share the cargo target dir and parallel
  builds race (AGENTS.md: avoid broad parallelism for shared compilation
  targets). Build / render / validate one at a time.
- The fix itself (one falsifiable change at the owning stage) and its TDD test.
- Gated actions (freeze / commit).

Tell each research subagent explicitly: **do not build, test, render, export, or
edit — only query, measure, and report.**

## Checkpoints (which apply depends on the mode)

At each ACTIVE checkpoint, present the evidence, then **ask permission with
`AskUserQuestion`** (a concrete approve/decline choice). Never proceed on a
presumed "yes"; never perform the action before the answer comes back.

| Checkpoint | Semi-automated | Fully automated |
|---|---|---|
| Reference selection (launch) | ask | ask |
| Diff-catalog confirmation (launch) | ask | ask |
| **Baseline freeze / re-freeze** | **gate** | **gate** |
| Git commit | gate | auto, no gate |
| Final parity (closing re-review) | gate | re-review → fix until clean |

- **Freeze is ALWAYS gated, in both modes** — §6/§7 are APPROVAL-GATED. Show the
  per-identity delta, then ask via `AskUserQuestion` ("Freeze these N
  identities?" → "Freeze" / "Don't freeze"). Never auto-approve, never freeze a
  value you can't explain.
- **Git commit** — semi-automated: show the diff/summary, ask "Commit this?" →
  "Commit" / "Not yet", commit only on yes. Fully automated: commit autonomously
  per coherent fix (message cites the catalog item), no question.
- **Final parity** — driven by the **Closing re-review** (below). Semi-automated:
  present that fresh re-review and ask whether parity is acceptable or another
  pass is wanted (another pass → resume fixing, then re-review again). Fully
  automated: the re-review keeps fixing until the screen is clean or the
  remainder is proven deferred/blocked, then finishes (no gate).
- **Reference selection** — the launch questions (steps 1–4), both modes.

## Strict rules (workflow §1 — non-negotiable)

- Generic + engine-faithful: no hard-coding, no name/ship/screen/manufacturer
  branches, no magic offsets/blend factors, **no hard-coded game-data VALUES**
  (palette literals, font sizes, brand font lists — fallbacks AND test fixtures
  included). Self-correcting: replace or flag pre-existing hard-coding in the
  SAME change; never extend it because precedent exists.
- IR is the sole styling authority; fix the owning upstream stage, not the
  draw-time symptom.
- Frozen platinum/gold guards are never silenced by editing tests/baselines;
  baselines move only through the audited freeze flow or a §6 known-outlier.
- 3000-line cap; remove no-effect experiments immediately (revert + record what
  was falsified); verify-on-write any doc command line you add.

## Closing re-review (before the retrospective — both modes)

When the catalog is resolved, do NOT trust it is done — re-evaluate from scratch,
exactly as the opening *Build & confirm the diff catalog* phase did:

1. **Re-render the screen fresh** and **re-run compare + self-verify** against the
   reference: look AGAIN (guard shape/count/offset misreads), and re-check the
   BACKGROUND/backplate layer (stretch, alignment). Build a fresh diff catalog of
   what REMAINS — including anything the fixes introduced or missed.
2. **Fully automated:** if any fixable difference remains, feed it back into the
   loop and FIX it; repeat re-render → re-review → fix until the screen is clean —
   every difference fixed or carrying a PROVEN deferral/blocker (size/risk is not
   a deferral; freeze stays gated). Do not finish while fixable diffs remain.
3. **Semi-automated:** this fresh re-review IS the Final-parity checkpoint —
   present it via `AskUserQuestion` ("parity acceptable" vs "another pass", with
   free-text comments); "another pass" resumes fixing, then re-reviews again.

Only once the closing re-review is clean (or its remainder is proven
deferred/blocked) does the arc proceed to the retrospective.

## Self-improve every arc: the retrospective (MANDATORY closing step)

**The arc is not done when the catalog is — it is done after the retrospective.**
Run it in the SAME session (lived context), in BOTH modes, before declaring
complete. It is the TodoWrite item added at arc start; do not close the arc with
it open.

Sweep this session's lived experience (friction, dead ends, retyped commands)
across these categories — for each, FIX it, don't just note it:

1. **Repeated manual work → tooling.** Anything typed >2× (ad-hoc crops, probe
   greps, command batteries) extends a `scripts/`/`examples/` tool (extend
   before creating).
2. **Silent failures → loud.** Any harness/guard that gave a wrong-but-plausible
   answer gets a distinct hard failure.
3. **Doc drift.** Every doc claim you relied on that was wrong/stale gets fixed
   with verify-on-write (run the command; repo-wide grep for renamed/deleted
   references in the same commit; `docs_reference_guard` covers new file-citing
   docs).
4. **Bootstrap cost.** Everything you had to RE-DERIVE (data locations, screen
   mappings, engine rules, probe names, don't-retry traps) lands in
   `ui-reference.md` (dossier / probe registry / glossary) or `ui-workflow.md` §10.
5. **Guard/freeze friction.** Detoured adjudications, hand-audited deltas,
   late-registered outliers → improve the flow or the doc that teaches it.
6. **Memory/handoff quality.** Would the handoff resume cold? Fix the §9
   expectations, not just this arc's file.
7. **Slow tooling → profiled speedup.** A tool you waited on repeatedly: MEASURE
   first (usually harness load, not a loop), prefer an existing faster path, then
   cut the dominant cost; VERIFY the output is unchanged.

Two destinations:

- **Process / tool / doc findings →** APPEND numbered items to
  `crates/starbreaker-ui/docs/ui-process-improvements.md` (the ledger;
  Observed/Improvement/Action format) and IMPLEMENT them — quick tooling wins
  first, then docs; one commit per coherent item citing its ledger number;
  `bash scripts/ui_check.sh` green per commit; process changes must not alter
  render behaviour. Baseline-affecting actions stay APPROVAL-GATED (the freeze
  gate). The external prompt `ui-process-retro-prompt.md` is the canonical
  version of this sweep — run it verbatim if you prefer; the categories above
  are its essence so the skill is self-contained either way.
- **Improvements to THIS skill →** append under **Open recommendations** in
  `recommendations.md` (next to this file); do not rewrite `SKILL.md` mid-arc.

Acceptance (bootstrap test): a fresh agent could run the next arc from
`ui-workflow.md` + `ui-reference.md` + the dossier alone. Any excursion you
needed is a doc bug — fix it before closing.

## Red flags — STOP, you're rationalizing

| Thought | Reality |
|---|---|
| "Just hard-code this one value/offset" | Banned, even in fixtures/fallbacks. Find the structural cause. |
| "Render differs from ref, so the render is wrong" | Captures have bloom/skew/resolution/hover artifacts. Compare structurally. |
| "I see a square / it's shifted left" (first glance) | Look AGAIN before cataloguing — first-glance shape/count/offset reads are the ones that turn out wrong. Measure if unsure. |
| "Only the foreground widgets differ" | Check the background/backplate layer too — stretch, scale, aspect, alignment, crop. It's the commonly-missed layer and it skews everything on top. |
| "The findings look right, start fixing" | First self-verify (look again + background), then confirm the catalog with the user via `AskUserQuestion`. Both modes, every arc. |
| "Memory says this region is faithful/owner-confirmed — skip it" | Inherited verdicts are hints, not conclusions. Re-derive each region from the reference at high zoom; the owner's view evolves and earlier passes under-scrutinize. Frozen/outlier regions route through §5/§6/§7. |
| "Freeze it so the guard passes" | Freezing a wrong value enshrines the miss. Fix the cause or register a §6 outlier. Freezing is a STOP for approval. |
| "Only one ship folder, so I'll presume it" | Always confirm SHIP via `AskUserQuestion`. One populated folder — or the ship the session was already on — is not a licence to auto-select. |
| "We picked the ship/screen earlier, reuse it" | Every run starts cold. Ask SHIP, SCREEN, REFERENCE, and SCOPE fresh via `AskUserQuestion`; a choice from earlier in the session or a prior arc is never carried over silently. |
| "Skip the reference confirmation, it's obvious" | Mandatory stop. The wrong reference wastes the whole arc. |
| "Batch the screen + reference questions to save a round-trip" | They're dependent — REFERENCE options come from the SCREEN answer. Ask sequentially; batching offers the reference for a presumed screen, not the chosen one. |
| "The skill summary is enough" | Read ui-workflow.md + ui-reference.md. They are the authority; this skill only orchestrates. |
| "I'll freeze without asking to save a round-trip" | Freezing a baseline is ALWAYS gated, in BOTH modes — show the delta and ask via `AskUserQuestion`. (Commits are gated in semi-automated; automatic in fully automated.) |
| "Fully automated, so I'll auto-freeze too" | No. Fully automated drops only the commit and final-parity gates; the freeze gate always stays. |
| "They'll obviously say yes, I'll just do it" | Ask anyway via `AskUserQuestion` at an active checkpoint. A presumed approval is not an approval. |
| "Which catalog item should I fix next?" | Never ask. Order by priority (workflow §4) and work top-down; only the active checkpoints interrupt. |
| "I found the root cause — should I fix it?" | Just fix it. Applying a structural fix is the loop's work, not a gate. Only freeze/commit/final can interrupt. |
| "Large blast radius / many frozen nodes — defer it" | Size/risk is not a blocker; it's a cue to research + plan, then fix. MEASURE the real impact with the disable→adjudicate audit + `ui_check.sh --full`; defer only on a PROVEN blocker. |
| "This deserves its own deliberate change later" | If it's the right fix, do it now: research, plan if large, execute. The change is autonomous; only the freeze is gated. |
| "Spin up parallel agents to build/render faster" | Builds share the cargo target and race. Only READ-ONLY research parallelizes; builds/renders/tests/fixes/freezes stay sequential in the main agent. |
| "Catalog's resolved — run the retro" | First do the Closing re-review: re-render + re-compare against the reference like the start (self-verify + background). Fully automated keeps fixing until clean. THEN the retro. |
| "Fixes are done — the arc's complete" | Not complete until the closing re-review is clean (or proven deferred) AND the retrospective runs (mandatory, both modes). Track it as a TodoWrite item from arc start; don't close with it open. |
