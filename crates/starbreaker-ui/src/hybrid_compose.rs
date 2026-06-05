//! Hybrid UI IR renderer that composes IR-backed BB content.
//!
//! Provides `render_ui_ir_with_swf_overlay`, which renders the IR document.
//! SWF visual-exports overlays are intentionally not applied; per-node symbol
//! drawing handles named SWF symbols at their correct canvas positions.

use image::RgbaImage;

use crate::bb_atlas::AtlasLibrary;
use crate::compose::ComposeContext;
use crate::error::UiError;
use crate::ir_compose::render_ui_ir_document;
use crate::ui_ir::UiIrDocument;

/// Render a UI IR document, compositing any required SWF content.
///
/// SWF visual-exports overlays are intentionally NOT applied here. Per-node
/// symbol drawing (for WidgetCustomShape) already handles named SWF symbols
/// at their correct positions. The full-stage visual-exports overlay only
/// adds ActionScript-controlled state-driven content (e.g. targeting brackets,
/// lock indicators) that must not appear in static "default state" renders.
pub fn render_ui_ir_with_swf_overlay(
    document: &UiIrDocument,
    ctx: &ComposeContext<'_>,
    atlas: &AtlasLibrary<'_>,
) -> Result<RgbaImage, UiError> {
    render_ui_ir_document(document, ctx, atlas)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::bb_atlas::AtlasLibrary;
    use crate::compose::ComposeContext;
    use crate::defaults::DefaultValueRegistry;
    use crate::style::{CrtParams, ManufacturerStyle};
    use crate::swf_assets::SwfAssetLibrary;
    use crate::ui_ir::{UI_IR_SCHEMA_VERSION, UiIrDocument, UiRendererHint};
    use crate::canvas::RgbaColor;

    struct EmptyFetcher;

    impl crate::bb_atlas::AssetFetcher for EmptyFetcher {
        fn fetch_image_bytes(&self, _p4k_path: &str) -> Option<Vec<u8>> {
            None
        }
    }

    fn stub_style() -> ManufacturerStyle {
        ManufacturerStyle {
            name: "drak".to_string(),
            primary_tint: RgbaColor { r: 240, g: 168, b: 104, a: 255 },
            secondary_tint: None,
            colour_slots: vec![RgbaColor { r: 240, g: 168, b: 104, a: 255 }],
            background: RgbaColor { r: 48, g: 32, b: 16, a: 255 },
            backlight: RgbaColor { r: 102, g: 214, b: 255, a: 255 },
            font_family_hints: Vec::new(),
            crt: CrtParams::default(),
        }
    }

    #[test]
    fn hybrid_rendering_does_not_require_separate_swf_assets_parameter() {
        // After the D5 fix, render_ui_ir_with_swf_overlay no longer calls
        // draw_swf_visual_exports_rgba. Per-node symbol drawing uses ctx.assets.
        // The function no longer requires a separate swf_assets argument.
        let document = UiIrDocument {
            schema_version: UI_IR_SCHEMA_VERSION,
            canvas_guid: "hybrid-guid".to_string(),
            canvas_name: Some("Hybrid".to_string()),
            target_width: 64,
            target_height: 64,
            selected_style_source: None,
            selected_swf_source: Some("test.swf".to_string()),
            renderer_hint: UiRendererHint::Hybrid,
            confidence: 100,
            warnings: Vec::new(),
            unresolved_references: Vec::new(),
            resolved_asset_refs: Vec::new(),
            missing_asset_refs: Vec::new(),
            nodes: Vec::new(),
        };

        let fetcher = EmptyFetcher;
        let atlas = AtlasLibrary::new(&fetcher, Some("drak"));
        let style = stub_style();
        let defaults = DefaultValueRegistry::with_well_known_path_defaults();
        let assets = SwfAssetLibrary::new(vec![
            b'F', b'W', b'S', 6, 21, 0, 0, 0,
            0x00, 0x18, 0x00, 0x01, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ])
        .expect("minimal swf should parse");
        let ctx = ComposeContext {
            style: &style,
            defaults: &defaults,
            assets: &assets,
        };

        render_ui_ir_with_swf_overlay(&document, &ctx, &atlas)
            .expect("hybrid IR should render without requiring separate swf_assets");
    }
}