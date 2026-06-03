# UI Matching Agent Prompt Template

Use this template to prompt an agent for end-to-end UI parity work against a
reference screenshot.

## Copy/Paste Prompt

```text


You are working in StarBreaker, in crate starbreaker-ui.

Before you plan or edit anything, read these files in order:
1. StarBreaker/AGENTS.md
2. StarBreaker/.github/copilot-instructions.md
3. StarBreaker/crates/starbreaker-ui/AGENTS.md
4. StarBreaker/crates/starbreaker-ui/docs/ui-matching-workflow.md

Generated image: /ships/Data/UI/Generated/ship/drak/Clipper/buildingblocks_canvas_mc_s_target_master.png 
Reference image: /reference/in-game/Clipper/Screen_Right_Upper_RTT.png 
To export the updated image after making changes: cd ~/projects/scorg_tools/StarBreaker && cargo run -p starbreaker --release -- entity export "drak_clipper" "~/projects/scorg_tools/ships" --kind decomposed --lod 0 --mip 0 --materials all

Goal:
Match the generated UI image to the provided reference image end-to-end, and do
not stop until all cataloged differences are resolved or a concrete blocker is
proven.

Important context:
- Reference screenshots are imperfect. They may be non-perpendicular, skewed,
	perspective-distorted, offset, partially occluded, not pixel-identical in
	resolution, or include in-game rendering artifacts.
- Do not assume perfect 1:1 pixel alignment from the screenshot alone.
- Use structural comparison, not naive pixel matching.

Operating rules:
1. Follow `crates/starbreaker-ui/docs/ui-matching-workflow.md`.
2. Use the correct tool for the file location:
	 - **Local workspace files** (generated PNGs, canvas JSON from decomposed export,
	   reference screenshots): read directly with `read_file`. MCP tools do NOT work
	   on these — they only access P4k contents.
	 - **P4k-native assets** (DDS textures, DataCore records, SWF files, chunk data):
	   use StarBreaker MCP tools.
	 - **Rendering and regression artifacts**: use CLI export commands.
3. For UI style/layout questions, run the dedicated MCP diagnostics before
	 ad-hoc shell probes or code edits:
	 - `ui_canvas_style_inventory` to locate authored style containers and entries.
	 - `ui_scene_style_probe` to confirm scene nodes, tags, raw colour fields, and
		 matched applied style entries.
	 - `ui_ir_query` to compile the canvas to canonical IR and return matching nodes.
4. **CRITICAL: If MCP tools are unavailable, use CLI fallback commands immediately.**
	 Do not waste iterations trying to call missing MCP tools. See "MCP Fallback" section.
5. Keep IR as styling authority. Do not invent style semantics in renderer code.
6. No hard-coded per-screen or per-name branches in production logic.
7. If a change has no measurable effect, remove it immediately.
8. Keep code lean: no dead helpers, no stale fallback paths, no layered
	 speculative logic left behind.
9. Run regular regression checks to prevent frozen-image regressions.

## MCP Fallback (when tools are unavailable)

If the MCP server does not expose `ui_canvas_style_inventory`, `ui_scene_style_probe`,
or `ui_ir_query` tools, use these CLI equivalents instead:

### IR Compilation (replaces `ui_ir_query`)
```bash
cd StarBreaker
SC_DATA_P4K="$HOME/Games/star-citizen/drive_c/Program Files/Roberts Space Industries/StarCitizen/LIVE/Data.p4k" \
  cargo run -p starbreaker --release -- ui debug <canvas_source_path>
```
This compiles the canvas to canonical IR and prints the result for analysis.

### Canvas Style Inventory (replaces `ui_canvas_style_inventory`)
```bash
SC_DATA_P4K="..." \
  cargo run -p starbreaker --release -- ui styles <canvas_source_path>
```
Lists authored style containers, embeddedStyles, defaultStyles, and brandStyles.

### Direct File Inspection
If the canvas source is a JSON file in the decomposed export, read it directly
with `read_file` — it is a local workspace file, NOT a P4k asset:
- Canvas JSON (local): `ships/Data/UI/Generated/ship/<manufacturer>/<ship>/<canvas>.json`
- Generated PNG (local): `ships/Data/UI/Generated/ship/<manufacturer>/<ship>/<canvas>.png`
- Reference screenshot (local): `reference/in-game/<ship>/<screenshot>.png`
- SWF assets (P4k): Use `p4k_list` and `p4k_read` MCP tools to browse Data.p4k

### SWF/Canvas Source Location
BuildingBlocks canvases are typically found at:
- P4k: `Data/UI/BuildingBlocks/assets/SWF/Canvas.swf`
- P4k: `Data/UI/BuildingBlocks/` (canvas JSON records)
- Decomposed export: `ships/Data/UI/Generated/ship/<manufacturer>/<ship>/`
- Entity DataCore: Check `UICanvasDecalDescriptorEntityComponentParams.canvas` field

## Expected Canvas Structure by Screen Type

For **target/status screens** (mc_s_target), expect:
- State-bound text widgets (NO TARGET / TARGET_NAME / LOCKED)
- Dashed separator lines (top/bottom)
- Navigation footer bar (<< TARGET_STATUS >>)
- Corner bracket decorations
- State-driven visibility (elements appear/disappear based on state tags)

For **annunciator screens** (h_eng_annunciator), expect:
- State-tagged items (StateModerate, StateCritical, StateFlashing)
- Accent color mapping (Accent2=warning, Accent3=critical)
- Grid layout of indicator items

For **MFD screens** (mfd_screen), expect:
- Complex multi-widget layouts
- Asset/image references
- Dynamic binding to ship state

## Known Pain Points to Watch For

- **State-bound visibility**: Many BuildingBlocks elements use state tags to control
  visibility. If elements are missing, check if the default state matches.
- **Widget tree resolution**: Custom widget types may not be resolved by the parser.
- **Layout engine limitations**: Elements may be parsed but not laid out correctly.
- **Style-tag drift**: Style tags alone should NOT change draw-time behavior.
  Check IR fields for explicit color/tint values.
- **Alpha suppression**: Elements with zero alpha may be invisible even if parsed.
- **Text metric drift**: Font resolution and text bounds must match game rendering.

Required workflow:

Phase A - Baseline and decomposition
- Load both images (reference + latest generated).
- Identify UI regions/components and catalog every difference:
	- extra/missing shapes
	- extra/missing images
	- text differences (content, font, weight, size)
	- positioning/alignment/scale differences
	- color/tint/alpha/blend differences
	- border/stroke/fill differences
- For each difference, assign probable ownership stage:
	- source data resolution
	- bb_layout
	- ui_ir compile/normalization
	- ir_compose draw-time behavior
- For each style/color/alpha/text-bound issue, capture evidence BEFORE editing:
	- MCP tools: `ui_canvas_style_inventory`, `ui_scene_style_probe`, `ui_ir_query`
	- CLI fallback: `cargo run -p starbreaker -- ui debug <canvas_path>`
	- Direct inspection: Read canvas JSON/SWF source files

Phase B - Plan
- Produce a concrete execution plan from the catalog.
- Order items by dependency and regression risk.
- Define success criteria for each item using measurable outcomes (IR/query
	values and rendered results), not only visual opinion.

Phase C - Execute iteratively
- Implement one focused fix at a time.
- After each fix:
	1) run the smallest relevant test/query checks,
	2) regenerate the target artifact,
	3) compare against the same catalog,
	4) update the remaining-differences list.
- Do not keep no-effect code. If MCP diagnostics or rendered artifacts show no
	measurable improvement, remove the experiment before trying the next
	hypothesis.

Phase D - Regression safety (run frequently, not just once)
- Run required UI regression path from ui-matching-workflow.md.
- If any platinum/gold regresses, fix root cause; do not weaken tests.

Phase E - Completion
- Continue until all cataloged differences are resolved.
- Provide final report:
	- resolved differences
	- remaining differences (if any) with proven blocker evidence
	- tests run and outcomes
	- final code cleanup summary (what was removed as no-effect/stale)

Additional analysis expectations:
- Account for perspective/skew when interpreting shape position and size.
- Prefer comparing relative layout relationships (spacing, alignment groups,
	visual hierarchy) rather than absolute raw pixel offsets from imperfect
	screenshots.
- For text, separate typography issues from placement issues.
- Validate inferred style-tag behavior in IR and query outputs before changing
	compose code.

Output requirements from you:
1. Initial difference catalog table.
2. Ordered fix plan.
3. Per-iteration delta log (what changed, what improved, what regressed).
4. Final parity assessment tied to the original catalog.



```

## Usage Notes

- Provide the agent with both image paths explicitly.
- Include the canvas GUID, target name, and render command currently used.
- **Always specify whether MCP tools are available** - if not, the agent should
  immediately use CLI fallback commands instead of wasting iterations.
- If you already know recurring pain points, append a short "watch for" list,
	such as style-tag drift, alpha suppression drift, or text metric drift.
- Keep added context concise; rely on referenced docs for detailed policy.

## Lessons Learned (from Drak Clipper Target Screen)

### What Went Wrong
1. **MCP tools not available**: Prompt assumed `ui_canvas_style_inventory`, `ui_scene_style_probe`,
   and `ui_ir_query` existed. They returned "tool not found" and the agent wasted iterations.
2. **No fallback documented**: No CLI equivalents were provided in the prompt.
3. **Canvas source location unclear**: Agent didn't know where to find `mc_s_target` source data.
4. **No expected structure guidance**: Agent didn't know what elements to expect on a target screen.
5. **Wrong tool for local files**: Agent used MCP `image_preview` and `p4k_read` on local workspace
   files (generated PNG, canvas JSON). These files exist in the decomposed export at
   `ships/Data/UI/Generated/...` and must be read with `read_file`, NOT MCP tools which only
   access P4k contents.

### What Would Have Helped
1. **MCP availability check**: A note at the top saying "MCP tools X, Y, Z are available" or not.
2. **CLI fallback commands**: Documented equivalents for when MCP tools are missing.
3. **Canvas source path**: Direct path to the canvas JSON/SWF in P4k or decomposed export.
4. **Expected element list**: What widgets/elements should appear on this screen type.
5. **State tag awareness**: Knowledge that many elements use state-bound visibility.
6. **Local vs P4k file distinction**: Explicitly state which files are local workspace files
   (read with `read_file`) vs P4k-native assets (use MCP tools). The generated PNG, reference
   screenshot, and decomposed canvas JSON are ALL local files — MCP tools will fail on them.

## Per-Task Findings

For each matching task, record Phase A findings in a separate file at:
`docs/ui-matching-tasks/<canvas-name>-findings.md`

Example: `docs/ui-matching-tasks/target-master-findings.md`

This keeps the template generic while capturing task-specific investigation data.
