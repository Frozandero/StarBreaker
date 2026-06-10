//! Brand modifier application for BuildingBlocks scenes.
//!
//! Applies brand-style modifiers (SvgPath, ImagePath, FillColor, BorderColor, etc.)
//! to nodes in a `BbScene` by matching `conditionsList` tags against node
//! `style_tag_uuids`.
//!
//! # Condition-matching algorithm
//! An entry matches a node when there exists at least one `conditionsList[i]` such
//! that **every** `conditions[j]` item passes. Each condition item is one of:
//! - `BuildingBlocks_StyleSelectorConditionTag` — `tag._RecordId_` must be in
//!   `node.style_tag_uuids`.
//! - `BuildingBlocks_StyleSelectorConditionAllOfTag` / `…AnyOfTag` — every / any
//!   of `tags[]._RecordId_` must be present.
//! - `BuildingBlocks_StyleSelectorConditionNotTag` — `tag._RecordId_` must be
//!   **absent** (drives at-rest hide rules like the footer's `BG_Neutral`).
//! - `BuildingBlocks_StyleSelectorConditionType` — `type` string must match the
//!   node widget type (e.g. `"Image"` → `WidgetImage`).
//! An entry with EMPTY or ABSENT `conditionsList` matches **every** node
//! (unconditional defaults).

use crate::bb_brand_style::BrandStyle;
use crate::bb_loc::LocFetcher;
use crate::bb_scene::{BbNode, BbNodeId, BbNodeType, BbScene};

mod colors;
mod modifiers;
mod modifiers_number;
#[cfg(test)]
mod tests_colors;
#[cfg(test)]
mod tests_conditions;
#[cfg(test)]
mod tests_conditions_ancestor;
#[cfg(test)]
mod tests_modifiers;
#[cfg(test)]
mod tests_modifiers_number;
#[cfg(test)]
mod tests_scene_styles;
#[cfg(test)]
mod tests_support;

use self::colors::{parse_color_value, ColorStyleRole};
use self::colors::PaletteSources;
use self::modifiers::{apply_inline_color_overlay, apply_modifier};

/// Apply brand-style modifiers to a scene.
///
/// For each node in `scene.nodes`, tests whether any `brand.entries[]` match the
/// node's `style_tag_uuids`, then applies all non-canvas-reference modifiers from
/// matching entries to the node.
///
/// Canvas-reference modifiers (those whose `field._Type_` ends with
/// `CanvasReferenceRecord`) are skipped — these are already handled by the
/// resolve pass in `bb_resolve.rs`.
///
/// When `loc_fetcher` is `Some`, string modifier values that start with `@` are
/// resolved through the localization fetcher before being written to the node.
pub fn apply_brand_modifiers(
    scene: &mut BbScene,
    brand: &BrandStyle<'_>,
    loc_fetcher: Option<&dyn LocFetcher>,
) {
    let palettes = PaletteSources::uniform(brand.raw);
    apply_style_entries(scene, brand.entries, &palettes, Some(&brand.identifier), loc_fetcher);
}

/// Like [`apply_brand_modifiers`], but resolving named colour roles against an
/// explicit palette record. A canvas's `brandStyles[]` container carries only
/// `entries`; the colour palette lives on the `BuildingBlocks_Style` record its
/// `brandIdentifier` names (e.g. `s_drak_hud`). Callers that can fetch that
/// record pass it here so colour modifiers (`BackgroundColor = Disabled@0.1`,
/// `BorderColorTop = Base@1.0`, …) resolve instead of being dropped.
pub fn apply_brand_modifiers_with_palette(
    scene: &mut BbScene,
    brand: &BrandStyle<'_>,
    palette_source: &serde_json::Value,
    loc_fetcher: Option<&dyn LocFetcher>,
) {
    // Chrome fields resolve against the fetched brand palette; fill fields keep
    // the container-only behaviour (see `PaletteSources`).
    let palettes = PaletteSources {
        fills: brand.raw,
        chrome: palette_source,
    };
    apply_style_entries(scene, brand.entries, &palettes, Some(&brand.identifier), loc_fetcher);
}

/// Apply arbitrary canvas style entries (for example `defaultStyles.entries`) to a scene.
pub fn apply_scene_style_entries(
    scene: &mut BbScene,
    entries: &[serde_json::Value],
    palette_source: &serde_json::Value,
    loc_fetcher: Option<&dyn LocFetcher>,
) {
    let palettes = PaletteSources::uniform(palette_source);
    apply_style_entries(scene, entries, &palettes, None, loc_fetcher);
}

fn apply_style_entries(
    scene: &mut BbScene,
    entries: &[serde_json::Value],
    palettes: &PaletteSources<'_>,
    style_identifier: Option<&str>,
    loc_fetcher: Option<&dyn LocFetcher>,
) {
    let style_probe = std::env::var("BB_A3_STYLE_PROBE").as_deref() == Ok("1");
    let node_ids: Vec<_> = scene.nodes.keys().copied().collect();
    for node_id in node_ids {
        let matching_entries: Vec<&serde_json::Value> = {
            let Some(node) = scene.nodes.get(&node_id) else {
                continue;
            };
            let matches: Vec<&serde_json::Value> = entries
                .iter()
                .filter(|entry| entry_matches_scene(entry, node_id, node, scene))
                .collect();
            if style_probe {
                let matched_names: Vec<&str> = matches
                    .iter()
                    .filter_map(|e| e.get("name").and_then(|v| v.as_str()))
                    .collect();
                log::info!(
                    "A3-style-probe: id=ptr:{} name={:?} tags={:?} matches={:?}",
                    node_id,
                    node.name,
                    node.style_tag_uuids,
                    matched_names
                );
            }
            matches
        };

        let Some(node) = scene.nodes.get_mut(&node_id) else {
            continue;
        };
        if style_identifier.is_some_and(looks_like_style_brand_identifier) {
            node.raw.as_object_mut().and_then(|obj| {
                obj.insert(
                    "__BrandIdentifier".to_string(),
                    serde_json::Value::String(style_identifier.unwrap().to_string()),
                )
            });
        }
        apply_inline_color_overlay(node, palettes.fills);
        resolve_node_background_color(node, palettes.fills);
        for entry in &matching_entries {
            apply_entry_modifiers(entry, node, palettes, loc_fetcher);
            record_applied_style_entry(node, entry);
        }
    }
}

fn record_applied_style_entry(node: &mut BbNode, entry: &serde_json::Value) {
    let Some(obj) = node.raw.as_object_mut() else {
        return;
    };
    let slot = obj
        .entry("__AppliedStyleEntries".to_string())
        .or_insert_with(|| serde_json::Value::Array(Vec::new()));
    if let serde_json::Value::Array(items) = slot {
        items.push(entry.clone());
    }
}

fn looks_like_style_brand_identifier(identifier: &str) -> bool {
    let lower = identifier.to_ascii_lowercase();
    lower.starts_with("s_") || lower.starts_with("gen_")
}

fn resolve_node_background_color(node: &mut BbNode, palette_source: &serde_json::Value) {
    if node
        .background
        .as_ref()
        .and_then(|bg| bg.fill_colour)
        .is_some()
    {
        return;
    }

    let background_color_value = node.raw.get("BackgroundColor");
    let authored_background_color_value = node
        .raw
        .get("background")
        .and_then(|bg| bg.get("color"))
        .filter(|value| !value.is_null());

    let Some(color_value) = background_color_value.or(authored_background_color_value) else {
        return;
    };
    let Some(color) = parse_color_value(color_value, palette_source, ColorStyleRole::Surface) else {
        return;
    };

    if node.background.is_none() {
        node.background = Some(Default::default());
    }
    if let Some(bg) = node.background.as_mut() {
        bg.fill_colour = Some(color);
    }
}

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
fn within_materialised_entry(node: &BbNode, scene: &BbScene) -> bool {
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

fn condition_matches_node(
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
        // `breakConditions` bound the walk, but only within MATERIALISED list
        // entries (`_MaterialisedEntry_`, the power pip stacks): per-entry
        // state tags must not leak across entry boundaries (an Unpowered
        // pip's fill must not match the column root's Powered tag), and the
        // boundary node itself is tested match-before-break (it carries both
        // the `general-list-item` break tag and its own state tag). An EMPTY
        // conditions list inside such an entry is a negative guard (true
        // unless an ancestor matches the breaks). Outside materialised
        // entries the long-verified semantics the medical baselines pin
        // apply: breaks are ignored and empty conditions match trivially.
        let break_conditions = condition
            .get("breakConditions")
            .and_then(|v| v.as_array())
            .map(|a| a.as_slice())
            .unwrap_or(&[]);
        let scoped_breaks = !break_conditions.is_empty() && within_materialised_entry(node, scene);
        if conditions.is_empty() && !scoped_breaks {
            // Legacy semantics: an empty conditions list matches at the first
            // resolvable ancestor (a parentless node has none and fails).
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
                return false;
            }
            current = ancestor.parent;
        }
        return conditions.is_empty();
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

fn condition_tag_id(condition: &serde_json::Value) -> Option<&str> {
    let tag = condition.get("tag")?;
    tag_ref_id(tag)
}

fn tag_ref_id(tag: &serde_json::Value) -> Option<&str> {
    tag.get("_RecordId_")
        .and_then(|v| v.as_str())
        .or_else(|| tag.as_str())
}

/// Return `true` when `type_str` from a `ConditionType` entry matches the node type.
///
/// Maps the game's short widget-family names (e.g. `"Image"`) to our `BbNodeType`
/// variants.  Unknown type strings return `false`.
fn node_type_matches(type_str: &str, ty: &BbNodeType) -> bool {
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
        _ => false,
    }
}

/// Apply all modifiers from a matching entry to a node.
fn apply_entry_modifiers(
    entry: &serde_json::Value,
    node: &mut BbNode,
    palettes: &PaletteSources<'_>,
    loc_fetcher: Option<&dyn LocFetcher>,
) {
    let modifiers = match entry.get("modifiers").and_then(|v| v.as_array()) {
        Some(m) => m,
        None => return,
    };

    for modifier in modifiers {
        apply_modifier(modifier, node, palettes, loc_fetcher);
    }
}

