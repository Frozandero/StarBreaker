# Clipper UI parity — open items (handoff)

> Slimmed 2026-06-13. The landed-work NARRATIVE (the per-step prose and the
> commit table) was removed — that knowledge now lives in the code,
> `crates/starbreaker-ui/docs/ui-fallback-register.md` (retired
> constants/fallbacks), `crates/starbreaker-ui/docs/ui-architecture-runbook.md`
> (engine models + the "Open architecture debt" section), and the git log.
> What remains is the **open Clipper parity work** plus a one-line provenance
> index so existing `catalog #N` / `§N` citations still resolve. Process +
> commands: `crates/starbreaker-ui/docs/ui-workflow.md` +
> `crates/starbreaker-ui/docs/ui-reference.md`; the per-screen dossier
> (`ui-reference.md` §3) maps each screen to its scene/canvas/preset. Fresh
> sessions: instantiate `crates/starbreaker-ui/docs/ui-matching-agent-prompt.md`
> with `SCREEN=Screen_Left_Lower_RTT`, `HANDOFF=` this file.

Reference images: `~/projects/scorg_tools/reference/in-game/Clipper/`
(`Screen_Left_Lower_RTT.png` power, `Screen_Right_Upper_RTT.png` target,
`screen_16x9_a-[medical1].png`, `mesh_end_screen_plane-[medical2].png`). The
red/white pip outlines in the power reference are mouse-hover artifacts —
ignore (owner's rule); captures are imperfect (skew/bloom) — compare
structurally (workflow §4).

## Diff catalog (power screen vs reference) — status

| # | Region | Difference | Status |
|---|---|---|---|
| 1 / 1b | Emissions | header collapse + emitted/ambient overlap | **FIXED** (SizeY behaviour + column zero-Auto text intrinsics/shrink) |
| 2 | Emissions | IR/EM/CS labels rendered `@LOC_PLACEHOLDER` | **FIXED** (clone FieldModifierLocalization → `_SynthLocalizedWidget_`) |
| 3 | OUTPUT card | title flow (icon→dots→title left-aligned) | **FIXED** (draw-metrics intrinsic measure, `pipeline/text_measure.rs`; measure == draw) |
| 4 | Battery card | OFFLINE text overflowed card | **FIXED** (draw-width box + shrink) |
| 5 | Cards/pips/gauges | icon/colour tints, pip brightness, "2" white | **MOSTLY LANDED**; open residuals below (P4, P13, IR/EM/CS) |
| 6 | Footer/scrollbar | faint track + backdrop band | **OPEN** (A7 class, below) |

## 2026-06-13 — user-symptom triage (Screen_Left_Lower_RTT)

Four reported symptoms, mapped to root cause (one landed, three already-open):

1. **Pip gauge "clipped right / container ~30px wider / slider wider"** —
   container width is **FAITHFUL**: `canvas_PowerSystems` is authored `0.7`
   Percent and its parent `base_PowerAssignmentListContainer` flexes Row with
   `axisJustification: "Center"`, so the 0.3+0.7 children centre with ~15px each
   side (the "30px"). The col-3 heat bar (authored right 1528) is fade-clipped by
   the `Scrollview` (overflow Clip, right 1499) — engine-authored list behaviour;
   its fill still renders to x≈1512. The reference's right extent measures to
   x≈1580, *past* the screen edge (1559) = capture bloom, not a real gauge. Do
   NOT widen/rejustify (would hack authored data). The "slider wider" half = **P7**,
   now **FAITHFUL**: the thumb formula (viewport/content × track) is already correct
   in `apply_scroll_thumb_rects`; 393px matches the DataCore-derived 7-column
   content; the reference 432 is bloom on the orange slider. No fix.
2. **Header side bars red + further out** — POSITION **LANDED** (SpaceBetween,
   see P13); COLOUR open = shared/brand **cascade** (Sep2/3 already Accent1; Sep1/4
   wrongly Base), NOT overlay-default — see P13.
3. **Output-card dotted separator** + **4c battery dotted separator** — empty
   `BuildingBlocks_WidgetSeparator` hosts (power titles id=604/607, emissions
   Sep*) with `asset_layout` but no `custom_shape`/`asset_ref`/children — need the
   widget-standard expansion = **P3** (parked; ID-band + brand blockers).
4. **Battery card:** (a) OFFLINE too low — `base_ValuesContainer` carries a
   non-zero `0.9 Auto` height hint → 141.9px box over 71px of "0/0" content,
   leaving the ~71px gap above OFFLINE; non-zero column Auto hints are **pinned by
   medical baselines** (workflow §10 don't-retry). (b) BATTERY overflows the card
   ~11px — title `text_BatteryTitle` is `Auto`-width (intrinsic), no shrink, in an
   authored `Start` row (`base_OutputHeader`) — all faithful. Measurement: OUTPUT
   cap-height matches the reference exactly (84=84 → font SIZE correct) and a
   fixed-geometry pip slab matches (199 vs 202 → rectification accurate), but text
   WIDTH measurements contradict (OUTPUT render-wider, footer ref-wider) = within
   bloom noise, no reliable systematic width error. So no clean battery-local fix;
   the ~11px is most likely minor letter-spacing (**P8** family, gated on a clean
   reference measurement the bloomed dim labels can't give) or a §6 known-outlier.
   (c) = item 3.

## Open items

### P3 — separator dots (power cards) — PARKED, full diagnosis (~30min to land)

The dotted icon/title separators are `BuildingBlocks_WidgetSeparator` widgets
(direction Vertical, style Tertiary on the power cards) — a modularkit
standard family: 6 records `modularkit/standard/widgets/{vertical,horizontal}
separator{primary,secondary,tertiary}widgetstandard.json`, each a single
ComponentRoot WidgetCustomShape whose PER-BRAND container authors the visual
(drak env: `DRAK_S42_seperator_vertical_2.svg`, EnableColorOverlay=false,
nine-slice). A working implementation was built and REVERTED for two blockers
(both are now architecture debt in the runbook):

1. **bb_scene**: add `BbNodeType::WidgetSeparator` (+3 exhaustive matches:
   bb_layout `type_name_str`, ui_ir `node_type_name` "widget_separator",
   compose `draw_node` no-op host arm) and convert the 3 existing
   `Other("BuildingBlocks_WidgetSeparator")` matches in ui_ir engine_02.part;
   add `"Separator"` to `node_type_matches`.
2. **Expansion** (bb_resolve engine_04 `expand_widget_standards`): include
   WidgetSeparator hosts, template by direction/style, no params. **BLOCKER A
   → runbook "per-host-type ID-band lanes":** instances consume the shared
   `0xF000_0000` band counter and SHIFT frozen platinum instance ids (the
   medical close-button X 4026531855 became a separator).
3. **Brand application** (`apply_separator_standard_styles`, engine_01.part,
   subtree-scoped via `apply_scene_style_entries_in_subtree`): **BLOCKER B →
   runbook "one brand-context resolver":** exact canvas-selected id + hud↔env
   sibling works for power (`s_drak_hud`→`s_drak_env`) and the medical bed,
   but the medical FOOTER selects `s_aegs_env` and matches the standard's
   `s_aegs_env` ⇒ AEGS divider leaks into platinum. Needs the typography-table
   model (`selected_style_name`: canvas:`<style-link>` else `s_<mfr>_{hud|env}`
   by canvas family) instead of `resolve_brand_style` on the component record.
4. The overlay-icon default must respect a styled `EnableColorOverlay=false`
   (PascalCase raw override) so the separator SVGs are not
   MissionObjectives-tinted.
5. Test fixtures need the tag database served (expansion bails without it).

### P4 — pip slab brightness

Pip slabs render saturated orange vs pale/washed in the reference. Likely the
pip fill colour role (Bright vs Base) combined with capture bloom — investigate
the IR tint token against a SAME-capture anchor (workflow §4 photometric
method) before changing a role; do not chase the bloom.

### P13 — header side bars (emissions Sep1/Sep4)

Two parts. **POSITION — LANDED 2026-06-13:** the bars sit at the emissions
header edges via authored `axisJustification: "SpaceBetween"` on
`base_EmissionsContainer` (GEN_MC_S_Emissions). `bb_layout` did not implement
the space-distributing modes — `SpaceBetween`/`SpaceAround`/`SpaceEvenly` fell
through the `main_offset` match to `_ => 0.0` (Start), left-packing the row, so
Sep4 sat at x≈1425 with ~104px of dead space on the right. Implemented the three
space modes in `bb_layout/engine_parts/engine_01.part` (slack = avail−total_main,
shared as equal gaps; test `flex_row_space_between_spreads_children_to_container_edges`).
The IR/EM/CS groups now spread and Sep4 reaches the content-right edge (x≈1510,
inside the container's ~17px padding) — matches the reference. Clean across all
three suites (lib + IR-snapshot + whole-image visual after re-export); no frozen
target uses a slack-bearing space mode, so nothing shifted.

**COLOUR — open, = brand-context-resolver (Blocker B), fully mapped 2026-06-13:**
the only VISIBLE header bars are Sep1 (x≈88) and Sep4 (x≈1510), both solid-fill
custom shapes (`render_shape:true`, empty `svg_path`) with token `Base` (orange).
Sep2/Sep3 carry token `Accent1` but render **zero-size** (w=0,h=0) — not visible.
Zoomed rectified-reference crops confirm the visible edge bars are **red** (salmon,
≈Accent1) and sit ~30px further out (x≈1540) than ours. So the bug is real.
Root cause: `BB_A3_STYLE_PROBE` (full) shows Sep1/Sep4 match exactly ONE entry —
the shared `mfd_g_emissions` **"New Style"** (`ConditionParent` → `AnyOfTag{80c85b19
(sep1),21cf313d(sep4)}`, modifier `BackgroundColor=Base@1.0`, file line ~177) — and
the brand pass `s_drak_hud` provides NO separator colour/visibility override (pass
order is correct: shared `mfd_g_emissions` BEFORE brand `s_drak_hud`, so a brand
entry *would* win — there just isn't one). The Accent1/visibility entries
("Vertical Separators Color" Accent1 on sep1/2/3; "Vertical Separator 1/4" IsActive
toggles; "Separator N" SizeX=2) live in `brandStyles[2]/[4]` — a brand index the
Clipper's resolver does NOT select. So the fix is the **brand-context resolver**
(select the right brand so its separator entries surface), NOT a scoped hack: a
blanket `primary→Accent1` fill rule would wrongly recolour the card greebles
(`card_BackgroundGreeble`, also `primary`+`Base` custom shapes) red. Same resolver
unblocks the dotted separators (P3 Blocker B). Touches the GOLD
`clipper_target_master` emissions header → audited gold re-freeze. The overlay
default (`custom_shape_overlay_icon_default`) is NOT involved (needs an SVG source
these bars lack).

### P7 — scrollbar slider width

Orange scrollbar slider measures ~393px vs reference ~432px. The driver is the
engine `_SizeRatio` input (CLIK thumb formula = viewport/content; plan P2.2b);
the slider-width math suggested ~0.7×padding-box but the card-width / scroll
position evidence is capture-skew-limited. Defer until the `_SizeRatio` source
is decoded.

### P8 — footer letter-spacing

Footer letter pitch is ~6% off (long-standing) — a global SWF `LetterSpacing`
model question, not a power-screen-local fix. Defer (documented).

### Close-button outset border (medical bed; target side-buttons in blast radius)

The frame node (`ComponentRoot`, authored 64×64, border 3px, radius 6) draws
its border OUTSET in-game (content-box: 64 + 2×3 = 70 visible; the reference
measures 68–70 at the aligned right edge); ours draws inset (64 visible).
**A w/h known-outlier is the WRONG vehicle** — outset borders are paint-only,
so the snapshot rect stays 64 and would never graduate. Implementation = outset
border drawing scoped to expanded-standard component roots (own
`canvas-proxy-root` tag); blast radius includes the GOLD target master's two
212×105.6 side buttons (3.33px borders) — measure those frames on the target
reference before landing, then artifact-adjudicate every target.

### IR/EM/CS abbreviation colour

In-game the IR/EM/CS abbreviations keep the H1 deep orange; ours get recoloured
to slot-0 Base by the shared `mfd_g_emissions` `Icon Color` entry (a normal
match). Evidence (incl. the footer `SelectedName` comment in `conditions.rs`)
suggests shared-record colour entries do NOT restyle textfield text — this
needs its own scoped rule (shared-tier colour entries skip text-format
restyling) plus a battery adjudication.

### A7 — backdrop band class

A faint footer track + backdrop band remain on the power footer and the target
status area. Partially resolved by the audited gold re-freeze (the
`clipper_target_master` `40:widget_custom_shape` SizeY 0.18 Percent → 194.4
band, alpha 0.1); the remaining A7 backdrop stack (faint track + band visible
pixels) is the open residual. Target dossier tracks it as "A7 backdrop stack
remainder".

### Linear-light compositing — GATED on owner approval

The glow renders darker than the reference because the engine composites in
LINEAR light and we blend in sRGB. The white-mask glow path is already
converted (landed, scoped: `blit_white_mask_overlay_linear`); the
**renderer-wide** migration is the gated item — full detail, predicted
numbers, and the "do not partially apply" rule are in the runbook's **Open
architecture debt** section.

### Minor — OFFLINE cap height

The battery OFFLINE glyph cap measures ~43px in the reference vs ~49–53
predicted (band-fit?) — unexplained; recorded in the measurement bank
(`crates/starbreaker-ui/tests/fixtures/ui_ir/reference_measurements_v1.notes.md`).

## Landed — provenance index

One line per landed step (full narrative in git; engine models in the runbook,
constant/fallback retirements in `ui-fallback-register.md`). Kept so the
`catalog #N` and `§N` citations elsewhere in the repo still resolve.

| Ref | What landed | Commit(s) |
|---|---|---|
| catalog #1/1b | emissions header collapse + emitted/ambient stacking | (Step 6 + flex shrink) |
| catalog #2 | IR/EM/CS labels via clone FieldModifierLocalization | (clone_expand) |
| catalog #3/#4 | draw-metrics intrinsic measure (measure == draw); OUTPUT/OFFLINE | (`pipeline/text_measure.rs`) |
| §5 | tint/defaultStyles cascade re-land; token-only fills | 79c87e8ea |
| §6 | medical: white X, Bioticorp logo −12px, position outliers registered | 4f8532f4e |
| §9 | annunciator rounds 2–3 + MFD body backplate | 81c93109e, f436b446f |
| §10 | annunciator round 4 — near-black bg + white-mask Base overlay | 16d595dbb |
| §11 | annunciator round 5 — WPN glow off + linear mask glow | f5040aeab |
| §12 | power review round 2 — P1 "2" cream, P2/P5 MissionObjectives icons | (P1 + next) |
| §13 | power review round 3 — per-axis Overflow clip; text-format route (open: OFFLINE cap 43) | a30761f20, 07c821a83 |
| §15 | text-format route landed + hard-coding remediation | 07c821a83, 4803d3c48 |
| §16 | MFD text size — host-path `imageSizePercent` division | 15d1e3b99 |
| P13-pos | header side bars reach edges — `bb_layout` SpaceBetween/Around/Evenly (were unimplemented → Start) | (2026-06-13) |
