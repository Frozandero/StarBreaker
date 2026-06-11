//! Style-entry condition matching: which scene nodes a brand/canvas
//! style entry applies to. Owns `entry_matches_scene` (entry-level
//! gate used by bb_resolve host selection), `condition_matches_node`
//! (tag / type / ancestor / interaction-state selectors, including the
//! materialised-entry-scoped `breakConditions` semantics), and the tag
//! and node-type reference helpers.

use super::*;

/// Test whether a brand-style entry matches a node within a scene.
///
/// An entry matches when:
/// - Its `conditionsList` is absent or empty (unconditional), OR
/// - There exists at least one `conditionsList[i]` such that **all**
///   `conditions[j]` items pass. Conditions may be nested (`AllOf`, `AnyOf`,
///   `Parent`), and parent conditions are evaluated against the node's direct
///   parent in the parsed BB scene hierarchy.
/// Whether `node` lies inside a materialised list-entry subtree (self or any
/// ancestor carries the `_MaterialisedEntry_` marker set by the list/array
/// materialisation machinery).
pub(crate) fn within_materialised_entry(node: &BbNode, scene: &BbScene) -> bool {
    if node
        .raw
        .get("_MaterialisedEntry_")
        .and_then(|v| v.as_bool())
        == Some(true)
    {
        return true;
    }
    let mut current = node.parent;
    while let Some(ancestor_id) = current {
        let Some(ancestor) = scene.nodes.get(&ancestor_id) else {
            return false;
        };
        if ancestor
            .raw
            .get("_MaterialisedEntry_")
            .and_then(|v| v.as_bool())
            == Some(true)
        {
            return true;
        }
        current = ancestor.parent;
    }
    false
}

pub(crate) fn entry_matches_scene(
    entry: &serde_json::Value,
    node_id: BbNodeId,
    node: &BbNode,
    scene: &BbScene,
) -> bool {
    let conditions_list = match entry.get("conditionsList").and_then(|v| v.as_array()) {
        Some(cl) => cl,
        None => return true,
    };

    if conditions_list.is_empty() {
        return true;
    }

    conditions_list.iter().any(|conditions_block| {
        let Some(conditions) = conditions_block.get("conditions").and_then(|v| v.as_array()) else {
            return false;
        };
        conditions
            .iter()
            .all(|condition| condition_matches_node(condition, node_id, node, scene))
    })
}

pub(crate) fn condition_matches_node(
    condition: &serde_json::Value,
    node_id: BbNodeId,
    node: &BbNode,
    scene: &BbScene,
) -> bool {
    let cond_type = condition
        .get("_Type_")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if cond_type.ends_with("ConditionAllOfCondition") {
        return condition
            .get("conditions")
            .and_then(|v| v.as_array())
            .map(|conditions| {
                conditions
                    .iter()
                    .all(|child| condition_matches_node(child, node_id, node, scene))
            })
            .unwrap_or(false);
    }

    if cond_type.ends_with("ConditionAnyOfCondition") {
        return condition
            .get("conditions")
            .and_then(|v| v.as_array())
            .map(|conditions| {
                conditions
                    .iter()
                    .any(|child| condition_matches_node(child, node_id, node, scene))
            })
            .unwrap_or(false);
    }

    if cond_type.ends_with("ConditionNotCondition") {
        return condition
            .get("conditions")
            .and_then(|v| v.as_array())
            .map(|conditions| {
                !conditions
                    .iter()
                    .any(|child| condition_matches_node(child, node_id, node, scene))
            })
            .unwrap_or(false);
    }

    if cond_type.ends_with("ConditionParent") {
        let Some(parent_id) = node.parent else {
            return false;
        };
        let Some(parent) = scene.nodes.get(&parent_id) else {
            return false;
        };
        return condition
            .get("conditions")
            .and_then(|v| v.as_array())
            .map(|conditions| {
                conditions
                    .iter()
                    .all(|child| condition_matches_node(child, parent_id, parent, scene))
            })
            .unwrap_or(false);
    }

    if cond_type.ends_with("ConditionAncestor") {
        let Some(conditions) = condition.get("conditions").and_then(|v| v.as_array()) else {
            return false;
        };
        // `breakConditions` bound the walk for POSITIVE conditions only
        // within MATERIALISED list entries (`_MaterialisedEntry_`, the power
        // pip stacks): per-entry state tags must not leak across entry
        // boundaries (an Unpowered pip's fill must not match the column
        // root's Powered tag), and the boundary node itself is tested
        // match-before-break (it carries both the `general-list-item` break
        // tag and its own state tag). An EMPTY conditions list with breaks
        // is a CONTAINMENT test EVERYWHERE: true only when an ancestor
        // matches the breaks (the pip selector arrow's entry requires living
        // inside the Secondary-tagged `PipBox_Selecter` slot; the touch-here
        // fingerprint's "ChangeSizeForASOP" requires an ASOP-host-tagged
        // ancestor and must NOT resize the medical at-rest fingerprint).
        let break_conditions = condition
            .get("breakConditions")
            .and_then(|v| v.as_array())
            .map(|a| a.as_slice())
            .unwrap_or(&[]);
        let containment_test = conditions.is_empty() && !break_conditions.is_empty();
        let scoped_breaks = containment_test
            || (!break_conditions.is_empty() && within_materialised_entry(node, scene));
        if conditions.is_empty() && !scoped_breaks {
            // Legacy semantics: an empty conditions list (and no breaks)
            // matches at the first resolvable ancestor (a parentless node
            // has none and fails).
            return node
                .parent
                .is_some_and(|parent| scene.nodes.contains_key(&parent));
        }
        let mut current = node.parent;
        while let Some(ancestor_id) = current {
            let Some(ancestor) = scene.nodes.get(&ancestor_id) else {
                break;
            };
            if !conditions.is_empty()
                && conditions
                    .iter()
                    .all(|child| condition_matches_node(child, ancestor_id, ancestor, scene))
            {
                return true;
            }
            if scoped_breaks
                && break_conditions
                    .iter()
                    .all(|child| condition_matches_node(child, ancestor_id, ancestor, scene))
            {
                // The walk stops at the boundary: positive conditions beyond
                // it never match; an empty-conditions containment test is
                // satisfied by reaching it.
                return conditions.is_empty();
            }
            current = ancestor.parent;
        }
        return false;
    }

    if cond_type.ends_with("ConditionAnyOfTag") {
        let Some(tags) = condition.get("tags").and_then(|v| v.as_array()) else {
            return false;
        };
        return tags.iter().filter_map(tag_ref_id).any(|tag_id| {
            node.style_tag_uuids
                .iter()
                .any(|node_tag| node_tag == tag_id)
        });
    }

    // ALL listed tags must be present (e.g. `BG_Warning` requires every warning
    // state tag). An empty/unresolvable `tags` list does NOT match — a condition
    // that requires "all of nothing" must not match every node (that over-matches
    // and reveals state-gated elements like the annunciator's ON gradient).
    if cond_type.ends_with("ConditionAllOfTag") {
        let Some(tags) = condition.get("tags").and_then(|v| v.as_array()) else {
            return false;
        };
        let ids: Vec<&str> = tags.iter().filter_map(tag_ref_id).collect();
        return !ids.is_empty()
            && ids
                .iter()
                .all(|tag_id| node.style_tag_uuids.iter().any(|node_tag| node_tag == *tag_id));
    }

    // The tag must be ABSENT (e.g. `BG_Neutral` hides the warning chrome with
    // `NotTag(warning-active)` when no warning state is active). This MUST be
    // checked before the `condition.get("tag").is_some()` catch below, because a
    // `NotTag` also carries a `tag` field and would otherwise be mis-evaluated as
    // a (presence) `ConditionTag` — the inverse of its meaning. An unresolvable
    // tag ref does NOT match (conservative — avoid over-matching).
    if cond_type.ends_with("ConditionNotTag") {
        return condition_tag_id(condition)
            .map(|tag_id| !node.style_tag_uuids.iter().any(|tag| tag == tag_id))
            .unwrap_or(false);
    }

    if cond_type.ends_with("ConditionTag") || condition.get("tag").is_some() {
        return condition_tag_id(condition)
            .map(|tag_id| node.style_tag_uuids.iter().any(|tag| tag == tag_id))
            .unwrap_or(false);
    }

    if cond_type.ends_with("ConditionType") {
        return condition
            .get("type")
            .and_then(|v| v.as_str())
            .map(|type_str| node_type_matches(type_str, &node.ty))
            .unwrap_or(true);
    }

    false
}

pub(crate) fn condition_tag_id(condition: &serde_json::Value) -> Option<&str> {
    let tag = condition.get("tag")?;
    tag_ref_id(tag)
}

pub(crate) fn tag_ref_id(tag: &serde_json::Value) -> Option<&str> {
    tag.get("_RecordId_")
        .and_then(|v| v.as_str())
        .or_else(|| tag.as_str())
}

/// Return `true` when `type_str` from a `ConditionType` entry matches the node type.
///
/// Maps the game's short widget-family names (e.g. `"Image"`) to our `BbNodeType`
/// variants.  Unknown type strings return `false`.
pub(crate) fn node_type_matches(type_str: &str, ty: &BbNodeType) -> bool {
    match type_str {
        "Image" => matches!(ty, BbNodeType::WidgetImage),
        // The game's `Text` widget type is the plain `WidgetText`, NOT a text
        // FIELD: the MFD footer's `TextSpacing` (LetterSpacing 5) and
        // `SelectedName`/`UnSelectedName` entries are all `Type(Text)` and none
        // of their effects appear on the (WidgetTextField) screen-name text in
        // the in-game reference — its tracking and colour come from the brand
        // H1 table instead.
        "Text" => matches!(ty, BbNodeType::WidgetText),
        "TextField" => matches!(ty, BbNodeType::WidgetTextField),
        "Canvas" => matches!(ty, BbNodeType::WidgetCanvas),
        "Icon" => matches!(ty, BbNodeType::WidgetIcon),
        "Card" => matches!(ty, BbNodeType::WidgetCard),
        // The game's `Base` widget type is the base display widget — it matches
        // `DisplayWidget` nodes (e.g. the footer's `base_BG`, which the
        // `ScreenNameBackground` style gates on `AllOf(Type=Base, Tag …)`).
        "Base" | "DisplayWidget" => matches!(ty, BbNodeType::DisplayWidget),
        "CustomShape" => matches!(ty, BbNodeType::WidgetCustomShape),
        "Polygon" => matches!(ty, BbNodeType::Other(kind)
            if kind.eq_ignore_ascii_case("BuildingBlocks_WidgetPolygon")),
        _ => false,
    }
}
