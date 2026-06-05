//! Programmatic SWF fixture builders and rendering assertion helpers.
//!
//! Fixtures are generated in-memory from Rust code so they stay readable and
//! maintainable across phases. Two canonical fixtures cover the full set of
//! SWF tag types the Flash-rendering pipeline needs to handle:
//!
//! - `make_labeled_frames_swf` — two named `FrameLabel` frames, each placing a
//!   distinct coloured shape; exported as `"shape_a"` / `"shape_b"`.
//! - `make_state_sprites_swf` — two exported `DefineSprite` symbols
//!   (`"StateA"` / `"StateB"`), a `DefineText` static record, a
//!   `DefineEditText` with HTML `initial_text` and a `@loc` key (mirroring
//!   `TargetStatus.swf`'s `id=22`), and an `ImportAssets` font reference.
//!
//! The `assert_swf_symbol_has_non_empty_coverage` helper renders a named
//! exported symbol via the production `draw_swf_symbol` path and asserts that
//! at least one non-transparent pixel was produced. Later phases reuse it to
//! verify that new rendering paths produce actual pixels.

use swf::{
    Color, Compression, ExportedAsset, FillStyle, Fixed8, FrameLabel, GlyphEntry, Header, Matrix,
    PlaceObject, PlaceObjectAction, PointDelta, Rectangle, RemoveObject, Shape, ShapeFlag,
    ShapeRecord, ShapeStyles, Sprite, StyleChangeData, SwfStr, Tag, Text, TextRecord, Twips,
};
use swf::Point;

// ── Shape helpers ─────────────────────────────────────────────────────────────

fn filled_rect_shape(id: swf::CharacterId, w_px: f64, h_px: f64, color: Color) -> Shape {
    let x_max = Twips::from_pixels(w_px);
    let y_max = Twips::from_pixels(h_px);
    let neg_x_max = Twips::from_pixels(-w_px);
    let neg_y_max = Twips::from_pixels(-h_px);
    Shape {
        version: 1,
        id,
        shape_bounds: Rectangle {
            x_min: Twips::ZERO,
            x_max,
            y_min: Twips::ZERO,
            y_max,
        },
        edge_bounds: Rectangle {
            x_min: Twips::ZERO,
            x_max,
            y_min: Twips::ZERO,
            y_max,
        },
        flags: ShapeFlag::empty(),
        styles: ShapeStyles {
            fill_styles: vec![FillStyle::Color(color)],
            line_styles: vec![],
        },
        shape: vec![
            ShapeRecord::StyleChange(Box::new(StyleChangeData {
                move_to: Some(Point::new(Twips::ZERO, Twips::ZERO)),
                fill_style_0: None,
                fill_style_1: Some(1),
                line_style: None,
                new_styles: None,
            })),
            ShapeRecord::StraightEdge {
                delta: PointDelta::new(x_max, Twips::ZERO),
            },
            ShapeRecord::StraightEdge {
                delta: PointDelta::new(Twips::ZERO, y_max),
            },
            ShapeRecord::StraightEdge {
                delta: PointDelta::new(neg_x_max, Twips::ZERO),
            },
            ShapeRecord::StraightEdge {
                delta: PointDelta::new(Twips::ZERO, neg_y_max),
            },
        ],
    }
}

fn place_tag(char_id: swf::CharacterId, depth: swf::Depth) -> Tag<'static> {
    Tag::PlaceObject(Box::new(PlaceObject {
        version: 2,
        action: PlaceObjectAction::Place(char_id),
        depth,
        matrix: Some(Matrix::IDENTITY),
        color_transform: None,
        ratio: None,
        name: None,
        clip_depth: None,
        class_name: None,
        filters: None,
        background_color: None,
        blend_mode: None,
        clip_actions: None,
        has_image: false,
        is_bitmap_cached: None,
        is_visible: None,
        amf_data: None,
    }))
}

fn remove_tag(depth: swf::Depth) -> Tag<'static> {
    Tag::RemoveObject(RemoveObject {
        depth,
        character_id: None,
    })
}

// ── Fixture 1: labeled frames ─────────────────────────────────────────────────

/// Two-frame SWF with named `FrameLabel`s and exported shapes `"shape_a"` /
/// `"shape_b"`.  Exercises: `FrameLabel`, multi-frame main timeline.
///
/// Frame 0 — label `"state_a"`, places shape `id=1` (red 100×100).
/// Frame 1 — label `"state_b"`, removes depth 1, places shape `id=2` (green 100×100).
pub fn make_labeled_frames_swf() -> Vec<u8> {
    let header = Header {
        compression: Compression::None,
        version: 8,
        stage_size: Rectangle {
            x_min: Twips::ZERO,
            x_max: Twips::from_pixels(200.0),
            y_min: Twips::ZERO,
            y_max: Twips::from_pixels(200.0),
        },
        frame_rate: Fixed8::from_f32(24.0),
        num_frames: 2,
    };

    let red = Color { r: 220, g: 50, b: 50, a: 255 };
    let green = Color { r: 50, g: 200, b: 80, a: 255 };

    let tags: Vec<Tag<'_>> = vec![
        Tag::DefineShape(filled_rect_shape(1, 100.0, 100.0, red)),
        Tag::DefineShape(filled_rect_shape(2, 100.0, 100.0, green)),
        Tag::ExportAssets(vec![
            ExportedAsset { id: 1, name: SwfStr::from_utf8_str("shape_a") },
            ExportedAsset { id: 2, name: SwfStr::from_utf8_str("shape_b") },
        ]),
        // Frame 0
        Tag::FrameLabel(FrameLabel {
            label: SwfStr::from_utf8_str("state_a"),
            is_anchor: false,
        }),
        place_tag(1, 1),
        Tag::ShowFrame,
        // Frame 1
        Tag::FrameLabel(FrameLabel {
            label: SwfStr::from_utf8_str("state_b"),
            is_anchor: false,
        }),
        remove_tag(1),
        place_tag(2, 1),
        Tag::ShowFrame,
    ];

    let mut buf = Vec::new();
    swf::write_swf(&header, &tags, &mut buf).expect("make_labeled_frames_swf: write_swf failed");
    buf
}

// ── Fixture 2: state sprites ──────────────────────────────────────────────────

/// Single-frame SWF with exported `DefineSprite`s, `DefineText`, HTML
/// `DefineEditText`, and `ImportAssets`.  Exercises all SWF tag types needed
/// by Phases 2–4.
///
/// Characters:
/// - `id=1` shape — red 80×80 (inside "StateA" sprite)
/// - `id=2` shape — blue 80×80 (inside "StateB" sprite)
/// - `id=3` sprite — places id=1 → exported as `"StateA"`
/// - `id=4` sprite — places id=2 → exported as `"StateB"`
/// - `id=5`, `id=6` — imported font symbols from `"fonts_en.gfx"`
/// - `id=7` `DefineText` — static glyph record referencing font `id=5`
/// - `id=8` `DefineEditText` — HTML `initial_text` with `@hud_NoTarget` loc key,
///   mirrors `TargetStatus.swf id=22`
pub fn make_state_sprites_swf() -> Vec<u8> {
    let header = Header {
        compression: Compression::None,
        version: 8,
        stage_size: Rectangle {
            x_min: Twips::ZERO,
            x_max: Twips::from_pixels(200.0),
            y_min: Twips::ZERO,
            y_max: Twips::from_pixels(200.0),
        },
        frame_rate: Fixed8::from_f32(24.0),
        num_frames: 1,
    };

    let red = Color { r: 220, g: 50, b: 50, a: 255 };
    let blue = Color { r: 50, g: 80, b: 220, a: 255 };

    // HTML initial_text mirrors the real TargetStatus.swf id=22 initial_text.
    // Raw string uses r##..## so that the inner "#ffffff" doesn't end the literal.
    let html_text = r##"<p align="center"><font face="$Furore" size="6" color="#ffffff" letterSpacing="0.600000">@hud_NoTarget</font></p>"##;

    let tags: Vec<Tag<'_>> = vec![
        // Shapes used by the state sprites
        Tag::DefineShape(filled_rect_shape(1, 80.0, 80.0, red)),
        Tag::DefineShape(filled_rect_shape(2, 80.0, 80.0, blue)),
        // State sprites
        Tag::DefineSprite(Sprite {
            id: 3,
            num_frames: 1,
            tags: vec![place_tag(1, 1), Tag::ShowFrame],
        }),
        Tag::DefineSprite(Sprite {
            id: 4,
            num_frames: 1,
            tags: vec![place_tag(2, 1), Tag::ShowFrame],
        }),
        // Font imports (mirrors TargetStatus.swf ImportAssets)
        Tag::ImportAssets {
            url: SwfStr::from_utf8_str("fonts_en.gfx"),
            imports: vec![
                ExportedAsset { id: 5, name: SwfStr::from_utf8_str("$Furore") },
                ExportedAsset { id: 6, name: SwfStr::from_utf8_str("$OrbitronLight") },
            ],
        },
        // Static DefineText referencing the imported font
        Tag::DefineText(Box::new(Text {
            id: 7,
            bounds: Rectangle {
                x_min: Twips::ZERO,
                x_max: Twips::from_pixels(200.0),
                y_min: Twips::ZERO,
                y_max: Twips::from_pixels(30.0),
            },
            matrix: Matrix::IDENTITY,
            records: vec![TextRecord {
                font_id: Some(5),
                color: Some(Color { r: 255, g: 255, b: 255, a: 255 }),
                x_offset: Some(Twips::ZERO),
                y_offset: Some(Twips::from_pixels(20.0)),
                height: Some(Twips::from_pixels(12.0)),
                glyphs: vec![GlyphEntry { index: 0, advance: 240 }],
            }],
        })),
        // HTML EditText mirroring TargetStatus.swf id=22
        Tag::DefineEditText(Box::new(
            swf::EditText::new()
                .with_id(8)
                .with_font_id(5, Twips::from_pixels(20.0))
                .with_bounds(Rectangle {
                    x_min: Twips::ZERO,
                    x_max: Twips::from_pixels(400.0),
                    y_min: Twips::ZERO,
                    y_max: Twips::from_pixels(30.0),
                })
                .with_initial_text(Some(SwfStr::from_utf8_str(html_text)))
                .with_is_html(true),
        )),
        // Export the state sprites
        Tag::ExportAssets(vec![
            ExportedAsset { id: 3, name: SwfStr::from_utf8_str("StateA") },
            ExportedAsset { id: 4, name: SwfStr::from_utf8_str("StateB") },
        ]),
        Tag::ShowFrame,
    ];

    let mut buf = Vec::new();
    swf::write_swf(&header, &tags, &mut buf).expect("make_state_sprites_swf: write_swf failed");
    buf
}

// ── Coverage assertion helper (Phase 0.3) ─────────────────────────────────────

/// Render `symbol_name` from `swf_bytes` via the production `draw_swf_symbol`
/// path and assert that at least one non-transparent pixel was produced.
///
/// Panics with a descriptive message on failure.  Reuse this helper in later
/// phases to verify that new rendering paths produce actual pixels.
pub fn assert_swf_symbol_has_non_empty_coverage(
    swf_bytes: &[u8],
    symbol_name: &str,
    dest_w: u32,
    dest_h: u32,
) {
    use starbreaker_ui::swf_assets::SwfAssetLibrary;
    use starbreaker_ui::swf_render::draw_swf_symbol;
    use tiny_skia::{Color, IntSize, Pixmap, Rect as TskRect};

    let assets = SwfAssetLibrary::new(swf_bytes.to_vec())
        .unwrap_or_else(|e| panic!("SwfAssetLibrary::new failed for symbol '{symbol_name}': {e}"));

    let size = IntSize::from_wh(dest_w, dest_h)
        .unwrap_or_else(|| panic!("invalid dest size {dest_w}x{dest_h}"));
    let mut pixmap = Pixmap::new(size.width(), size.height())
        .unwrap_or_else(|| panic!("Pixmap::new({dest_w}, {dest_h}) failed"));

    let dest = TskRect::from_xywh(0.0, 0.0, dest_w as f32, dest_h as f32)
        .unwrap_or_else(|| panic!("TskRect::from_xywh failed for {dest_w}x{dest_h}"));

    let white = Color::from_rgba8(255, 255, 255, 255);
    let drew = draw_swf_symbol(&mut pixmap, &assets, symbol_name, dest, white, 1.0);

    assert!(
        drew,
        "draw_swf_symbol returned false for symbol '{symbol_name}' — export not found or no drawable characters"
    );

    let non_transparent = pixmap
        .data()
        .chunks_exact(4)
        .filter(|px| px[3] > 0)
        .count();

    assert!(
        non_transparent > 0,
        "symbol '{symbol_name}' drew but produced 0 non-transparent pixels in a {dest_w}x{dest_h} pixmap"
    );
}
