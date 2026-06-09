//! Host Flash stage→target text scale for screen bindings.
//!
//! The engine hosts MFD frame canvases inside the binding's Flash movie (e.g.
//! `BuildingBlocks_root.swf`, a 1280×720 GFx stage) and renders that stage onto
//! the screen RTT with NoBorder/cover scaling. Geometry reflows through the BB
//! layout, but textfield font sizes are stage-unit values, so they pick up the
//! stage→target view scale. [`host_stage_text_scale`] derives that scale from
//! the binding's host movie header.

use super::SwfFetcher;

/// The GFx NoBorder/cover scale from the binding's host Flash stage to the
/// render target: `max(target_w/stage_w, target_h/stage_h)`. Returns 1.0 when
/// the binding carries no host movie or its stage is degenerate, so text
/// renders at its design size unchanged.
///
/// Verified against the Clipper target/power screen captures: H1 60 on the
/// 1280×720 `BuildingBlocks_root.swf` stage renders at 60 × 1200/720 = 100
/// target px on the 1600×1200 MFD RTT.
pub(super) fn host_stage_text_scale(
    host_swf_path: Option<&str>,
    swf_fetcher: &dyn SwfFetcher,
    target_size: (u32, u32),
) -> f32 {
    let Some(path) = host_swf_path.filter(|p| !p.is_empty()) else {
        return 1.0;
    };
    let Ok(bytes) = swf_fetcher.fetch_swf_bytes(path) else {
        log::debug!("host stage scale: could not fetch host movie '{path}'; text unscaled");
        return 1.0;
    };
    let Ok(library) = crate::swf_assets::SwfAssetLibrary::new(bytes) else {
        log::debug!("host stage scale: could not parse host movie '{path}'; text unscaled");
        return 1.0;
    };
    let (stage_w, stage_h) = library.stage_size();
    if !(stage_w.is_finite() && stage_h.is_finite()) || stage_w <= 0.0 || stage_h <= 0.0 {
        return 1.0;
    }
    let scale = (target_size.0 as f32 / stage_w).max(target_size.1 as f32 / stage_h);
    if scale.is_finite() && scale > 0.0 { scale } else { 1.0 }
}
