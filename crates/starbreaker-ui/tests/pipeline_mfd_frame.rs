//! TDD tests for Phase 6: MFD frame composition and footer injection.
//!
//! Tests:
//! 1. `mfd_frame_canvas_used_when_distinct_from_content` — mfd binding compiles from frame canvas
//! 2. `mfd_base_root_alpha_patched_to_one` — base_Root alpha=0.0 is patched to 1.0
//! 3. `mfd_screen_name_injected_into_text_ScreenName` — screen_name_loc_key resolves into node
//! 4. `mfd_non_mfd_binding_still_uses_content_canvas` — other bindings unaffected
//! 5. `mfd_frame_render_uses_content_when_same_guid` — no frame substitution when canvas_guid==content_guid

use std::collections::HashMap;

use starbreaker_ui::{
    CanvasFetcher, PipelineInputs, StyleFetcher, SwfFetcher, UiBindingView, UiError,
    compile_ir_for_binding,
};
use starbreaker_ui::bb_loc::LocFetcher;
use starbreaker_ui::pipeline::AssetFetcher;
use starbreaker_ui::ui_ir::UiIrTextPayload;

// ── Minimal fakes ──────────────────────────────────────────────────────────────

struct MapCanvasFetcher(HashMap<String, serde_json::Value>);

impl CanvasFetcher for MapCanvasFetcher {
    fn fetch_canvas_json(&self, guid: &str) -> Result<serde_json::Value, UiError> {
        self.0
            .get(guid)
            .cloned()
            .ok_or_else(|| UiError::RenderError(format!("missing: {guid}")))
    }
}

struct NoSwf;
impl SwfFetcher for NoSwf {
    fn fetch_swf_bytes(&self, _: &str) -> Result<Vec<u8>, UiError> {
        Err(UiError::RenderError("no swf".into()))
    }
}

struct NoStyle;
impl StyleFetcher for NoStyle {
    fn fetch_manufacturer_style(&self, _: &str) -> Result<starbreaker_ui::ManufacturerStyle, UiError> {
        Err(UiError::RenderError("no style".into()))
    }
}

struct NoAsset;
impl AssetFetcher for NoAsset {
    fn fetch_image_bytes(&self, _: &str) -> Option<Vec<u8>> {
        None
    }
}

struct MapLocFetcher(HashMap<String, String>);
impl LocFetcher for MapLocFetcher {
    fn fetch_loc(&self, key: &str) -> Option<String> {
        self.0.get(key).cloned()
    }
}

// ── Canvas fixtures ────────────────────────────────────────────────────────────

fn frame_canvas() -> serde_json::Value {
    serde_json::json!({
        "_RecordName_": "BuildingBlocks_Canvas.FrameCanvas",
        "_RecordValue_": {
            "size": {"x": 1920.0, "y": 1080.0},
            "scene": [
                {
                    "_Pointer_": "ptr:1",
                    "_Type_": "BuildingBlocks_WidgetCanvas",
                    "name": "base_Root",
                    "alpha": 0.0,
                    "isActive": true,
                    "inheritsAlpha": true,
                    // Page-in start-state: authored alpha 0.0 with a page-in
                    // animation block (matches m_eng_mfdcontent.base_Root). The
                    // structural rule settles this to 1.0 for static renders.
                    "animation": {"animationTimeline": null, "duration": 1.0, "additive": true},
                    "sizing": {
                        "width": {"behavior": "Fixed", "value": 1920.0},
                        "height": {"behavior": "Fixed", "value": 1080.0}
                    }
                },
                {
                    "_Pointer_": "ptr:2",
                    "_Type_": "BuildingBlocks_WidgetTextField",
                    "name": "text_ScreenName",
                    "parent": "_PointsTo_:ptr:1",
                    "isActive": true,
                    "text": "@ui_leaderboards_Loadout",
                    "sizing": {
                        "width": {"behavior": "Fixed", "value": 400.0},
                        "height": {"behavior": "Fixed", "value": 50.0}
                    }
                },
                {
                    "_Pointer_": "ptr:3",
                    "_Type_": "BuildingBlocks_WidgetTextField",
                    "name": "frame_chrome_label",
                    "parent": "_PointsTo_:ptr:1",
                    "isActive": true,
                    "text": "FRAME",
                    "sizing": {
                        "width": {"behavior": "Fixed", "value": 400.0},
                        "height": {"behavior": "Fixed", "value": 50.0}
                    }
                }
            ],
            "operations": []
        }
    })
}

fn content_canvas() -> serde_json::Value {
    serde_json::json!({
        "_RecordName_": "BuildingBlocks_Canvas.ContentCanvas",
        "_RecordValue_": {
            "size": {"x": 800.0, "y": 600.0},
            "scene": [
                {
                    "_Pointer_": "ptr:10",
                    "_Type_": "BuildingBlocks_WidgetTextField",
                    "name": "content_only_label",
                    "isActive": true,
                    "text": "CONTENT",
                    "sizing": {
                        "width": {"behavior": "Fixed", "value": 200.0},
                        "height": {"behavior": "Fixed", "value": 50.0}
                    }
                }
            ],
            "operations": []
        }
    })
}

fn mfd_fetcher() -> MapCanvasFetcher {
    let mut map = HashMap::new();
    map.insert("frame-guid".into(), frame_canvas());
    map.insert("content-guid".into(), content_canvas());
    MapCanvasFetcher(map)
}

fn mfd_binding<'a>(screen_name_key: Option<&'a str>) -> UiBindingView<'a> {
    UiBindingView {
        canvas_guid: Some("frame-guid"),
        content_canvas_guid: Some("content-guid"),
        binding_kind: Some("mfd"),
        manufacturer_id: Some("drak"),
        helper_name: Some("TestHelper"),
        default_view_index: None,
        default_screen_slot: None,
        screen_name_loc_key: screen_name_key,
    }
}

fn inputs_with_binding<'a>(
    binding: &'a UiBindingView<'a>,
    canvas_fetcher: &'a MapCanvasFetcher,
    loc_fetcher: Option<&'a dyn LocFetcher>,
) -> PipelineInputs<'a> {
    PipelineInputs {
        binding,
        canvas_fetcher,
        swf_fetcher: &NoSwf,
        style_fetcher: &NoStyle,
        asset_fetcher: &NoAsset,
        target_size: (200, 150),
        apply_postprocess: false,
        animation_sample_percent: None,
        localization_map: None,
        loc_fetcher,
    }
}

// ── Test 1: Frame canvas used for mfd bindings with distinct GUIDs ─────────────

#[test]
fn mfd_frame_canvas_used_when_distinct_from_content() {
    let fetcher = mfd_fetcher();
    let binding = mfd_binding(None);
    let inputs = inputs_with_binding(&binding, &fetcher, None);

    let ir = compile_ir_for_binding(&inputs).expect("compile should succeed");

    // Frame canvas contains "text_ScreenName" and "frame_chrome_label"; content contains "content_only_label".
    // With frame render, IR should include frame nodes.
    let has_screen_name = ir.nodes.iter().any(|n| n.name == "text_ScreenName");
    let has_chrome = ir.nodes.iter().any(|n| n.name == "frame_chrome_label");
    assert!(
        has_screen_name,
        "IR should contain text_ScreenName from the frame canvas; nodes: {:?}",
        ir.nodes.iter().map(|n| &n.name).collect::<Vec<_>>()
    );
    assert!(
        has_chrome,
        "IR should contain frame_chrome_label from the frame canvas"
    );
}

// ── Test 2: base_Root alpha=0.0 patched to 1.0 ────────────────────────────────

#[test]
fn mfd_base_root_alpha_patched_to_one() {
    let fetcher = mfd_fetcher();
    let binding = mfd_binding(None);
    let inputs = inputs_with_binding(&binding, &fetcher, None);

    let ir = compile_ir_for_binding(&inputs).expect("compile should succeed");

    let root_node = ir.nodes.iter().find(|n| n.name == "base_Root");
    assert!(root_node.is_some(), "IR should have base_Root node from frame canvas");
    let alpha = root_node.unwrap().alpha;
    assert_eq!(
        alpha, 1.0,
        "base_Root with alpha=0.0 must be patched to 1.0 in frame render; got {alpha}"
    );
}

// ── Test 3: text_ScreenName gets injected screen name ─────────────────────────

#[test]
fn mfd_screen_name_injected_into_text_ScreenName() {
    let fetcher = mfd_fetcher();
    let binding = mfd_binding(Some("@ui_MFD_View_TargetStatus"));
    let mut loc_map = HashMap::new();
    loc_map.insert("ui_MFD_View_TargetStatus".to_string(), "TARGET STATUS".to_string());
    let loc = MapLocFetcher(loc_map);
    let inputs = inputs_with_binding(&binding, &fetcher, Some(&loc));

    let ir = compile_ir_for_binding(&inputs).expect("compile should succeed");

    let node = ir.nodes.iter().find(|n| n.name == "text_ScreenName")
        .expect("text_ScreenName node must be in IR");

    match &node.text_payload {
        Some(UiIrTextPayload::Resolved { text }) => {
            assert_eq!(
                text, "TARGET STATUS",
                "text_ScreenName must be resolved to the screen name"
            );
        }
        other => panic!(
            "expected Resolved{{text: TARGET STATUS}}, got {:?}", other
        ),
    }
}

// ── Test 4: Non-mfd bindings still use content canvas ─────────────────────────

#[test]
fn mfd_non_mfd_binding_still_uses_content_canvas() {
    let fetcher = mfd_fetcher();
    let binding = UiBindingView {
        canvas_guid: Some("frame-guid"),
        content_canvas_guid: Some("content-guid"),
        binding_kind: Some("physical"),  // NOT "mfd"
        manufacturer_id: Some("drak"),
        helper_name: Some("TestHelper"),
        default_view_index: None,
        default_screen_slot: None,
        screen_name_loc_key: None,
    };
    let inputs = inputs_with_binding(&binding, &fetcher, None);

    let ir = compile_ir_for_binding(&inputs).expect("compile should succeed");

    // Non-mfd bindings should use content_canvas_guid → shows content_only_label, not frame nodes
    let has_content = ir.nodes.iter().any(|n| n.name == "content_only_label");
    let has_frame = ir.nodes.iter().any(|n| n.name == "frame_chrome_label");
    assert!(
        has_content,
        "non-mfd binding must use content canvas (content_only_label expected)"
    );
    assert!(
        !has_frame,
        "non-mfd binding must NOT use frame canvas (frame_chrome_label should be absent)"
    );
}

// ── Test 5: Same canvas_guid and content_canvas_guid uses content directly ────

#[test]
fn mfd_frame_render_uses_content_when_same_guid() {
    let fetcher = mfd_fetcher();
    let binding = UiBindingView {
        canvas_guid: Some("content-guid"),       // same as content
        content_canvas_guid: Some("content-guid"),
        binding_kind: Some("mfd"),
        manufacturer_id: Some("drak"),
        helper_name: Some("TestHelper"),
        default_view_index: None,
        default_screen_slot: None,
        screen_name_loc_key: None,
    };
    let inputs = inputs_with_binding(&binding, &fetcher, None);

    let ir = compile_ir_for_binding(&inputs).expect("compile should succeed");

    let has_content = ir.nodes.iter().any(|n| n.name == "content_only_label");
    let has_frame = ir.nodes.iter().any(|n| n.name == "frame_chrome_label");
    assert!(
        has_content,
        "same-GUID mfd binding must use content canvas (content_only_label expected)"
    );
    assert!(
        !has_frame,
        "same-GUID mfd binding must NOT render frame (frame_chrome_label should be absent)"
    );
}
