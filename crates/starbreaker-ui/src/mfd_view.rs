//! MFD active content-view selection for frame-canvas renders.
//!
//! The MFD frame canvas (e.g. `m_eng_mfdcontent`) statically embeds every
//! content view (target, self, power, …) as mutually-exclusive `WidgetCanvas`
//! slots; at runtime the engine activates exactly one based on the bound view
//! and aspect-ratio state. A static export has no runtime state, so without
//! selection every view renders at once and overlaps.
//!
//! [`select_active_mfd_view`] activates the slot whose embedded `canvas:`
//! reference matches the binding's content canvas and deactivates the sibling
//! view slots, so the capture shows the bound screen. Selection is structural
//! (no per-asset names): the mutually-exclusive slots are the `WidgetCanvas`
//! nodes sharing the matched slot's parent and layer.

use std::collections::HashSet;

use crate::bb_scene::{BbNode, BbNodeId, BbNodeType, BbScene};

/// Activate the content-view slot matching `bound_content_record_name` and
/// deactivate its mutually-exclusive peers.
///
/// `bound_content_record_name` is the binding's content canvas `_RecordName_`
/// (e.g. `"BuildingBlocks_Canvas.MC_S_Target_Master"`). Peers are the
/// `WidgetCanvas` nodes that share the matched slot's parent **and** layer but
/// reference a different canvas — same parent + same layer marks the engine's
/// mutually-exclusive view slots and excludes sibling chrome at other layers
/// (e.g. the header/footer canvas, which sits on a distinct layer). No-op when
/// the frame has no slot matching the bound canvas (e.g. non-MFD frames).
pub fn select_active_mfd_view(scene: &mut BbScene, bound_content_record_name: &str) {
    let bound = normalize_record_name(bound_content_record_name);
    if bound.is_empty() {
        return;
    }

    let matched = scene.nodes.iter().find_map(|(id, node)| {
        if node.ty != BbNodeType::WidgetCanvas {
            return None;
        }
        (canvas_ref_record_name(node)? == bound).then_some((*id, node.parent, node.layer))
    });
    let Some((matched_id, parent, layer)) = matched else {
        return;
    };

    let peers: HashSet<BbNodeId> = scene
        .nodes
        .iter()
        .filter(|(id, node)| {
            **id != matched_id
                && node.ty == BbNodeType::WidgetCanvas
                && node.parent == parent
                && node.layer == layer
                && canvas_ref_record_name(node).is_some()
        })
        .map(|(id, _)| *id)
        .collect();

    deactivate_subtrees(scene, &peers);
    // The frame's runtime view-selector boolean (e.g. `useportraitview`) has no
    // static default, so the resolver may have deactivated the bound view's slot
    // too. Re-activate it so the bound screen renders.
    activate_node(scene, matched_id);
}

/// Re-activate a single node (the matched view slot). Its already-resolved
/// content (e.g. the no-target placeholder state) is preserved — only the slot
/// node's own activation, which the frame view-selector toggled off, is restored.
fn activate_node(scene: &mut BbScene, id: BbNodeId) {
    if let Some(node) = scene.nodes.get_mut(&id) {
        node.is_active = true;
    }
}

/// The lower-cased, prefix-stripped record name a `WidgetCanvas` node references
/// via its `canvas:` URL, or `None` if it has no canvas reference.
fn canvas_ref_record_name(node: &BbNode) -> Option<String> {
    let url = node
        .raw
        .get("canvas")
        .and_then(|value| value.as_str())
        .filter(|s| !s.is_empty() && *s != "null")?;
    Some(normalize_record_name(&crate::pipeline::extract_record_name(url)))
}

fn normalize_record_name(name: &str) -> String {
    name.strip_prefix("BuildingBlocks_Canvas.")
        .unwrap_or(name)
        .to_ascii_lowercase()
}

/// Set `is_active = false` on every node in the subtrees rooted at `roots`.
fn deactivate_subtrees(scene: &mut BbScene, roots: &HashSet<BbNodeId>) {
    let mut stack: Vec<BbNodeId> = roots.iter().copied().collect();
    let mut seen: HashSet<BbNodeId> = HashSet::new();
    while let Some(node_id) = stack.pop() {
        if !seen.insert(node_id) {
            continue;
        }
        let Some(node) = scene.nodes.get_mut(&node_id) else {
            continue;
        };
        node.is_active = false;
        stack.extend(node.children.iter().copied());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A frame embedding two mutually-exclusive content views (same parent +
    /// layer) plus a footer at a different layer. Selecting the target view must
    /// activate it (even if the resolver had deactivated it), deactivate the
    /// sibling self view, and leave the footer untouched.
    fn frame_scene() -> serde_json::Value {
        serde_json::json!({
            "_RecordName_": "BuildingBlocks_Canvas.M_Eng_MfdContent",
            "_RecordValue_": {
                "size": {"x": 800, "y": 600},
                "scene": [
                    {"_Pointer_": "ptr:1", "_Type_": "BuildingBlocks_DisplayWidget", "name": "base_content",
                     "isActive": true, "sizing": {"width": {"behavior": "Fixed", "value": 800.0}, "height": {"behavior": "Fixed", "value": 600.0}}},
                    {"_Pointer_": "ptr:2", "_Type_": "BuildingBlocks_WidgetCanvas", "name": "canvas_PortraitMFDView",
                     "parent": "_PointsTo_:ptr:1", "isActive": false, "layer": 5,
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

    #[test]
    fn selects_bound_view_and_deactivates_sibling_keeping_footer() {
        let mut scene = crate::bb_scene::parse_bb_canvas(&frame_scene()).expect("parse");
        select_active_mfd_view(&mut scene, "BuildingBlocks_Canvas.MC_S_Target_Master");

        let by_name = |n: &str| scene.nodes.values().find(|node| node.name == n).cloned().unwrap();
        assert!(by_name("canvas_PortraitMFDView").is_active, "bound target view must be activated");
        assert!(!by_name("canvas_LandscapeMFDView").is_active, "sibling self view must be deactivated");
        assert!(by_name("canvas_Header / Footer").is_active, "footer (different layer) must stay active");
    }

    #[test]
    fn no_op_when_no_view_matches() {
        let mut scene = crate::bb_scene::parse_bb_canvas(&frame_scene()).expect("parse");
        select_active_mfd_view(&mut scene, "BuildingBlocks_Canvas.MC_S_Power_Master");
        // Nothing matched → leave activation exactly as authored.
        let by_name = |n: &str| scene.nodes.values().find(|node| node.name == n).cloned().unwrap();
        assert!(by_name("canvas_LandscapeMFDView").is_active);
        assert!(!by_name("canvas_PortraitMFDView").is_active);
    }
}
