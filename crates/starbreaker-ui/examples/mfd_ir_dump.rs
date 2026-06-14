//! Debugging helper: compile a (framed) MFD binding IR via the local
//! decompiled-record mirror and dump name-filtered nodes with rect, state,
//! fill, and asset fields.
//!
//! Usage:
//!   cargo run --release -p starbreaker-ui --example mfd_ir_dump -- \
//!     <canvas-guid> <content-guid> [name-filter] [WxH]
//!
//! The record mirror is expected at `../ships/dcb_canvas/libs/foundry/records`
//! relative to the workspace root (the standard decomposed-export layout).
//! Uses the pipeline's well-known defaults registry, no SWF/P4K assets.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use starbreaker_ui::pipeline::AssetFetcher;
use starbreaker_ui::{
    CanvasFetcher, PipelineInputs, StyleFetcher, SwfFetcher, UiBindingView, UiError,
};

struct Fs(HashMap<String, PathBuf>);
impl CanvasFetcher for Fs {
    fn fetch_canvas_json(&self, guid: &str) -> Result<serde_json::Value, UiError> {
        let key = guid.to_ascii_lowercase();
        let p = self
            .0
            .get(&key)
            .ok_or_else(|| UiError::RenderError(format!("missing {guid}")))?;
        Ok(serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap())
    }
    fn fetch_canvas_by_name(&self, name: &str) -> Result<serde_json::Value, UiError> {
        self.fetch_canvas_json(name)
    }
}
struct NoSwf;
impl SwfFetcher for NoSwf {
    fn fetch_swf_bytes(&self, _: &str) -> Result<Vec<u8>, UiError> {
        Err(UiError::RenderError("no swf in mirror".into()))
    }
}
struct NoStyle;
impl StyleFetcher for NoStyle {
    fn fetch_manufacturer_style(
        &self,
        _: &str,
    ) -> Result<starbreaker_ui::ManufacturerStyle, UiError> {
        Err(UiError::RenderError("no style db in mirror".into()))
    }
}
struct NoAsset;
impl AssetFetcher for NoAsset {
    fn fetch_image_bytes(&self, _: &str) -> Option<Vec<u8>> {
        None
    }
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                collect(&p, out);
            } else if p.extension().and_then(|x| x.to_str()) == Some("json") {
                out.push(p);
            }
        }
    }
}

/// Minimal stderr logger so library `log::` probes (e.g. `BB_A3_STYLE_PROBE`)
/// are visible from this debug helper. Enabled via `MFD_IR_DUMP_LOG=1`.
struct StderrLogger;
impl log::Log for StderrLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }
    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            eprintln!("[{}] {}", record.level(), record.args());
        }
    }
    fn flush(&self) {}
}
static LOGGER: StderrLogger = StderrLogger;

fn main() {
    if std::env::var("MFD_IR_DUMP_LOG").as_deref() == Ok("1") {
        let _ = log::set_logger(&LOGGER);
        log::set_max_level(log::LevelFilter::Info);
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../ships/dcb_canvas/libs/foundry/records")
        .canonicalize()
        .expect("record mirror at ../ships/dcb_canvas/libs/foundry/records");
    // The Fs fetcher parses the WHOLE record mirror up front (~thousands of
    // files, tens of seconds). Announce it so a slow startup is never mistaken
    // for a pipeline hang (ledger 42: a 94s run was misread as an infinite
    // layout loop). The real export path (`starbreaker ui render`) is ~9s.
    let load_start = std::time::Instant::now();
    let mut files = Vec::new();
    collect(&root, &mut files);
    eprintln!(
        "mfd_ir_dump: loading {} record-mirror files (one-time ~minute; this is harness load, NOT a pipeline loop)…",
        files.len()
    );
    let mut map = HashMap::new();
    for p in &files {
        if let Ok(j) = serde_json::from_str::<serde_json::Value>(
            &std::fs::read_to_string(p).unwrap_or_default(),
        ) {
            if let Some(id) = j.get("_RecordId_").and_then(|v| v.as_str()) {
                map.insert(id.to_ascii_lowercase(), p.clone());
            }
            if let Some(n) = j.get("_RecordName_").and_then(|v| v.as_str()) {
                map.insert(n.to_ascii_lowercase(), p.clone());
                map.insert(
                    n.strip_prefix("BuildingBlocks_Canvas.")
                        .unwrap_or(n)
                        .to_ascii_lowercase(),
                    p.clone(),
                );
            }
            if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                map.entry(stem.to_ascii_lowercase()).or_insert_with(|| p.clone());
            }
        }
    }
    eprintln!(
        "mfd_ir_dump: mirror loaded ({} keys) in {:.1}s; compiling IR…",
        map.len(),
        load_start.elapsed().as_secs_f32()
    );
    let canvas = std::env::args().nth(1).expect("canvas guid/name");
    let content = std::env::args().nth(2).expect("content guid/name");
    let filter = std::env::args().nth(3).unwrap_or_default().to_ascii_lowercase();
    let size = std::env::args().nth(4).unwrap_or_else(|| "1600x1200".into());
    let (w, h) = size
        .split_once('x')
        .and_then(|(a, b)| Some((a.parse().ok()?, b.parse().ok()?)))
        .unwrap_or((1600, 1200));
    let fetcher = Fs(map);
    let binding = UiBindingView {
        canvas_guid: Some(&canvas),
        content_canvas_guid: Some(&content),
        binding_kind: Some("mfd"),
        manufacturer_id: Some("drak"),
        helper_name: Some("mfd_ir_dump"),
        default_view_index: None,
        default_screen_slot: None,
        screen_name_loc_key: None,
        host_swf_path: None,
    };
    let inputs = PipelineInputs {
        binding: &binding,
        canvas_fetcher: &fetcher,
        swf_fetcher: &NoSwf,
        style_fetcher: &NoStyle,
        asset_fetcher: &NoAsset,
        target_size: (w, h),
        apply_postprocess: false,
        animation_sample_percent: Some(0.0),
        localization_map: None,
        loc_fetcher: None,
        derived_values: None,
    };
    let compile_start = std::time::Instant::now();
    let ir = starbreaker_ui::compile_ir_for_binding(&inputs).expect("compile");
    eprintln!(
        "mfd_ir_dump: IR compiled in {:.1}s",
        compile_start.elapsed().as_secs_f32()
    );
    println!("nodes: {}", ir.nodes.len());
    for n in &ir.nodes {
        if !filter.is_empty() && !n.name.to_ascii_lowercase().contains(&filter) {
            continue;
        }
        let r = &n.computed_rect;
        println!(
            "{} {} '{}' parent={:?} active={} rect=({:.0},{:.0},{:.0},{:.0}) alpha={} bg={:?} bg_token={:?}",
            n.id,
            n.node_type,
            n.name,
            n.parent_id,
            n.is_active,
            r.x,
            r.y,
            r.w,
            r.h,
            n.alpha,
            n.background_fill_colour,
            n.background_fill_colour_token,
        );
        if n.background_fill_alpha.is_some() {
            println!("    bg_alpha={:?}", n.background_fill_alpha);
        }
        if n.asset_ref.is_some() {
            println!("    asset={:?}", n.asset_ref);
        }
        if n.segmented_fill.is_some() {
            println!("    segmented={:?}", n.segmented_fill);
        }
        if n.icon_tint_colour.is_some() || n.icon_tint_colour_token.is_some() {
            println!("    icon_tint={:?} icon_tint_token={:?}", n.icon_tint_colour, n.icon_tint_colour_token);
        }
        if !n.resolved_style_tags.is_empty() {
            let tags: Vec<&str> = n
                .resolved_style_tags
                .iter()
                .filter_map(|t| t.tag_name.as_deref())
                .collect();
            println!("    tags={tags:?}");
        }
        if !n.children.is_empty() {
            println!("    children={:?}", n.children);
        }
    }
}
