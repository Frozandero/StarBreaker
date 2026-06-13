//! The single selector engine for the BuildingBlocks style cascade
//! (plan P4.2; pass inventory: `crates/starbreaker-ui/docs/ui-cascade-passes.md`).
//!
//! A [`StyleSheet`] describes ONE entry container (its cascade [`Tier`],
//! identifier, palette sources, entries, and scope); [`apply`] runs a slice
//! of sheets in order through the SAME application kernel every legacy
//! entry point uses (`bb_brand_apply::apply_style_entries_filtered`), so
//! conditions, modifiers, probes, and the `__InlineFontSize` /
//! `__EntryFontSize` / `__AppliedStyleEntries` marker semantics are reused
//! verbatim. The TEXT-FORMAT route (Parent-wrapped entries styling a
//! textfield's text format) is gated on [`Tier::Brand`] — the tier carries
//! the semantics the legacy path inferred from the `s_*` identifier prefix.

use crate::bb_loc::LocFetcher;
use crate::bb_scene::{BbNodeId, BbScene};

/// Cascade tier of a sheet's ORIGIN container, lowest first. Deferred
/// late-state re-application is a [`SheetScope::Subtree`], not a tier: a
/// deferred sheet keeps its origin tier (so e.g. a deferred brand sheet
/// keeps brand-tier semantics), per the P4.2 design.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    /// Canvas `style` record link (applied only when no brand resolves).
    StyleLink,
    /// `defaultStyles.sharedStyles` record.
    Shared,
    /// Selected `brandStyles[]` container — the only tier with the
    /// text-format route.
    Brand,
    /// The canvas's `embeddedStyles`.
    Embedded,
    /// Widget-standard module sheets (`sk_<brand>_*styles`) and the
    /// expanded standards' own embedded entries.
    StandardModule,
    /// The empty finishing pass that guarantees node `inlineStyles` apply
    /// on canvases with no other containers.
    Inline,
}

/// What part of the scene a sheet applies to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SheetScope {
    /// Every node.
    Scene,
    /// Only the subtree under this root (deferred late-state passes).
    Subtree(BbNodeId),
    /// Only nodes whose `raw` carries this marker key (widget-standard
    /// module sheets, e.g. `_ScrollbarStandard_`).
    Marker(String),
}

/// One entry container, ready to apply.
pub struct StyleSheet<'a> {
    pub tier: Tier,
    /// Probe/marker identifier (brand name, shared record name,
    /// "embeddedStyles", …). Brand-tier sheets also stamp
    /// `__BrandIdentifier` on nodes, as the legacy path did.
    pub identifier: String,
    /// Palette for fill-class colour roles.
    pub fills: &'a serde_json::Value,
    /// Palette for chrome-class colour roles (`PaletteSources` split).
    pub chrome: &'a serde_json::Value,
    pub entries: &'a [serde_json::Value],
    pub scope: SheetScope,
}

impl<'a> StyleSheet<'a> {
    /// A scene-scoped sheet with one palette for both colour classes.
    pub fn uniform(
        tier: Tier,
        identifier: impl Into<String>,
        palette: &'a serde_json::Value,
        entries: &'a [serde_json::Value],
    ) -> Self {
        Self {
            tier,
            identifier: identifier.into(),
            fills: palette,
            chrome: palette,
            entries,
            scope: SheetScope::Scene,
        }
    }
}

/// Apply `sheets` to the scene IN ORDER. Order is the cascade: callers pass
/// sheets lowest-tier first; within a pass the kernel applies matching
/// entries then the node's own `inlineStyles` last (unchanged semantics).
pub fn apply(scene: &mut BbScene, sheets: &[StyleSheet<'_>], loc_fetcher: Option<&dyn LocFetcher>) {
    for sheet in sheets {
        apply_sheet(scene, sheet, loc_fetcher);
    }
}

fn apply_sheet(scene: &mut BbScene, sheet: &StyleSheet<'_>, loc_fetcher: Option<&dyn LocFetcher>) {
    let allowed = match &sheet.scope {
        SheetScope::Subtree(root) => Some(collect_subtree(scene, *root)),
        _ => None,
    };
    let scope_marker = match &sheet.scope {
        SheetScope::Marker(marker) => Some(marker.as_str()),
        _ => None,
    };
    crate::bb_brand_apply::apply_style_entries_for_engine(
        scene,
        sheet.entries,
        sheet.fills,
        sheet.chrome,
        Some(sheet.identifier.as_str()),
        loc_fetcher,
        scope_marker,
        allowed.as_ref(),
        // The text-format route and the `__BrandIdentifier` stamp are
        // brand-TIER semantics, not identifier-prefix semantics.
        sheet.tier == Tier::Brand,
    );
}

fn collect_subtree(scene: &BbScene, root: BbNodeId) -> std::collections::HashSet<BbNodeId> {
    let mut allowed = std::collections::HashSet::new();
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        if !allowed.insert(id) {
            continue;
        }
        if let Some(node) = scene.nodes.get(&id) {
            stack.extend(node.children.iter().copied());
        }
    }
    allowed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bb_scene::parse_bb_canvas;

    fn scene_with_tagged_nodes() -> BbScene {
        let canvas = serde_json::json!({
            "_RecordValue_": {
                "_Type_": "BuildingBlocks_Canvas",
                "size": {"x": 100.0, "y": 100.0},
                "scene": [
                    {"_Pointer_": "ptr:1", "_Type_": "BuildingBlocks_DisplayWidget",
                     "name": "root", "isActive": true,
                     "styleTags": [{"_RecordId_": "aaaa-tag"}]},
                    {"_Pointer_": "ptr:2", "_Type_": "BuildingBlocks_DisplayWidget",
                     "name": "child", "parent": "_PointsTo_:ptr:1", "isActive": true,
                     "styleTags": [{"_RecordId_": "aaaa-tag"}]}
                ]
            }
        });
        parse_bb_canvas(&canvas).expect("fixture parses")
    }

    fn alpha_entry(name: &str, value: f64) -> serde_json::Value {
        serde_json::json!({
            "name": name,
            "conditionsList": [{
                "conditions": [{
                    "_Type_": "BuildingBlocks_StyleSelectorConditionTag",
                    "tag": {"_RecordId_": "aaaa-tag"}
                }]
            }],
            "modifiers": [{
                "_Type_": "BuildingBlocks_FieldModifierNumber",
                "field": "Alpha",
                "value": value
            }]
        })
    }

    fn node_alpha(scene: &BbScene, name: &str) -> f32 {
        scene
            .nodes
            .values()
            .find(|n| n.name == name)
            .expect("node")
            .alpha
    }

    #[test]
    fn sheets_apply_in_order_later_wins() {
        let mut scene = scene_with_tagged_nodes();
        let palette = serde_json::json!({});
        let low = [alpha_entry("low", 0.25)];
        let high = [alpha_entry("high", 0.75)];
        let sheets = [
            StyleSheet::uniform(Tier::Shared, "shared", &palette, &low),
            StyleSheet::uniform(Tier::Embedded, "embeddedStyles", &palette, &high),
        ];
        apply(&mut scene, &sheets, None);
        assert_eq!(node_alpha(&scene, "root"), 0.75, "later sheet wins");
    }

    #[test]
    fn subtree_scope_leaves_outside_nodes_untouched() {
        let mut scene = scene_with_tagged_nodes();
        let child_id = *scene
            .nodes
            .iter()
            .find(|(_, n)| n.name == "child")
            .map(|(id, _)| id)
            .expect("child id");
        let palette = serde_json::json!({});
        let entries = [alpha_entry("deferred", 0.5)];
        let sheets = [StyleSheet {
            tier: Tier::Embedded,
            identifier: "deferred-origin".to_string(),
            fills: &palette,
            chrome: &palette,
            entries: &entries,
            scope: SheetScope::Subtree(child_id),
        }];
        apply(&mut scene, &sheets, None);
        assert_eq!(node_alpha(&scene, "child"), 0.5, "subtree node styled");
        assert_eq!(node_alpha(&scene, "root"), 1.0, "outside node untouched");
    }

    #[test]
    fn brand_tier_stamps_brand_identifier_marker() {
        let mut scene = scene_with_tagged_nodes();
        let palette = serde_json::json!({});
        let entries = [alpha_entry("brand", 0.9)];
        let sheets = [StyleSheet::uniform(Tier::Brand, "s_test_brand", &palette, &entries)];
        apply(&mut scene, &sheets, None);
        let root = scene.nodes.values().find(|n| n.name == "root").unwrap();
        assert_eq!(
            root.raw.get("__BrandIdentifier").and_then(|v| v.as_str()),
            Some("s_test_brand"),
            "brand tier stamps the identifier"
        );
    }

    #[test]
    fn non_brand_tier_does_not_stamp_brand_identifier() {
        let mut scene = scene_with_tagged_nodes();
        let palette = serde_json::json!({});
        let entries = [alpha_entry("shared", 0.9)];
        let sheets = [StyleSheet::uniform(Tier::Shared, "mfd_g_content", &palette, &entries)];
        apply(&mut scene, &sheets, None);
        let root = scene.nodes.values().find(|n| n.name == "root").unwrap();
        assert!(root.raw.get("__BrandIdentifier").is_none());
    }
}
