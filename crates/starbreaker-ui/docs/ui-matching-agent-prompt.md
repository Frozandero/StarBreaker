# UI parity — per-screen agent prompt

Instantiate the variables and paste. All process knowledge lives in
`docs/ui-workflow.md` + `docs/ui-reference.md` (repo `docs/`); keep this
prompt short and let those docs do the arming.

## Template

```text
You are working in /home/tom/projects/scorg_tools/StarBreaker (branch
feature/ui), crate starbreaker-ui.

Read, in order:
1. StarBreaker/AGENTS.md
2. crates/starbreaker-ui/AGENTS.md
3. docs/ui-workflow.md      (the process — follow it exactly)
4. docs/ui-reference.md     (commands, tools, data; find SCREEN in §3 dossier)

SCREEN=<dossier row, e.g. Screen_Left_Lower_RTT>
HANDOFF=<optional, e.g. docs/ui-clipper-parity-handoff.md — read it if set>
KNOWN_SYMPTOMS=<optional user-observed differences, else start from review>

Goal: get SCREEN's render closer to its reference image (dossier row gives
scene, canvas, reference, compare preset, frozen tier, open issues).

Work: replay-render SCREEN; run scripts/ui_compare.py with the preset and
READ the crops; build/extend the numbered diff catalog; then the TDD loop
from docs/ui-workflow.md §3 — failing test first, one structural fix at the
owning stage, scripts/ui_check.sh every cycle, re-render + compare, update
the catalog and the arc memory/handoff. Guard trips → §5 adjudication
(structural discriminators, never names). Baselines/outliers only via §6/§7
with explicit approval. Do not stop until the catalog items are fixed,
explicitly deferred with reasons, or a concrete blocker is proven.
```

## Example (power screen)

`SCREEN=Screen_Left_Lower_RTT`,
`HANDOFF=docs/ui-clipper-parity-handoff.md` — the dossier maps it to the
LOD0 scene, canvas `MC_S_Power_Master`, reference
`reference/in-game/Clipper/Screen_Left_Lower_RTT.png`, preset `power`.
