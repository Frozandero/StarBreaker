//! Canvas style-entry projection for the pipeline: applies the root canvas's
//! own style entries onto the resolved scene after graph resolution (split
//! from `pipeline/mod.rs` for the line cap).

use crate::bb_brand_apply::{apply_brand_modifiers, apply_scene_style_entries};
use crate::bb_brand_style::resolve_brand_style;

pub(super) fn project_canvas_style_entries(
    scene: &mut crate::bb_scene::BbScene,
    raw_root_json: &serde_json::Value,
    manufacturer_id: Option<&str>,
    loc_fetcher: Option<&dyn crate::bb_loc::LocFetcher>,
) {
    let record_value = raw_root_json.get("_RecordValue_").unwrap_or(raw_root_json);
    let selected_brand = resolve_brand_style(raw_root_json, manufacturer_id, None);
    let palette_source = selected_brand.map(|brand| brand.raw).unwrap_or(record_value);

    if let Some(default_entries) = record_value
        .get("defaultStyles")
        .and_then(|styles| styles.get("entries"))
        .and_then(|entries| entries.as_array())
    {
        apply_scene_style_entries(scene, default_entries, palette_source, loc_fetcher);
    }

    if let Some(brand) = resolve_brand_style(raw_root_json, manufacturer_id, None) {
        apply_brand_modifiers(scene, &brand, loc_fetcher);
    }
}

