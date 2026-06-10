//! Tests for the numeric binding ops used by the power-screen pip sizing
//! chains (`gen_mc_s_powerlistitem`): `NumberFromBoolean` branch selection,
//! `BooleanFromNumber` comparisons, `NumberRound` (decimal places),
//! `NumberFromIntegerSwitch`, and `NumberArithmatic` with an absent LHS
//! falling back to `amount` (the engine's `Div(amount=1, inputB=x)` = `1/x`).

use crate::canvas::Value;
use crate::defaults::DefaultValueRegistry;

use super::BindingResolver;

fn resolver_for(ops: serde_json::Value) -> BindingResolver {
    let ops_vec: Vec<serde_json::Value> = ops.as_array().expect("ops array").clone();
    BindingResolver::from_operations(&ops_vec)
}

/// `NumberFromBoolean` selects the `inputTrue`/`inputFalse` branch POINTER
/// when wired, with the `isTrue`/`isFalse` literals as fallbacks; the boolean
/// input compares a number against the authored literal (`Greater 15`, the
/// pip width/height fallback threshold).
#[test]
fn number_from_boolean_selects_branch_by_number_comparison() {
    let ops = serde_json::json!([
        {"_Type_": "BuildingBlocks_BindingsNumberField",
         "widget": "_PointsTo_:ptr:1", "field": "SizeX", "input": "_PointsTo_:ptr:10"},
        {"_Pointer_": "ptr:10", "_Type_": "BuildingBlocks_BindingsNumberFromBoolean",
         "isTrue": 0.5, "isFalse": 1.0, "inputTrue": null, "inputFalse": null,
         "input": "_PointsTo_:ptr:11"},
        {"_Pointer_": "ptr:11", "_Type_": "BuildingBlocks_BindingsBooleanFromNumber",
         "type": "Greater", "number": 15.0, "input": "_PointsTo_:ptr:12", "inputB": null},
        {"_Pointer_": "ptr:12", "_Type_": "BuildingBlocks_BindingsNumberVariable",
         "path": [], "binding": "count", "inheritsNamespace": true}
    ]);
    let resolver = resolver_for(ops);

    let mut defaults = DefaultValueRegistry::default();
    defaults.insert_path("count", Value::Float(3.0));
    assert_eq!(
        resolver.resolve_field_number(1, "SizeX", &defaults),
        Some(1.0),
        "3 > 15 is false: the isFalse literal"
    );

    let mut defaults = DefaultValueRegistry::default();
    defaults.insert_path("count", Value::Float(16.0));
    assert_eq!(
        resolver.resolve_field_number(1, "SizeX", &defaults),
        Some(0.5),
        "16 > 15 is true: the isTrue literal"
    );
}

/// `NumberRound` rounds to `amount` DECIMAL PLACES, and a `NumberArithmatic`
/// `Div` with no `input` uses `amount` as its left operand: the pip slot
/// height `round3(1 / MaxPipList)`.
#[test]
fn number_round_and_div_amount_lhs() {
    let ops = serde_json::json!([
        {"_Type_": "BuildingBlocks_BindingsNumberField",
         "widget": "_PointsTo_:ptr:1", "field": "SizeY", "input": "_PointsTo_:ptr:10"},
        {"_Pointer_": "ptr:10", "_Type_": "BuildingBlocks_BindingsNumberRound",
         "amount": 3, "input": "_PointsTo_:ptr:11"},
        {"_Pointer_": "ptr:11", "_Type_": "BuildingBlocks_BindingsNumberArithmatic",
         "type": "Div", "amount": 1.0, "input": null, "inputB": "_PointsTo_:ptr:12"},
        {"_Pointer_": "ptr:12", "_Type_": "BuildingBlocks_BindingsNumberVariable",
         "path": [], "binding": "max", "inheritsNamespace": true}
    ]);
    let resolver = resolver_for(ops);
    let mut defaults = DefaultValueRegistry::default();
    defaults.insert_path("max", Value::Float(6.0));
    let value = resolver
        .resolve_field_number(1, "SizeY", &defaults)
        .expect("resolves");
    assert!((value - 0.167).abs() < 1e-9, "round3(1/6) = 0.167, got {value}");
}

/// `NumberFromIntegerSwitch` maps integer cases to numbers with a default:
/// `switch(pipState; 5 → 1.0; default 0)` forces the PreAssigned pip's fill.
#[test]
fn number_from_integer_switch_maps_cases() {
    let ops = serde_json::json!([
        {"_Type_": "BuildingBlocks_BindingsNumberField",
         "widget": "_PointsTo_:ptr:1", "field": "SizeX", "input": "_PointsTo_:ptr:10"},
        {"_Pointer_": "ptr:10", "_Type_": "BuildingBlocks_BindingsNumberArithmatic",
         "type": "Max", "amount": 1.0, "input": "_PointsTo_:ptr:11", "inputB": "_PointsTo_:ptr:12"},
        {"_Pointer_": "ptr:11", "_Type_": "BuildingBlocks_BindingsNumberVariable",
         "path": [], "binding": "pipamount", "inheritsNamespace": true},
        {"_Pointer_": "ptr:12", "_Type_": "BuildingBlocks_BindingsNumberFromIntegerSwitch",
         "defaultValue": 0.0, "defaultOverride": null, "input": "_PointsTo_:ptr:13",
         "values": [{"_Type_": "BuildingBlocks_IntegerNumberPair", "first": 5, "second": 1.0}]},
        {"_Pointer_": "ptr:13", "_Type_": "BuildingBlocks_BindingsIntegerVariable",
         "path": [], "binding": "pipstate", "inheritsNamespace": true}
    ]);
    let resolver = resolver_for(ops);

    let mut defaults = DefaultValueRegistry::default();
    defaults.insert_path("pipamount", Value::Float(0.3));
    defaults.insert_path("pipstate", Value::Int(5));
    assert_eq!(
        resolver.resolve_field_number(1, "SizeX", &defaults),
        Some(1.0),
        "state 5 (PreAssigned) forces full fill via Max(0.3, 1.0)"
    );

    let mut defaults = DefaultValueRegistry::default();
    defaults.insert_path("pipamount", Value::Float(0.3));
    defaults.insert_path("pipstate", Value::Int(1));
    assert_eq!(
        resolver.resolve_field_number(1, "SizeX", &defaults),
        Some(0.3),
        "other states fall to the switch default 0: Max(0.3, 0.0)"
    );
}

/// The full authored pip-slot SizeY chain: `pipLinkAmount > 1 ? slot×links :
/// slot` where `slot = MaxPipList > 15 ? PipHeight : round3(1/MaxPipList)` —
/// with the engines column's MaxPipList = 6 every slot is 1/6 of the column.
#[test]
fn pip_slot_size_y_chain_resolves_one_sixth() {
    let ops = serde_json::json!([
        {"_Type_": "BuildingBlocks_BindingsNumberField",
         "widget": "_PointsTo_:ptr:2", "field": "SizeY", "input": "_PointsTo_:ptr:53"},
        {"_Pointer_": "ptr:53", "_Type_": "BuildingBlocks_BindingsNumberFromBoolean",
         "isTrue": 0.0, "isFalse": 1.0,
         "inputTrue": "_PointsTo_:ptr:75", "inputFalse": "_PointsTo_:ptr:76",
         "input": "_PointsTo_:ptr:77"},
        {"_Pointer_": "ptr:77", "_Type_": "BuildingBlocks_BindingsBooleanFromInteger",
         "type": "Greater", "inputL": "_PointsTo_:ptr:78", "inputR": null, "value": 1},
        {"_Pointer_": "ptr:78", "_Type_": "BuildingBlocks_BindingsIntegerVariable",
         "path": [], "binding": "piplinkamount", "inheritsNamespace": true},
        {"_Pointer_": "ptr:75", "_Type_": "BuildingBlocks_BindingsNumberArithmatic",
         "type": "Mul", "amount": 1.0, "input": "_PointsTo_:ptr:76", "inputB": "_PointsTo_:ptr:79"},
        {"_Pointer_": "ptr:79", "_Type_": "BuildingBlocks_BindingsNumberFromInteger",
         "asSeconds": false, "input": "_PointsTo_:ptr:78"},
        {"_Pointer_": "ptr:76", "_Type_": "BuildingBlocks_BindingsNumberFromBoolean",
         "isTrue": 0.0, "isFalse": 1.0,
         "inputTrue": "_PointsTo_:ptr:56", "inputFalse": "_PointsTo_:ptr:57",
         "input": "_PointsTo_:ptr:58"},
        {"_Pointer_": "ptr:58", "_Type_": "BuildingBlocks_BindingsBooleanFromNumber",
         "type": "Greater", "number": 15.0, "input": "_PointsTo_:ptr:55", "inputB": null},
        {"_Pointer_": "ptr:55", "_Type_": "BuildingBlocks_BindingsNumberFromInteger",
         "asSeconds": false, "input": "_PointsTo_:ptr:54"},
        {"_Pointer_": "ptr:54", "_Type_": "BuildingBlocks_BindingsIntegerVariable",
         "path": [], "binding": "maxpiplist", "inheritsNamespace": true},
        {"_Pointer_": "ptr:56", "_Type_": "BuildingBlocks_BindingsNumberComponentParameter",
         "name": "Pip height", "parameter": "ParamInput1", "defaultValue": 0.06},
        {"_Pointer_": "ptr:57", "_Type_": "BuildingBlocks_BindingsNumberRound",
         "amount": 3, "input": "_PointsTo_:ptr:82"},
        {"_Pointer_": "ptr:82", "_Type_": "BuildingBlocks_BindingsNumberArithmatic",
         "type": "Div", "amount": 1.0, "input": null, "inputB": "_PointsTo_:ptr:62"},
        {"_Pointer_": "ptr:62", "_Type_": "BuildingBlocks_BindingsNumberFromInteger",
         "asSeconds": false, "input": "_PointsTo_:ptr:54"}
    ]);
    let resolver = resolver_for(ops);
    let mut defaults = DefaultValueRegistry::default();
    defaults.insert_path("maxpiplist", Value::Int(6));
    defaults.insert_path("piplinkamount", Value::Int(1));
    let value = resolver
        .resolve_field_number(2, "SizeY", &defaults)
        .expect("resolves");
    assert!((value - 0.167).abs() < 1e-9, "slot height = round3(1/6), got {value}");
}
