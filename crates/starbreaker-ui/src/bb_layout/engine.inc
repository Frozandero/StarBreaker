// Split include hub: implementation lives in engine_parts/engine_NN.part
// chunks (cap 3000 lines, target <=2500 - crates/starbreaker-ui/docs/ui-workflow.md rule 5).
// Keep chunk ordering stable: later items can depend on earlier ones.

include!("engine_parts/engine_01.part");
include!("engine_parts/engine_02.part");
include!("engine_parts/engine_03.part");
