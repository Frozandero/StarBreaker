//! Ancestor style-condition tests with `breakConditions`: the walk up the
//! node tree stops at the first ancestor matching the break (the power pip
//! block's `general-list-item` boundary), and a boundary node carrying both
//! the break tag and the queried state tag still matches (match-before-break).

use super::tests_support::make_test_scene;
use super::*;
use serde_json::json;

/// root (Powered) > pip (general-list-item + Unpowered) > fill.
fn pip_scene() -> BbScene {
    let mut scene = make_test_scene();
    {
        let root = scene.nodes.get_mut(&1).unwrap();
        root.style_tag_uuids = vec!["tag-powered".to_string()];
        root.children = vec![2];
    }
    let mut pip = scene.nodes.get(&1).unwrap().clone();
    pip.id = 2;
    pip.parent = Some(1);
    pip.children = vec![3];
    pip.name = "pip".to_string();
    pip.style_tag_uuids = vec!["tag-list-item".to_string(), "tag-unpowered".to_string()];
    pip.raw = serde_json::json!({"_MaterialisedEntry_": true});
    scene.nodes.insert(2, pip);
    let mut fill = scene.nodes.get(&1).unwrap().clone();
    fill.id = 3;
    fill.parent = Some(2);
    fill.children = vec![];
    fill.name = "fill".to_string();
    fill.style_tag_uuids = vec!["tag-fill".to_string()];
    scene.nodes.insert(3, fill);
    scene
}

fn ancestor_entry(state_tag: &str, alpha: f64) -> serde_json::Value {
    json!({
        "name": format!("entry_{state_tag}"),
        "conditionsList": [
            {
                "conditions": [
                    {
                        "_Type_": "BuildingBlocks_StyleSelectorConditionAncestor",
                        "breakConditions": [
                            {"_Type_": "BuildingBlocks_StyleSelectorConditionTag",
                             "tag": {"_RecordId_": "tag-list-item"}}
                        ],
                        "conditions": [
                            {"_Type_": "BuildingBlocks_StyleSelectorConditionTag",
                             "tag": {"_RecordId_": state_tag}}
                        ]
                    },
                    {"_Type_": "BuildingBlocks_StyleSelectorConditionTag",
                     "tag": {"_RecordId_": "tag-fill"}}
                ]
            }
        ],
        "modifiers": [
            {"field": {"_Type_": "BuildingBlocks_FieldModifierNumber",
                       "field": "Alpha", "value": alpha}}
        ]
    })
}

/// The break boundary (pip block) carries the queried tag itself: matches.
#[test]
fn ancestor_condition_matches_boundary_node_state() {
    let mut scene = pip_scene();
    let entries = [ancestor_entry("tag-unpowered", 0.25)];
    let brand = BrandStyle {
        identifier: "test_brand".to_string(),
        entries: &entries,
        raw: &json!({}),
    };
    apply_brand_modifiers(&mut scene, &brand, None);
    assert_eq!(
        scene.nodes.get(&3).unwrap().alpha,
        0.25,
        "fill must match its pip block's Unpowered state"
    );
}

/// A state tag past the break boundary must NOT match: the column root's
/// Powered tag is invisible to the fill (the walk stops at the pip block).
#[test]
fn ancestor_condition_stops_at_break_boundary() {
    let mut scene = pip_scene();
    let entries = [ancestor_entry("tag-powered", 0.5)];
    let brand = BrandStyle {
        identifier: "test_brand".to_string(),
        entries: &entries,
        raw: &json!({}),
    };
    apply_brand_modifiers(&mut scene, &brand, None);
    assert_eq!(
        scene.nodes.get(&3).unwrap().alpha,
        1.0,
        "the root's Powered tag lies beyond the pip-block break boundary"
    );
}


/// The pip selector arrow's authored entry: `IsActive=true` on Polygon
/// widgets whose ancestor (within the `general-list-item` break) carries the
/// `selected` tag, with an empty-conditions negative guard breaking at
/// `Secondary`.
#[test]
fn selector_arrow_entry_activates_polygon_on_selected_pip() {
    let mut scene = pip_scene();
    {
        let pip = scene.nodes.get_mut(&2).unwrap();
        pip.style_tag_uuids = vec![
            "tag-list-item".to_string(),
            "tag-selected".to_string(),
        ];
    }
    {
        let selecter = scene.nodes.get_mut(&3).unwrap();
        selecter.style_tag_uuids = vec!["tag-secondary".to_string()];
    }
    let mut arrow = scene.nodes.get(&1).unwrap().clone();
    arrow.id = 4;
    arrow.parent = Some(3);
    arrow.children = vec![];
    arrow.name = "arrow".to_string();
    arrow.style_tag_uuids = vec![];
    arrow.is_active = false;
    arrow.ty = crate::bb_scene::BbNodeType::Other("BuildingBlocks_WidgetPolygon".to_string());
    scene.nodes.insert(4, arrow);
    scene.nodes.get_mut(&3).unwrap().children = vec![4];

    let entries = [serde_json::json!({
        "name": "PipBox_Selector_Arrow_Visibility",
        "conditionsList": [
            {"conditions": [
                {"_Type_": "BuildingBlocks_StyleSelectorConditionAncestor",
                 "breakConditions": [
                    {"_Type_": "BuildingBlocks_StyleSelectorConditionTag",
                     "tag": {"_RecordId_": "tag-list-item"}}],
                 "conditions": [
                    {"_Type_": "BuildingBlocks_StyleSelectorConditionTag",
                     "tag": {"_RecordId_": "tag-selected"}}]},
                {"_Type_": "BuildingBlocks_StyleSelectorConditionAncestor",
                 "breakConditions": [
                    {"_Type_": "BuildingBlocks_StyleSelectorConditionTag",
                     "tag": {"_RecordId_": "tag-secondary"}}],
                 "conditions": []},
                {"_Type_": "BuildingBlocks_StyleSelectorConditionType", "type": "Polygon"}
            ]}
        ],
        "modifiers": [
            {"_Type_": "BuildingBlocks_FieldModifierBoolean",
             "field": "IsActive", "value": true}
        ]
    })];
    let brand = BrandStyle {
        identifier: "test_brand".to_string(),
        entries: &entries,
        raw: &serde_json::json!({}),
    };
    apply_brand_modifiers(&mut scene, &brand, None);
    assert!(
        scene.nodes.get(&4).unwrap().is_active,
        "the selected pip's arrow must activate"
    );
}
