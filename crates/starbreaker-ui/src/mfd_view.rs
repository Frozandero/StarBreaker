//! MFD active content-view selection for frame-canvas renders.
//!
//! The MFD frame canvas (e.g. `m_eng_mfdcontent`) statically embeds every
//! content view (target/self/power/…) as mutually-exclusive `WidgetCanvas`
//! slots; at runtime the engine instantiates exactly one based on the bound view
//! and aspect-ratio state. A static export has no runtime state, so the frame's
//! boolean state filter marks the slots `Instantiated` by an arbitrary default —
//! dropping the bound view's content during resolution (Pass 2) and merging the
//! wrong one.
//!
//! [`apply_bound_view_instantiation`] corrects this at resolution time: it forces
//! the slot whose embedded `canvas:` reference matches the binding's content
//! canvas to be instantiated, and forces the mutually-exclusive sibling slots
//! out. Selection is structural (no per-asset names): the mutually-exclusive
//! slots are the `WidgetCanvas` nodes sharing the matched slot's parent and layer
//! (the footer/chrome sits on a distinct layer and is untouched).

use std::collections::HashSet;

use crate::bb_scene::{BbNode, BbNodeId, BbNodeType, BbScene};

/// Adjust `instantiated_false` so only the content view matching
/// `bound_content_record_name` is instantiated among its mutually-exclusive
/// peers.
///
/// `bound_content_record_name` is the binding's content canvas `_RecordName_`
/// (e.g. `"BuildingBlocks_Canvas.MC_S_Target_Master"`). The matched slot is
/// removed from `instantiated_false` (forced on) and its mutually-exclusive
/// peers — `WidgetCanvas` nodes sharing its parent **and** layer that reference a
/// different canvas — are inserted (forced off). No-op when no slot matches
/// (non-MFD frames, or frames that don't embed the bound view).
///
/// Peer detection is structural and **relative** — peers share the *matched
/// slot's* layer, not a hard-coded one — so the always-on chrome (header/footer)
/// on a different layer is left alone, and the frame's absolute layering doesn't
/// matter. The one assumption is that a frame's mutually-exclusive content views
/// share a layer with each other (observed on `m_eng_mfdcontent`: views layer 5,
/// footer layer 10). If a future ship authors its views on differing layers, the
/// 3d crate's `SMFDView` view-canvas set (already enumerated for the binding)
/// would be the stronger signal to thread in.
pub fn apply_bound_view_instantiation(
    scene: &BbScene,
    bound_content_record_name: &str,
    instantiated_false: &mut HashSet<BbNodeId>,
) {
    let Some((matched_id, parent, layer)) = matched_view_slot(scene, bound_content_record_name)
    else {
        return;
    };

    instantiated_false.remove(&matched_id);
    for (id, node) in &scene.nodes {
        if *id != matched_id
            && node.ty == BbNodeType::WidgetCanvas
            && node.parent == parent
            && node.layer == layer
            && canvas_ref_record_name(node).is_some()
        {
            instantiated_false.insert(*id);
        }
    }
}

/// The `(id, parent, layer)` of the `WidgetCanvas` slot whose `canvas:` reference
/// matches `bound_content_record_name`, or `None`.
fn matched_view_slot(
    scene: &BbScene,
    bound_content_record_name: &str,
) -> Option<(BbNodeId, Option<BbNodeId>, i32)> {
    let bound = normalize_record_name(bound_content_record_name);
    if bound.is_empty() {
        return None;
    }
    scene.nodes.iter().find_map(|(id, node)| {
        if node.ty != BbNodeType::WidgetCanvas {
            return None;
        }
        (canvas_ref_record_name(node)? == bound).then_some((*id, node.parent, node.layer))
    })
}

/// The lower-cased, prefix-stripped record name a `WidgetCanvas` node references
/// via its `canvas:` URL, or `None` if it has no canvas reference.
fn canvas_ref_record_name(node: &BbNode) -> Option<String> {
    let url = node
        .raw
        .get("canvas")
        .and_then(|value| value.as_str())
        .filter(|s| !s.is_empty() && *s != "null")?;
    Some(normalize_record_name(&crate::record_name::extract_record_name(url)))
}

fn normalize_record_name(name: &str) -> String {
    name.strip_prefix("BuildingBlocks_Canvas.")
        .unwrap_or(name)
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A frame embedding two mutually-exclusive content views (same parent +
    /// layer) plus a footer at a different layer.
    fn frame_scene() -> serde_json::Value {
        serde_json::json!({
            "_RecordName_": "BuildingBlocks_Canvas.M_Eng_MfdContent",
            "_RecordValue_": {
                "size": {"x": 800, "y": 600},
                "scene": [
                    {"_Pointer_": "ptr:1", "_Type_": "BuildingBlocks_DisplayWidget", "name": "base_content",
                     "isActive": true, "sizing": {"width": {"behavior": "Fixed", "value": 800.0}, "height": {"behavior": "Fixed", "value": 600.0}}},
                    {"_Pointer_": "ptr:2", "_Type_": "BuildingBlocks_WidgetCanvas", "name": "canvas_PortraitMFDView",
                     "parent": "_PointsTo_:ptr:1", "isActive": true, "layer": 5,
                     "canvas": "file://./screens/target/mc_s_target_master.json"},
                    {"_Pointer_": "ptr:3", "_Type_": "BuildingBlocks_WidgetCanvas", "name": "canvas_LandscapeMFDView",
                     "parent": "_PointsTo_:ptr:1", "isActive": true, "layer": 5,
                     "canvas": "file://./screens/self/mc_s_self_master.json"},
                    {"_Pointer_": "ptr:4", "_Type_": "BuildingBlocks_WidgetCanvas", "name": "canvas_Header / Footer",
                     "parent": "_PointsTo_:ptr:1", "isActive": true, "layer": 10,
                     "canvas": "file://./header_types/gen_mc_s_header.json"}
                ],
                "operations": []
            }
        })
    }

    fn slot_id(scene: &BbScene, name: &str) -> BbNodeId {
        *scene.nodes.iter().find(|(_, n)| n.name == name).unwrap().0
    }

    #[test]
    fn forces_bound_view_on_and_sibling_off_keeping_footer() {
        let scene = crate::bb_scene::parse_bb_canvas(&frame_scene()).expect("parse");
        let portrait = slot_id(&scene, "canvas_PortraitMFDView");
        let landscape = slot_id(&scene, "canvas_LandscapeMFDView");
        let footer = slot_id(&scene, "canvas_Header / Footer");

        // Resolver default mistakenly skipped the bound (target) slot.
        let mut inst_false: HashSet<BbNodeId> = HashSet::from([portrait]);
        apply_bound_view_instantiation(&scene, "BuildingBlocks_Canvas.MC_S_Target_Master", &mut inst_false);

        assert!(!inst_false.contains(&portrait), "bound target slot must be instantiated");
        assert!(inst_false.contains(&landscape), "sibling self slot must be skipped");
        assert!(!inst_false.contains(&footer), "footer (other layer) must be untouched");
    }

    #[test]
    fn no_op_when_no_view_matches() {
        let scene = crate::bb_scene::parse_bb_canvas(&frame_scene()).expect("parse");
        let mut inst_false: HashSet<BbNodeId> = HashSet::new();
        apply_bound_view_instantiation(&scene, "BuildingBlocks_Canvas.MC_S_Power_Master", &mut inst_false);
        assert!(inst_false.is_empty(), "no matching slot → no changes");
    }

    /// End-to-end through the resolver: a frame embedding two mutually-exclusive
    /// view slots must merge ONLY the bound view's content in Pass 2 (the other
    /// slot is forced into `instantiated_false` and skipped). This guards the real
    /// integration — the unit tests above exercise the helper in isolation, but
    /// the bug they protect against (the bound view's content dropped during
    /// resolution) only manifests through `resolve_canvas_graph`.
    #[test]
    fn resolver_merges_only_the_bound_view_content() {
        let frame = serde_json::json!({
            "_RecordName_": "BuildingBlocks_Canvas.TestFrame",
            "_RecordValue_": {
                "size": {"x": 800, "y": 600},
                "scene": [
                    {"_Pointer_": "ptr:1", "_Type_": "BuildingBlocks_DisplayWidget", "name": "base",
                     "isActive": true, "sizing": {"width": {"behavior": "Fixed", "value": 800.0}, "height": {"behavior": "Fixed", "value": 600.0}}},
                    {"_Pointer_": "ptr:2", "_Type_": "BuildingBlocks_WidgetCanvas", "name": "canvas_ViewA",
                     "parent": "_PointsTo_:ptr:1", "isActive": true, "layer": 5, "canvas": "file://./viewa.json"},
                    {"_Pointer_": "ptr:3", "_Type_": "BuildingBlocks_WidgetCanvas", "name": "canvas_ViewB",
                     "parent": "_PointsTo_:ptr:1", "isActive": true, "layer": 5, "canvas": "file://./viewb.json"}
                ],
                "operations": []
            }
        });
        let view = |name: &str, marker: &str| serde_json::json!({
            "_RecordName_": format!("BuildingBlocks_Canvas.{name}"),
            "_RecordValue_": {"size": {"x": 800, "y": 600}, "scene": [
                {"_Pointer_": "ptr:1", "_Type_": "BuildingBlocks_WidgetTextField", "name": marker,
                 "isActive": true, "text": "x",
                 "sizing": {"width": {"behavior": "Fixed", "value": 100.0}, "height": {"behavior": "Fixed", "value": 20.0}}}
            ], "operations": []}
        });
        let view_a = view("ViewA", "marker_A");
        let view_b = view("ViewB", "marker_B");
        let fetch = move |path: &str| -> Result<serde_json::Value, String> {
            let p = path.to_ascii_lowercase();
            if p.contains("viewa") {
                Ok(view_a.clone())
            } else if p.contains("viewb") {
                Ok(view_b.clone())
            } else {
                Err(format!("unknown canvas: {path}"))
            }
        };

        let scene = crate::bb_resolve::resolve_canvas_graph_with_loc_and_bound_view(
            &frame,
            Some("drak"),
            &fetch,
            None,
            Some("BuildingBlocks_Canvas.ViewA"),
        )
        .expect("resolve");

        let names: Vec<&str> = scene.nodes.values().map(|n| n.name.as_str()).collect();
        assert!(
            names.contains(&"marker_A"),
            "bound view A content must be merged; got {names:?}"
        );
        assert!(
            !names.contains(&"marker_B"),
            "non-bound view B content must NOT be merged; got {names:?}"
        );
    }
}
