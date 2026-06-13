//! Draw-faithful pre-layout text measurement for the render pipeline.
//!
//! Implements [`crate::ui_ir::DrawTextMeasure`] over the SAME SWF font assets
//! and selection logic the compose-time draw uses
//! (`ir_compose::select_imported_ui_font_from_assets`), so `bb_layout`'s
//! intrinsic text boxes hug the glyphs the renderer will actually paint
//! (measure == draw by construction; crates/starbreaker-ui/docs/ui-clipper-parity-handoff.md
//! catalog #3). Returns `None` when no imported font would be selected — the
//! renderer then falls back to the TTF path, whose ×1.5-calibrated estimate
//! `bb_layout` already models.

use crate::ir_compose::{select_imported_ui_font_from_assets, swf_line_box_px};
use crate::swf_assets::SwfAssetLibrary;
use crate::text::TextRenderer;
use crate::ui_ir::DrawTextMeasure;

pub(super) struct SwfDrawTextMeasure<'a> {
    assets: &'a SwfAssetLibrary,
    renderer: TextRenderer,
}

impl<'a> SwfDrawTextMeasure<'a> {
    pub(super) fn new(assets: &'a SwfAssetLibrary) -> Self {
        Self {
            assets,
            renderer: TextRenderer::new(),
        }
    }
}

impl DrawTextMeasure for SwfDrawTextMeasure<'_> {
    fn measure_px(
        &self,
        font_symbol: Option<&str>,
        label_style: Option<&str>,
        text: &str,
        font_px: f32,
        letter_spacing_px: f32,
    ) -> Option<(f32, f32)> {
        if text.trim().is_empty() || font_px <= 0.0 {
            return None;
        }
        let selection =
            select_imported_ui_font_from_assets(self.assets, font_symbol, label_style)?;
        // Width: the widest line's advance width through the draw's own glyph
        // advances (the same primitive the draw uses for wrapping). Height:
        // the draw's full em line box per line.
        let mut width = 0.0f32;
        let mut lines = 0usize;
        for line in text.split('\n') {
            lines += 1;
            if let Some(line_width) = self.renderer.measure_swf_advance_width(
                line,
                selection.font,
                font_px,
                letter_spacing_px,
            ) {
                width = width.max(line_width);
            }
        }
        if width <= 0.0 || lines == 0 {
            return None;
        }
        let height = swf_line_box_px(selection.font, font_px) * lines as f32;
        Some((width, height))
    }
}
