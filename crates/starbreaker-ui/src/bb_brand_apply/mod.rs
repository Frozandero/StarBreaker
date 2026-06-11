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
mod conditions;
mod modifiers;
mod modifiers_number;
#[cfg(test)]
mod tests_colors;
#[cfg(test)]
mod tests_conditions;
#[cfg(test)]
mod tests_inline_styles;
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
pub(crate) use self::conditions::*;
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
    apply_style_entries(scene, brand.entries, &palettes, Some(&brand.identifier), loc_fetcher, None);
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
    apply_style_entries(scene, brand.entries, &palettes, Some(&brand.identifier), loc_fetcher, None);
}

/// Like [`apply_brand_modifiers_with_palette`], but only nodes whose `raw`
/// carries `scope_marker` participate. Standard-template module sheets are
/// per-instantiated-canvas in the engine: the scrollbar sheet's `Root` entry
/// targets the generic `canvas-proxy-root` tag every expanded standard root
/// carries, so a scene-wide application would restyle icon/button roots too.
pub fn apply_brand_modifiers_with_palette_scoped(
    scene: &mut BbScene,
    brand: &BrandStyle<'_>,
    palette_source: &serde_json::Value,
    loc_fetcher: Option<&dyn LocFetcher>,
    scope_marker: &str,
) {
    let palettes = PaletteSources {
        fills: brand.raw,
        chrome: palette_source,
    };
    apply_style_entries(
        scene,
        brand.entries,
        &palettes,
        Some(&brand.identifier),
        loc_fetcher,
        Some(scope_marker),
    );
}

/// Apply arbitrary canvas style entries (for example `defaultStyles.entries`) to a scene.
pub fn apply_scene_style_entries(
    scene: &mut BbScene,
    entries: &[serde_json::Value],
    palette_source: &serde_json::Value,
    loc_fetcher: Option<&dyn LocFetcher>,
) {
    let palettes = PaletteSources::uniform(palette_source);
    apply_style_entries(scene, entries, &palettes, None, loc_fetcher, None);
}

fn apply_style_entries(
    scene: &mut BbScene,
    entries: &[serde_json::Value],
    palettes: &PaletteSources<'_>,
    style_identifier: Option<&str>,
    loc_fetcher: Option<&dyn LocFetcher>,
    scope_marker: Option<&str>,
) {
    let style_probe = std::env::var("BB_A3_STYLE_PROBE").as_deref() == Ok("1");
    let node_ids: Vec<_> = scene.nodes.keys().copied().collect();
    for node_id in node_ids {
        let (matching_entries, inline_entries): (Vec<&serde_json::Value>, Vec<serde_json::Value>) = {
            let Some(node) = scene.nodes.get(&node_id) else {
                continue;
            };
            if let Some(marker) = scope_marker
                && node.raw.get(marker).is_none()
            {
                continue;
            }
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
                    "A3-style-probe[{}]: id=ptr:{} name={:?} tags={:?} matches={:?}",
                    style_identifier.unwrap_or("?"),
                    node_id,
                    node.name,
                    node.style_tag_uuids,
                    matched_names
                );
            }
            let inline: Vec<serde_json::Value> = node
                .raw
                .get("inlineStyles")
                .and_then(|v| v.as_array())
                .map(|entries| {
                    entries
                        .iter()
                        .filter(|entry| entry_matches_scene(entry, node_id, node, scene))
                        .cloned()
                        .collect()
                })
                .unwrap_or_default();
            (matches, inline)
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
        // The node's own authored `inlineStyles` are the FINAL cascade stage
        // (highest specificity, above brand/default entries; the power
        // screen's `text_BatteryTitle` / `text_OutputTitle` author an inline
        // `FontSize 30` that overrides their brand-standard size). Applied at
        // the end of every entry pass so a later pass cannot bury them. An
        // inline `FontSize` is additionally marked so font resolution prefers
        // it over the brand-table standard.
        for entry in &inline_entries {
            let sets_font_size = entry
                .get("modifiers")
                .and_then(|v| v.as_array())
                .is_some_and(|mods| {
                    mods.iter()
                        .any(|m| m.get("field").and_then(|f| f.as_str()) == Some("FontSize"))
                });
            apply_entry_modifiers(entry, node, palettes, loc_fetcher);
            if sets_font_size
                && let Some(obj) = node.raw.as_object_mut()
            {
                obj.insert("__InlineFontSize".to_string(), serde_json::Value::Bool(true));
            }
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

