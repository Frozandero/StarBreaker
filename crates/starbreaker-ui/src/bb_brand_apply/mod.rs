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
    // Brand-tier gates, explicit (production styling routes through
    // `bb_style_engine`; this wrapper serves the condition/modifier tests).
    apply_style_entries_gated(
        scene,
        brand.entries,
        &palettes,
        Some(&brand.identifier),
        loc_fetcher,
        None,
        None,
        true,
        true,
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
    apply_style_entries_gated(scene, entries, &palettes, None, loc_fetcher, None, None, false, false);
}


/// The selector-engine entry point (`bb_style_engine`, plan P4.2): same
/// kernel as every legacy wrapper, but the text-format route and the
/// `__BrandIdentifier` stamp are gated EXPLICITLY by the sheet's tier
/// (`brand_tier`) instead of being inferred from the identifier prefix.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_style_entries_for_engine(
    scene: &mut BbScene,
    entries: &[serde_json::Value],
    fills_palette: &serde_json::Value,
    chrome_palette: &serde_json::Value,
    style_identifier: Option<&str>,
    loc_fetcher: Option<&dyn LocFetcher>,
    scope_marker: Option<&str>,
    allowed_nodes: Option<&std::collections::HashSet<BbNodeId>>,
    brand_tier: bool,
) {
    let palettes = PaletteSources {
        fills: fills_palette,
        chrome: chrome_palette,
    };
    apply_style_entries_gated(
        scene,
        entries,
        &palettes,
        style_identifier,
        loc_fetcher,
        scope_marker,
        allowed_nodes,
        brand_tier,
        brand_tier,
    )
}



#[allow(clippy::too_many_arguments)]
fn apply_style_entries_gated(
    scene: &mut BbScene,
    entries: &[serde_json::Value],
    palettes: &PaletteSources<'_>,
    style_identifier: Option<&str>,
    loc_fetcher: Option<&dyn LocFetcher>,
    scope_marker: Option<&str>,
    allowed_nodes: Option<&std::collections::HashSet<BbNodeId>>,
    text_format_route: bool,
    stamp_brand: bool,
) {
    let style_probe = std::env::var("BB_A3_STYLE_PROBE").as_deref() == Ok("1");
    let node_ids: Vec<_> = scene.nodes.keys().copied().collect();
    for node_id in node_ids {
        if let Some(allowed) = allowed_nodes
            && !allowed.contains(&node_id)
        {
            continue;
        }
        let (matching_entries, text_format_entries, inline_entries): (
            Vec<&serde_json::Value>,
            Vec<&serde_json::Value>,
            Vec<serde_json::Value>,
        ) = {
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
            // Entries that select the textfield's implicit text-format child
            // through a `Parent(...)` wrapper (see `entry_matches_text_format`)
            // — they apply only their text-format modifiers. The route runs
            // ONLY for manufacturer BRAND containers (`s_*` identifiers):
            // - embedded containers are name-invoked state/override sheets
            //   (the target screen's `Bright Elements` and the medical bed's
            //   `Textfield_BrightColor_Override` are Bright overrides the
            //   at-rest references do NOT show);
            // - shared generic sheets don't restyle the text format either
            //   (mfd_g_emissions' `Header Text` FillColor=Accent1 — the
            //   in-game emitted values keep the brand H1 deep orange).
            // Brand-tier evidence: the M_Eng_MFDContent drak `FontSizeSmall`
            // table + `Bright Orange Objects`, the power card's
            // `Battery Powered/Depleted Text` sizes and the medical mainmenu
            // banner's `New Style` (Bright + FontSize 40), all verified
            // against in-game captures.
            let text_format_matches: Vec<&serde_json::Value> = if text_format_route {
                entries
                    .iter()
                    .filter(|entry| {
                        !entry_matches_scene(entry, node_id, node, scene)
                            && entry_matches_text_format(entry, node_id, node, scene)
                    })
                    .collect()
            } else {
                Vec::new()
            };
            if style_probe {
                let matched_names: Vec<&str> = matches
                    .iter()
                    .filter_map(|e| e.get("name").and_then(|v| v.as_str()))
                    .collect();
                let text_format_names: Vec<&str> = text_format_matches
                    .iter()
                    .filter_map(|e| e.get("name").and_then(|v| v.as_str()))
                    .collect();
                // eprintln (not log::info) so the probe prints without RUST_LOG,
                // matching BB_TEXT_FORMAT_PROBE (plan P0.4, ledger item 25).
                eprintln!(
                    "A3-style-probe[{}]: id=ptr:{} name={:?} tags={:?} matches={:?} text_format={:?}",
                    style_identifier.unwrap_or("?"),
                    node_id,
                    node.name,
                    node.style_tag_uuids,
                    matched_names,
                    text_format_names
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
            (matches, text_format_matches, inline)
        };

        let Some(node) = scene.nodes.get_mut(&node_id) else {
            continue;
        };
        if stamp_brand && let Some(identifier) = style_identifier {
            node.raw.as_object_mut().and_then(|obj| {
                obj.insert(
                    "__BrandIdentifier".to_string(),
                    serde_json::Value::String(identifier.to_string()),
                )
            });
        }
        apply_inline_color_overlay(node, palettes.fills);
        resolve_node_background_color(node, palettes.fills);
        for entry in &matching_entries {
            if std::env::var("BB_TEXT_FORMAT_PROBE").as_deref() == Ok("1")
                && entry.get("modifiers").and_then(|v| v.as_array()).is_some_and(|mods| {
                    mods.iter().any(|m| {
                        serde_json::to_string(m).unwrap_or_default().contains("\"FontSize\"")
                    })
                })
            {
                eprintln!(
                    "TFPROBE-NORMAL pass={} node={} name={:?} entry={:?} mods={} conds={}",
                    style_identifier.unwrap_or("?"),
                    node_id,
                    node.name,
                    entry.get("name").and_then(|v| v.as_str()).unwrap_or("?"),
                    serde_json::to_string(entry.get("modifiers").unwrap_or(&serde_json::Value::Null))
                        .unwrap_or_default()
                        .chars()
                        .take(220)
                        .collect::<String>(),
                    serde_json::to_string(entry.get("conditionsList").unwrap_or(&serde_json::Value::Null))
                        .unwrap_or_default()
                        .chars()
                        .take(260)
                        .collect::<String>()
                );
            }
            apply_entry_modifiers(entry, node, palettes, loc_fetcher);
            record_applied_style_entry(node, entry);
        }
        for entry in &text_format_entries {
            if std::env::var("BB_TEXT_FORMAT_PROBE").as_deref() == Ok("1") {
                eprintln!(
                    "TFPROBE pass={} node={} name={:?} tags={:?} entry={:?}",
                    style_identifier.unwrap_or("?"),
                    node_id,
                    node.name,
                    node.style_tag_uuids,
                    entry.get("name").and_then(|v| v.as_str()).unwrap_or("?")
                );
            }
            apply_entry_text_format_modifiers(entry, node, palettes, loc_fetcher);
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

/// Apply only the TEXT-FORMAT modifiers of an entry matched via
/// [`entry_matches_text_format`] (the entry selects the textfield's implicit
/// text-format child, so widget-geometry/background modifiers do not apply).
fn apply_entry_text_format_modifiers(
    entry: &serde_json::Value,
    node: &mut BbNode,
    palettes: &PaletteSources<'_>,
    loc_fetcher: Option<&dyn LocFetcher>,
) {
    let Some(modifiers) = entry.get("modifiers").and_then(|v| v.as_array()) else {
        return;
    };
    let mut sets_font_size = false;
    for modifier in modifiers {
        if is_text_format_modifier(modifier) {
            apply_modifier(modifier, node, palettes, loc_fetcher);
            if serde_json::to_string(modifier)
                .unwrap_or_default()
                .contains("\"FontSize\"")
            {
                sets_font_size = true;
            }
        }
    }
    // A TEXT-FORMAT-routed FontSize targets the field's text format directly
    // and outranks the named-style table (the power emissions texts render
    // the M_Eng drak `FontSizeSmall` 40 over the drak Heading1 standard's
    // 60). A LITERAL widget match does NOT set this marker: the medical
    // mainmenu `TierLevel` "T3" takes the same entry's `FillColor=Bright`
    // through its flag-tagged parent, but renders the Title4 table size
    // (~90), not the entry's 40 — widget-level FontSize raw stays below the
    // table (STYLE before RAW).
    if sets_font_size
        && let Some(obj) = node.raw.as_object_mut()
    {
        obj.insert("__EntryFontSize".to_string(), serde_json::Value::Bool(true));
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

