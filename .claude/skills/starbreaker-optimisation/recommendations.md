# starbreaker-optimisation — recommendations

Append improvements to **THIS skill** here during a pass (do not rewrite
`SKILL.md` mid-pass). Process/tooling/profiling findings go to
`docs/optimisation-ledger.md` instead; this file is only for changes to how the
skill itself should guide the next pass.

Format per item: **Observed** (what friction/gap surfaced) → **Recommendation**
(the concrete SKILL.md edit) → **Status** (open / applied in commit `<sha>`).

## Open recommendations

_(none yet)_

## Applied

- **Skill created (2026-06-21).** Distilled from the Idris export optimisation arc:
  the O(depth) canonicalisation win (landed) and the parallel interior-sidecar
  rewrite (byte-identical + deterministic + memory-safe but **+26s slower**,
  reverted). That arc is the "watched it fail" grounding for the
  profile-parallelism-before-you-build rule and the keep-or-revert discipline.
  Initial flow + red-flags table seeded from it.
