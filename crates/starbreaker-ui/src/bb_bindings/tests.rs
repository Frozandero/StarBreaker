use serde_json::json;

use super::*;
use crate::canvas::Value;
use crate::defaults::DefaultValueRegistry;

fn resolver() -> BindingResolver {
    BindingResolver {
        widget_to_path: Default::default(),
        widget_to_loc_key: Default::default(),
        widget_to_input_ptrs: Default::default(),
        widget_field_to_input_ptrs: Default::default(),
        field_name_to_input_ptrs: Default::default(),
        ptr_to_op: Default::default(),
        ptr_to_path: Default::default(),
        widget_to_string: Default::default(),
    }
}

    #[test]
    fn text_at_key_resolves_via_loc_map() {
        let resolver = resolver();
        let mut defaults = DefaultValueRegistry::default();
        defaults.merge_localization([("foo".to_string(), "POWER MANAGEMENT".to_string())].into());

        let raw = json!({"text": "@foo"});
        let result = resolver.resolve_text_detailed(0, &raw, &defaults);
        assert_eq!(result.text, "POWER MANAGEMENT");
    }

    #[test]
    fn text_literal_returned_as_is() {
        let resolver = resolver();
        let defaults = DefaultValueRegistry::default();

        let raw = json!({"text": "Hello World"});
        let result = resolver.resolve_text_detailed(0, &raw, &defaults);
        assert_eq!(result.text, "Hello World");
    }

    #[test]
    fn loc_string_field_resolved() {
        let resolver = resolver();
        let mut defaults = DefaultValueRegistry::default();
        defaults.merge_localization([("mykey".to_string(), "My Label".to_string())].into());

        // `locString` field carries the loc key; `text` is absent.
        let raw = json!({"locString": "@mykey"});
        let result = resolver.resolve_text_detailed(0, &raw, &defaults);
        assert_eq!(result.text, "My Label");
    }

    #[test]
    fn loc_string_field_respects_case_modifier() {
        let resolver = resolver();
        let mut defaults = DefaultValueRegistry::default();
        defaults.merge_localization([("mykey".to_string(), "My Label".to_string())].into());

        let raw = json!({
            "locString": "@mykey",
            "labelProperties": {
                "caseModifier": "Upper"
            }
        });
        let result = resolver.resolve_text_detailed(0, &raw, &defaults);
        assert_eq!(result.text, "MY LABEL");
    }

    #[test]
    fn top_level_case_modifier_applies_to_loc_string_field() {
        let resolver = resolver();
        let mut defaults = DefaultValueRegistry::default();
        defaults.merge_localization([("mykey".to_string(), "My Label".to_string())].into());

        let raw = json!({
            "locString": "@mykey",
            "caseModifier": "Upper"
        });
        let result = resolver.resolve_text_detailed(0, &raw, &defaults);
        assert_eq!(result.text, "MY LABEL");
    }

    #[test]
    fn label_properties_case_modifier_upper_applied() {
        let resolver = resolver();
        let mut defaults = DefaultValueRegistry::default();
        defaults.merge_localization([("info_kiosks_logoscreen_001".to_string(), "Touch to start".to_string())].into());

        let raw = json!({
            "labelProperties": {
                "label": "@Info_Kiosks_LogoScreen_001",
                "caseModifier": "Upper"
            }
        });
        let result = resolver.resolve_text_detailed(0, &raw, &defaults);
        assert_eq!(result.text, "TOUCH TO START");
    }

    #[test]
    fn loc_empty_sentinel_skipped() {
        let resolver = resolver();
        let defaults = DefaultValueRegistry::default();

        // @LOC_EMPTY resolves to "" (suppressed sentinel) — must not emit that.
        let raw = json!({"locString": "@LOC_EMPTY"});
        let result = resolver.resolve_text_detailed(0, &raw, &defaults);
        assert_eq!(result.text, "");
    }

    /// Build a resolver where node 5 is a component-parameter-driven label: it has
    /// a `ComponentLabelProperties` with a placeholder `label`, plus a
    /// `LocalizedField → ParamInput0` op whose `LocalizedComponentParameter`
    /// resolves to `param_default`. `node_raw` carries the placeholder label.
    fn param_label_resolver(param_default: &str) -> (BindingResolver, serde_json::Value) {
        let mut r = resolver();
        // The LocalizedComponentParameter op (the field's runtime content source).
        r.ptr_to_op.insert(
            70,
            json!({
                "_Pointer_": "ptr:70",
                "_Type_": "BuildingBlocks_BindingsLocalizedComponentParameter",
                "parameter": "ParamInput0",
                "defaultValue": param_default
            }),
        );
        r.widget_field_to_input_ptrs.insert((5, "ParamInput0".to_string()), vec![70]);
        r.widget_to_input_ptrs.insert(5, vec![70]);
        let raw = json!({
            "labelProperties": {
                "_Type_": "BuildingBlocks_ComponentLabelProperties",
                "label": "@placeholder_loadout",
                "caseModifier": "Upper"
            }
        });
        (r, raw)
    }

    /// An inactive component-parameter-driven label (its param resolves to
    /// `@LOC_PLACEHOLDER`) renders EMPTY — the authored `labelProperties.label`
    /// placeholder must NOT leak (the footer "LOADOUT" bug).
    #[test]
    fn component_param_label_with_empty_param_resolves_empty() {
        let (r, raw) = param_label_resolver("@LOC_PLACEHOLDER");
        let mut defaults = DefaultValueRegistry::default();
        // Prove the placeholder would otherwise resolve to visible text.
        defaults.merge_localization([("placeholder_loadout".to_string(), "LOADOUT".to_string())].into());
        let result = r.resolve_text_detailed(5, &raw, &defaults);
        assert_eq!(result.text, "", "empty param ⇒ empty field, not the placeholder label");
    }

    /// An active component-parameter-driven label shows the PARAM value (with the
    /// authored case modifier), not the placeholder label.
    #[test]
    fn component_param_label_with_content_uses_param_and_case_modifier() {
        let (r, raw) = param_label_resolver("@screen_name");
        let mut defaults = DefaultValueRegistry::default();
        defaults.merge_localization([
            ("screen_name".to_string(), "Target Status".to_string()),
            ("placeholder_loadout".to_string(), "LOADOUT".to_string()),
        ].into());
        let result = r.resolve_text_detailed(5, &raw, &defaults);
        assert_eq!(result.text, "TARGET STATUS", "param value wins, upper-cased; placeholder ignored");
    }

    /// A static label (no `ParamInput0` binding) still uses `labelProperties.label`.
    #[test]
    fn static_label_without_param_binding_uses_label() {
        let r = resolver();
        let mut defaults = DefaultValueRegistry::default();
        defaults.merge_localization([("hud_notarget".to_string(), "NO TARGET".to_string())].into());
        let raw = json!({
            "labelProperties": {
                "_Type_": "BuildingBlocks_ComponentLabelProperties",
                "label": "@hud_NoTarget"
            }
        });
        let result = r.resolve_text_detailed(5, &raw, &defaults);
        assert_eq!(result.text, "NO TARGET");
    }

    /// `TagFromBoolean` carries each branch tag as a Tag record-reference OBJECT
    /// (`isTrue`/`isFalse` with `_RecordName_`/`_RecordId_`), not just an inline
    /// string. A `PrimaryStateTag` whose boolean is true must resolve to the
    /// `isTrue` tag (the footer's selected-screen state tag).
    #[test]
    fn tag_from_boolean_resolves_is_true_record_ref_object() {
        let resolver = BindingResolver::from_operations(&[
            json!({
                "_Type_": "BuildingBlocks_BindingsStringField",
                "widget": "ptr:5", "field": "PrimaryStateTag", "input": "ptr:11"
            }),
            json!({
                "_Pointer_": "ptr:11",
                "_Type_": "BuildingBlocks_BindingsTagFromBoolean",
                "isTrue": { "_RecordName_": "Tag.deadbeef-0000-0000-0000-000000000000",
                            "_RecordId_": "deadbeef-0000-0000-0000-000000000000" },
                "isFalse": null,
                "input": "ptr:12"
            }),
            json!({
                "_Pointer_": "ptr:12",
                "_Type_": "BuildingBlocks_BindingsBooleanComponentParameter",
                "parameter": "P", "defaultValue": true
            }),
        ]);
        let defaults = DefaultValueRegistry::default();
        assert_eq!(
            resolver.resolve_field_text(5, "PrimaryStateTag", &defaults).as_deref(),
            Some("Tag.deadbeef-0000-0000-0000-000000000000"),
            "TagFromBoolean must resolve the isTrue record-ref object when enabled"
        );
    }

    /// When the boolean is false and `isFalse` is null, the tag resolves to none.
    #[test]
    fn tag_from_boolean_false_with_null_is_false_resolves_none() {
        let resolver = BindingResolver::from_operations(&[
            json!({
                "_Type_": "BuildingBlocks_BindingsStringField",
                "widget": "ptr:5", "field": "PrimaryStateTag", "input": "ptr:11"
            }),
            json!({
                "_Pointer_": "ptr:11",
                "_Type_": "BuildingBlocks_BindingsTagFromBoolean",
                "isTrue": { "_RecordName_": "Tag.deadbeef-0000-0000-0000-000000000000" },
                "isFalse": null,
                "input": "ptr:12"
            }),
            json!({
                "_Pointer_": "ptr:12",
                "_Type_": "BuildingBlocks_BindingsBooleanComponentParameter",
                "parameter": "P", "defaultValue": false
            }),
        ]);
        let defaults = DefaultValueRegistry::default();
        assert_eq!(resolver.resolve_field_text(5, "PrimaryStateTag", &defaults), None);
    }

    /// `BooleanFromInteger` comparing two *unbound* `IntegerComponentParameter`s
    /// (both falling back to their `defaultValue`) is not a real runtime state:
    /// the footer's `bindingid(ParamInput0, -1) == selectedmfd(ParamInput1, -1)`
    /// gate is only true at runtime when this screen is the selected MFD, which it
    /// is not at rest. Comparing the two sentinel defaults (`-1 == -1`) would
    /// spuriously fire the selected-screen tag, so the evaluator must fall to the
    /// at-rest heuristic (`Equal` → false) and resolve no `isTrue` tag — leaving
    /// the unselected footer styling (dark box, orange text) the reference shows.
    #[test]
    fn tag_from_boolean_via_two_unbound_params_equal_resolves_none() {
        let resolver = BindingResolver::from_operations(&[
            json!({
                "_Type_": "BuildingBlocks_BindingsStringField",
                "widget": "ptr:5", "field": "PrimaryStateTag", "input": "ptr:11"
            }),
            json!({
                "_Pointer_": "ptr:11",
                "_Type_": "BuildingBlocks_BindingsTagFromBoolean",
                "isTrue": { "_RecordName_": "Tag.fef243b7-0000-0000-0000-000000000000" },
                "isFalse": null,
                "input": "ptr:12"
            }),
            json!({
                "_Pointer_": "ptr:12",
                "_Type_": "BuildingBlocks_BindingsBooleanFromInteger",
                "type": "Equal", "value": 0,
                "inputL": "ptr:13", "inputR": "ptr:14"
            }),
            json!({ "_Pointer_": "ptr:13", "_Type_": "BuildingBlocks_BindingsIntegerComponentParameter",
                    "parameter": "ParamInput0", "defaultValue": -1 }),
            json!({ "_Pointer_": "ptr:14", "_Type_": "BuildingBlocks_BindingsIntegerComponentParameter",
                    "parameter": "ParamInput1", "defaultValue": -1 }),
        ]);
        let defaults = DefaultValueRegistry::default();
        assert_eq!(
            resolver.resolve_field_text(5, "PrimaryStateTag", &defaults),
            None,
            "two unbound-param sentinels (-1 == -1) must NOT fire the selected tag"
        );
    }

    /// A `BooleanFromInteger` comparing one unbound parameter against an *authored
    /// literal* (`value`/`inputR` constant) is a real comparison the designer
    /// wrote, so it must evaluate literally — not fall to the heuristic. Here a
    /// parameter defaulting to 5 `Equal` the authored literal 5 is genuinely true.
    #[test]
    fn boolean_from_integer_param_equals_authored_literal_uses_real_comparison() {
        let resolver = BindingResolver::from_operations(&[
            json!({
                "_Type_": "BuildingBlocks_BindingsStringField",
                "widget": "ptr:5", "field": "PrimaryStateTag", "input": "ptr:11"
            }),
            json!({
                "_Pointer_": "ptr:11",
                "_Type_": "BuildingBlocks_BindingsTagFromBoolean",
                "isTrue": { "_RecordName_": "Tag.abcd1234-0000-0000-0000-000000000000" },
                "isFalse": null,
                "input": "ptr:12"
            }),
            json!({
                "_Pointer_": "ptr:12",
                "_Type_": "BuildingBlocks_BindingsBooleanFromInteger",
                "type": "Equal", "value": 5, "inputR": null,
                "inputL": "ptr:13"
            }),
            json!({ "_Pointer_": "ptr:13", "_Type_": "BuildingBlocks_BindingsIntegerComponentParameter",
                    "parameter": "ParamInput0", "defaultValue": 5 }),
        ]);
        let defaults = DefaultValueRegistry::default();
        assert_eq!(
            resolver.resolve_field_text(5, "PrimaryStateTag", &defaults).as_deref(),
            Some("Tag.abcd1234-0000-0000-0000-000000000000"),
            "param(5) Equal authored-literal 5 must be a real comparison → true"
        );
    }

    #[test]
    fn synth_string_widget_ptr_string_maps_to_resolved_string() {
        let resolver = BindingResolver::from_operations(&[json!({
            "_Type_": "_SynthStringWidget_",
            "widget": "ptr:4",
            "resolvedString": "UI/Textures/I_InteractiveScreens/Med/i_med_bioc_menuoption_a.tif"
        })]);
        assert_eq!(
            resolver.resolve_string_binding(4),
            Some("UI/Textures/I_InteractiveScreens/Med/i_med_bioc_menuoption_a.tif")
        );
    }

    #[test]
    fn integer_component_parameter_uses_field_override_before_default() {
        let resolver = BindingResolver::from_operations(&[
            json!({
                "_Pointer_": "ptr:1",
                "_Type_": "BuildingBlocks_BindingsIntegerComponentParameter",
                "parameter": "ParamInput0",
                "defaultValue": 0
            }),
            json!({
                "_Pointer_": "ptr:2",
                "_Type_": "BuildingBlocks_BindingsIntegerVariable",
                "binding": "/AnnunciatorProvider/Issues/Issue1/Severity"
            }),
            json!({
                "_Type_": "BuildingBlocks_BindingsIntegerField",
                "widget": "ptr:100",
                "field": "ParamInput0",
                "input": "ptr:2"
            }),
            json!({
                "_Pointer_": "ptr:3",
                "_Type_": "BuildingBlocks_BindingsTagFromIntegerSwitch",
                "values": [
                    {
                        "first": 1,
                        "second": {
                            "_RecordName_": "Tag.SeverityLow"
                        }
                    },
                    {
                        "first": 2,
                        "second": {
                            "_RecordName_": "Tag.SeverityMed"
                        }
                    }
                ],
                "defaultValue": {
                    "_RecordName_": "Tag.None"
                },
                "input": "ptr:1"
            }),
            json!({
                "_Type_": "BuildingBlocks_BindingsStringField",
                "widget": "ptr:200",
                "field": "PrimaryStateTag",
                "input": "ptr:3"
            }),
        ]);

        let mut defaults = DefaultValueRegistry::default();
        defaults.insert_path(
            "/AnnunciatorProvider/Issues/Issue1/Severity",
            Value::Int(2),
        );

        let tag = resolver
            .resolve_field_text(200, "PrimaryStateTag", &defaults)
            .expect("state tag should resolve from variable override");
        assert_eq!(tag, "Tag.SeverityMed");
    }
