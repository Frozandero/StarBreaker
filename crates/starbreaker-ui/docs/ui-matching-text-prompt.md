# UI Matching Agent Prompt Template (Text-Only)

Use this template when the agent cannot view images directly and relies on
the user to catalog visual differences between a reference screenshot and
a generated UI render.

## Design

- The agent does **not** compare images.
- The user (or a vision-capable assistant) provides a structured difference
  catalog upfront.
- The agent uses that catalog as the working backlog and drives the fix
  iterations via MCP tools, CLI probes, and code changes.
- After each fix, the agent asks the user to re-render and report any
  remaining differences.

## Copy/Paste Prompt

```text
You are working in StarBreaker, in crate starbreaker-ui.

Before you plan or edit anything, read these files in order:
1. StarBreaker/AGENTS.md
2. StarBreaker/.github/copilot-instructions.md
3. StarBreaker/crates/starbreaker-ui/AGENTS.md
4. StarBreaker/crates/starbreaker-ui/docs/ui-matching-workflow.md

Goal:
Match the generated UI to the provided reference screenshot. You cannot view
images directly — the user will supply a structured difference catalog and
validate each fix visually.

Reference image: <USER_FILLS_IN>
Generated image: <USER_FILLS_IN>

To export the updated image after making changes:
  cd ~/projects/scorg_tools/StarBreaker && cargo run -p starbreaker --release -- entity export "<entity>" "~/projects/scorg_tools/ships" --kind decomposed --lod 0 --mip 0 --materials all

Operating rules:
1. Follow `crates/starbreaker-ui/docs/ui-matching-workflow.md`.
2. Use StarBreaker MCP tools first for investigation; use CLI export for
   rendering and regression artifacts.
3. For UI style/layout questions, run dedicated MCP diagnostics before
   ad-hoc probes or code edits:
   - `ui_canvas_style_inventory` — authored style containers and entries.
   - `ui_scene_style_probe` — scene nodes, tags, colours, matched styles.
   - `ui_ir_query` — compile canvas to canonical IR.
4. **CRITICAL: If MCP tools are unavailable, use CLI fallback commands
   immediately.** Do not waste iterations calling missing tools.
5. Keep IR as styling authority. Do not invent style semantics in renderer
   code.
6. No hard-coded per-screen or per-name branches in production logic.
7. If a change has no measurable effect, remove it immediately.
8. Keep code lean: no dead helpers, stale fallback paths, or speculative
   logic left behind.
9. Run regular regression checks to prevent frozen-image regressions.

## MCP Fallback (when tools are unavailable)

If MCP tools are missing, use these CLI equivalents:

### IR Compilation (replaces `ui_ir_query`)
```bash
cd StarBreaker
SC_DATA_P4K="$HOME/Games/star-citizen/drive_c/Program Files/Roberts Space Industries/StarCitizen/LIVE/Data.p4k" \
  cargo run -p starbreaker --release -- ui debug <canvas_source_path>
```

### Canvas Style Inventory (replaces `ui_canvas_style_inventory`)
```bash
SC_DATA_P4K="..." \
  cargo run -p starbreaker --release -- ui styles <canvas_source_path>
```

### Direct File Inspection
- Canvas JSON: `ships/Data/UI/Generated/ship/<manufacturer>/<ship>/<canvas>.json`
- SWF assets: `ships/Data/UI/BuildingBlocks/assets/SWF/`
- P4k: use `p4k_list` and `p4k_read` MCP tools

### SWF/Canvas Source Location
- P4k: `Data/UI/BuildingBlocks/assets/SWF/Canvas.swf`
- Decomposed: `ships/Data/UI/Generated/ship/<manufacturer>/<ship>/`

## Expected Canvas Structure by Screen Type

**Target/status screens** (mc_s_target):
- State-bound text widgets (NO TARGET / TARGET_NAME / LOCKED)
- Dashed separator lines (top/bottom)
- Navigation footer bar (<< TARGET_STATUS >>)
- Corner bracket decorations
- State-driven visibility

**Annunciator screens** (h_eng_annunciator):
- State-tagged items (StateModerate, StateCritical, StateFlashing)
- Accent color mapping (Accent2=warning, Accent3=critical)
- Grid layout of indicator items

**MFD screens** (mfd_screen):
- Complex multi-widget layouts, asset/image references
- Dynamic binding to ship state

## Known Pain Points
- **State-bound visibility**: elements use state tags to control visibility.
- **Widget tree resolution**: custom widget types may not be resolved.
- **Layout engine limitations**: elements parsed but not laid out correctly.
- **Style-tag drift**: IR fields drive draw-time behavior, not tags.
- **Alpha suppression**: zero-alpha elements invisible even if parsed.
- **Text metric drift**: font resolution and bounds vs. game rendering.

## REQUIRED: User Difference Catalog

Before the agent begins work, the user must provide a structured difference
catalog. If the catalog below is empty, the agent MUST ask the user to
compare the reference and generated images and report differences.

--- USER DIFFERENCE CATALOG (fill in or leave blank to trigger agent query) ---

| # | Category  | Element Description | Reference State | Generated State | Probable Owner |
|---|-----------|---------------------|-----------------|-----------------|----------------|
| 1 |           |                     |                 |                 |                |
| 2 |           |                     |                 |                 |                |

--- END USER DIFFERENCE CATALOG ---

Category options: text, shape, image, color, position, size, visibility, stroke, fill, alignment, scale
Probable owner options: source_data, bb_layout, ui_ir, ir_compose, unknown

## Required Workflow

### Phase A — Receive or Request Catalog

If the user difference catalog is populated:
- Acknowledge receipt.
- Validate each entry for clarity; ask the user to clarify vague
  descriptions (e.g. "it looks off" becomes "the dashed line is 10px lower").
- Skip to Phase B.

If the catalog is empty:
- Ask the user to compare the reference and generated images and report
  differences using the table format above.
- Provide guiding questions:
  - Are there missing elements (shapes, text, images, lines)?
  - Are there extra elements not in the reference?
  - Do text labels match in content, size, and color?
  - Are colors/tints correct (including brand accent colors)?
  - Are positions, spacing, and alignment correct?
  - Are borders, strokes, or fills present or missing?
  - Are any elements visible that should be hidden, or vice versa?
- Wait for the user's response before proceeding.

### Phase B — Plan

- From the catalog, produce a concrete execution plan.
- Order items by dependency and regression risk.
- Define success criteria for each item using measurable outcomes
  (IR/query values and rendered results), not only visual opinion.
- Present the plan to the user for approval.

### Phase C — Execute Iteratively

For each catalog item (in plan order):
1. Investigate with MCP tools or CLI probes. Capture evidence.
2. Form one falsifiable hypothesis about the root cause.
3. Make one focused code change.
4. Run the smallest relevant test/query to verify the change has effect.
5. Re-render the target artifact and ask the user:
   "Please compare the new render. Is catalog item N resolved?
    If not, describe what still differs."
6. If the user confirms resolved, mark the item complete.
7. If not resolved, update the catalog entry with the new state and continue.
8. Do not keep no-effect code. Remove experiments immediately.

### Phase D — Regression Safety

Run the UI regression suite regularly (not just once at the end):
- If any platinum/gold baseline regresses, fix the root cause.
- Do not weaken tests to accommodate a regression.

### Phase E — Completion

- Continue until all cataloged differences are resolved.
- Provide final report:
  - resolved differences (list by catalog item number)
  - remaining differences with proven blocker evidence
  - tests run and outcomes
  - code cleanup summary (what was removed as no-effect/stale)

```

## Usage Notes

- Provide the agent with both image paths explicitly.
- Include the canvas GUID, target name, and render command currently used.
- **State clearly whether MCP tools are available** — if not, the agent
  should immediately use CLI fallback commands.
- If you know recurring pain points for this screen, append a short
  "watch for" list (e.g. style-tag drift, alpha suppression, text drift).
- Keep added context concise; rely on referenced docs for detailed policy.

## Lessons Learned (from Drak Clipper Target Screen)

**What went wrong:**
1. MCP tools were not available but prompt assumed they existed.
2. No CLI fallback commands were documented.
3. Canvas source location was unclear to the agent.
4. No guidance on expected screen structure.

**What would have helped:**
1. A note stating which MCP tools are available.
2. CLI fallback commands documented upfront.
3. Direct path to canvas JSON/SWF source.
4. Expected element list for the screen type.
5. State tag awareness for visibility logic.

## Per-Task Findings

For each matching task, record Phase A findings in a separate file at:
`docs/ui-matching-tasks/<canvas-name>-findings.md`

Example: `docs/ui-matching-tasks/target-master-findings.md`

This keeps the template generic while capturing task-specific data.
