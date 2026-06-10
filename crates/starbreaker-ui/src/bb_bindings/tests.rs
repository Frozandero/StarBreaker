use serde_json::json;

use super::*;
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
        static_variable_values: Default::default(),
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

