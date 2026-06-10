//! Number/boolean modifier application tests for the geometry fields split
//! into `modifiers_number.rs`: padding, border corner radii, and svg flips.

use super::tests_support::make_test_scene;
use super::*;
use serde_json::json;

    /// `Padding*` modifiers must land on the typed `node.padding` the layout
    /// engine reads — the modular-kit button `Root` entry insets the icon
    /// instance inside the chrome box with `PaddingTop/Right/Bottom/Left 15`.
    #[test]
    fn test_padding_fields_apply_to_typed_padding() {
        let mut scene = make_test_scene();
        let brand = BrandStyle {
            identifier: "test_brand".to_string(),
            entries: &[json!({
                "modifiers": [
                    {
                        "_Type_": "BuildingBlocks_FieldModifierNumber",
                        "field": "PaddingTop",
                        "value": 15.0
                    },
                    {
                        "_Type_": "BuildingBlocks_FieldModifierNumber",
                        "field": "PaddingRight",
                        "value": 16.0
                    },
                    {
                        "_Type_": "BuildingBlocks_FieldModifierNumber",
                        "field": "PaddingBottom",
                        "value": 17.0
                    },
                    {
                        "_Type_": "BuildingBlocks_FieldModifierNumber",
                        "field": "PaddingLeft",
                        "value": 18.0
                    }
                ]
            })],
            raw: &json!({}),
        };

        apply_brand_modifiers(&mut scene, &brand, None);

        let node = scene.nodes.get(&1).unwrap();
        assert_eq!(node.padding.top, 15.0);
        assert_eq!(node.padding.right, 16.0);
        assert_eq!(node.padding.bottom, 17.0);
        assert_eq!(node.padding.left, 18.0);
    }

    /// `SvgFlipHorizontal` writes the authored raw `svgFill.flipHorizontal`
    /// the IR's asset-layout reader consumes — the MFD header's "Button Icon
    /// Flip" entry mirrors the left nav arrow.
    #[test]
    fn test_svg_flip_horizontal_writes_raw_svg_fill() {
        let mut scene = make_test_scene();
        let brand = BrandStyle {
            identifier: "test_brand".to_string(),
            entries: &[json!({
                "modifiers": [
                    {"_Type_": "BuildingBlocks_FieldModifierBoolean", "field": "SvgFlipHorizontal", "value": true}
                ]
            })],
            raw: &json!({}),
        };

        apply_brand_modifiers(&mut scene, &brand, None);

        let node = scene.nodes.get(&1).unwrap();
        assert_eq!(
            node.raw
                .get("svgFill")
                .and_then(|s| s.get("flipHorizontal"))
                .and_then(|v| v.as_bool()),
            Some(true),
            "flip must land on raw svgFill.flipHorizontal"
        );
    }

    /// `Border<Corner>Radius` modifiers write the authored raw border-radius
    /// structure the IR's `node_corner_radius` reads — the sk RootGhost entry
    /// rounds the ghost button chrome with all four corners at 6.0.
    #[test]
    fn test_border_corner_radius_fields_apply_to_raw_border() {
        let mut scene = make_test_scene();
        let brand = BrandStyle {
            identifier: "test_brand".to_string(),
            entries: &[json!({
                "modifiers": [
                    {"_Type_": "BuildingBlocks_FieldModifierNumber", "field": "BorderTopLeftRadius", "value": 6.0},
                    {"_Type_": "BuildingBlocks_FieldModifierNumber", "field": "BorderTopRightRadius", "value": 6.0},
                    {"_Type_": "BuildingBlocks_FieldModifierNumber", "field": "BorderBottomLeftRadius", "value": 6.0},
                    {"_Type_": "BuildingBlocks_FieldModifierNumber", "field": "BorderBottomRightRadius", "value": 6.0}
                ]
            })],
            raw: &json!({}),
        };

        apply_brand_modifiers(&mut scene, &brand, None);

        let node = scene.nodes.get(&1).unwrap();
        for corner in ["topLeftRadius", "topRightRadius", "bottomLeftRadius", "bottomRightRadius"] {
            let radius = node
                .raw
                .get("border")
                .and_then(|b| b.get(corner))
                .and_then(|c| c.get("radius"))
                .and_then(|r| r.get("value"))
                .and_then(|v| v.as_f64());
            assert_eq!(radius, Some(6.0), "corner '{corner}' radius should be 6.0");
        }
    }

