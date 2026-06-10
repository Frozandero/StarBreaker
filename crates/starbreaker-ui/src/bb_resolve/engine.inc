// Split include hub: this file stays under the line cap; implementation lives in engine_parts/*.inc.
// Keep part ordering stable because later parts can depend on earlier items.

include!("engine_parts/part_01.part");
include!("engine_parts/part_02.part");
include!("engine_parts/part_03.part");
include!("engine_parts/pass1.part");
include!("engine_parts/part_04.part");
include!("engine_parts/part_05.part");
include!("engine_parts/part_06.part");
include!("engine_parts/param_targets.part");
include!("engine_parts/part_07.part");
include!("engine_parts/part_08.part");
include!("engine_parts/part_09.part");
include!("engine_parts/part_10.part");
include!("engine_parts/part_11.part");
include!("engine_parts/part_12.part");
include!("engine_parts/part_13.part");
include!("engine_parts/list_binding.part");
include!("engine_parts/list_binding_tests.part");
include!("engine_parts/array_list.part");
include!("engine_parts/registry_gates.part");
include!("engine_parts/array_list_tests.part");
include!("engine_parts/param_relay_tests.part");
include!("engine_parts/param_leak_tests.part");
include!("engine_parts/widget_standard_expansion.part");
include!("engine_parts/scrollbar_expansion.part");
include!("engine_parts/widget_standard_expansion_tests.part");
