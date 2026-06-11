# default_value_registry_v1.json — provenance notes

Companion to `default_value_registry_v1.json` (JSON carries no comments).
One section per pinned path family: what it is, where the value came from,
and the sunset condition (usually "move to derivation in
`crates/starbreaker-3d/src/ui_pipeline/ship_values.rs`"). Update this file
in the SAME commit as any registry change
(docs/ui-process-improvements.md item 10). Per-ship derived values do NOT
appear in the registry — they flow through
`PipelineInputs::derived_values` at export/replay and shadow these paths
(probe with `SB_SHIP_VALUES_DUMP=1`).

| Path family | Provenance | Sunset |
|---|---|---|
| `/vehicle/targetname`, `/vehicle/target/*`, `/targetselector/*`, `/commscontroller/canhailtarget`, `/vehicle/gungroup` | At-rest engine state for the target screen (NO TARGET pose), reference-verified vs `Screen_Right_Upper_RTT.png` | Keep — genuine at-rest defaults (no target locked at spawn) |
| `/ship/hp/current`, `/ship/hp/max`, `/seatdashboard/powerstate`, `/seatdashboard/powercurrent`, `/seatdashboard/powermax` | At-rest dashboard values, reference-verified | `powercurrent/powermax` mirror OUTPUT 2/16 — sunset together with the totalPossiblePower derivation below |
| `/AnnunciatorProvider/Issues/[000i]/Severity` | Annunciator at-rest issue severities, reference-verified vs `Screen_Annunciator_L.png` (gold). RE-ADJUDICATED 2026-06-11 with the entry-driven chiclet TEXT colours as the discriminator (the original pinning could only see borders/fills, which don't discriminate): items 0/2/3 = 0 (no state — reference shows plain Base-orange labels), item 1 (WPN) = 1 (Moderate — amber block, dark Disabled text), [0008] = 5 (Offline — the LEFT master wires its 5th chiclet to issue 8, NOT 4; grey 143 'Off - Text'). [0004]=5 kept for the other consumers wired to issue 4 | Keep until annunciator issue derivation exists |
| `CloneLocationInfo/*`, `Bed/MedBed/*`, `bed/playerinfo/name`, `state.BaseScreens.*` (+ `Bed/`-prefixed variants) | Medical at-rest state, platinum-verified. KEYS ARE PRE-COMPOSITION: medical canvases author bindings (partially) pre-qualified and relative `urlPostfix` namespaces are deliberately NOT composed — see `docs/ui-workflow.md` §10 | Migrate keys to fully-qualified namespaces IF relative urlPostfix composition ever lands (requires medical platinum re-freeze) |
| `EnableBackground`, `isAlignedRight` | Engine booleans, at-rest true for the frozen screens | Keep |
| `piplist`, `piplist/[000i]/*` (icons, totals, pipstates, selectedindex, ispoweredoff), `resourcenetworkui/powermanagement/pipsLengthMax` | Power-screen at-rest profile. NOW SHADOWED by derivation (`ship_values.rs` derives all of these per ship); registry copies remain as the documented fallback for bare replays without ship data | Remove once every render path guarantees derived values (verify with `SB_SHIP_VALUES_DUMP=1` + a registry-less test render) |
| `piplist/[000i]/tempindicator/*` | Shadowed by derivation (per-item overheat + reference-calibrated `overheat + 68 K` ceiling; ambient 290 K, min 273 K) | Same as above. The 68 K headroom is reference-calibrated — decode the engine's real gauge-scale rule to sunset |
| `/resourcenetworkUi/powermanagement/totalPossiblePower` = 16, `availablePower` = 2 | REFERENCE-PINNED (in-game `Screen_Left_Lower_RTT.png` shows "2 / 16"); the derivation formula is NOT decoded (plants generate 2×14 = 28 ≠ 16) | Derive in `ship_values.rs` once the resource-network output formula is decoded |
| `iscast` = false | Engine-pushed per render-target type: data authors the editor-pose `defaultValue: true`, the engine wires FALSE for screen render targets (casts are the small HUD projections under `casts/`). Boolean ComponentParameters consult the registry BY NAME before the editor default | Keep — this IS the engine value for screens; a cast render path would override per-instance |
| `item_type*` localization entries (`localization` section) | UI type labels from game localization | Keep (static localization mirror) |

Emissions signature values (`vehicle/signaturesystem/signatures/[000i]/...`)
are NOT in the registry — they are reference-pinned in `ship_values.rs`
(IR 3500/294.1, EM 14900/0, CS 18600/0) with a TODO to derive from the
vehicle's signature components.
