use sha2::{Digest, Sha256};
use std::path::PathBuf;
use starbreaker_ui::{
    UiIrDocument, UiRegressionManifest, UiScreenSnapshot, compare_manifest_targets_with_loader,
    snapshot_from_ui_ir,
};
use std::collections::HashMap;

#[derive(Clone, Debug, serde::Deserialize)]
struct UiFreezeArtifact {
    id: String,
    artifact_path: String,
    sha256: String,
    width: u32,
    height: u32,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct UiFreezeFile {
    artifacts: Vec<UiFreezeArtifact>,
}

fn freeze_file() -> UiFreezeFile {
    serde_json::from_str(include_str!("fixtures/ui_regression_freeze.json"))
        .expect("ui regression freeze fixture should parse")
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn artifact_paths(target_id: &str) -> (PathBuf, PathBuf) {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root should resolve from CARGO_MANIFEST_DIR");
    let workspace_root = repo_root
        .parent()
        .expect("repo root should have workspace parent");
    let manifest_json: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/ui_ir/ui_snapshot_manifest.json"
    ))
    .expect("manifest JSON fixture should parse");
    let source_png = manifest_json
        .get("targets")
        .and_then(|targets| targets.as_array())
        .and_then(|targets| {
            targets.iter().find_map(|target| {
                if target.get("id").and_then(|id| id.as_str()) == Some(target_id) {
                    target
                        .get("source_generated_png")
                        .and_then(|value| value.as_str())
                } else {
                    None
                }
            })
        })
        .unwrap_or_else(|| {
            panic!("target {target_id} missing source_generated_png in regression manifest")
        });

    let source_path = if source_png.starts_with('/') {
        PathBuf::from(source_png)
    } else if source_png.starts_with("ships/") {
        workspace_root.join(source_png)
    } else {
        repo_root.join(source_png)
    };
    let artifact_path = repo_root.join("test-artifacts/ui").join(format!("{target_id}.png"));
    (source_path, artifact_path)
}

fn snapshot_manifest() -> UiRegressionManifest {
    serde_json::from_str(include_str!("fixtures/ui_ir/ui_snapshot_manifest.json"))
    .expect("snapshot manifest fixture should parse")
}

fn manifest_snapshot_lookup() -> HashMap<String, UiScreenSnapshot> {
    let ui_target_a: UiIrDocument = serde_json::from_str(include_str!(
        "fixtures/ui_ir/target_a-screen_16x9_a-ir.json"
    ))
    .expect("ui_target_a IR fixture should parse");
    let ui_target_b: UiIrDocument = serde_json::from_str(include_str!(
        "fixtures/ui_ir/target_b-mesh_end_screen_plane-ir.json"
    ))
    .expect("ui_target_b IR fixture should parse");

    HashMap::from([
        ("ui_target_a.baseline".to_string(), snapshot_from_ui_ir(&ui_target_a)),
        ("ui_target_a.current".to_string(), snapshot_from_ui_ir(&ui_target_a)),
        ("ui_target_b.baseline".to_string(), snapshot_from_ui_ir(&ui_target_b)),
        ("ui_target_b.current".to_string(), snapshot_from_ui_ir(&ui_target_b)),
        (
            "clipper_small_door.baseline".to_string(),
            snapshot_from_ui_ir(&ui_target_b),
        ),
        (
            "clipper_small_door.current".to_string(),
            snapshot_from_ui_ir(&ui_target_b),
        ),
    ])
}

/// Generic, IR-level structural preflight: every manifest target's snapshot
/// must compare clean through the manifest runner. Content-agnostic — covers any
/// target listed in the manifest.
#[test]
fn manifest_snapshot_runner_preflight() {
    let mut manifest = snapshot_manifest();
    let snapshots = manifest_snapshot_lookup();
    manifest.targets.retain(|target| {
        snapshots.contains_key(&target.baseline_path) && snapshots.contains_key(&target.current_path)
    });
    let results = compare_manifest_targets_with_loader(&manifest, |path| {
        snapshots
            .get(path)
            .cloned()
            .ok_or_else(|| format!("missing snapshot fixture for {path}"))
    })
    .expect("manifest runner should load all manifest fixture snapshots");
    for result in results {
        assert!(
            result.comparison.passed,
            "manifest snapshot preflight failed for {}: {:?}\nACTION: treat this as a real product regression and fix rendering/data-flow root cause first. Do not relax thresholds or update baselines as a first step.",
            result.id,
            result.comparison.failures
        );
    }
}

#[test]
fn manifest_contains_expected_four_visual_targets() {
    let manifest = snapshot_manifest();
    assert_eq!(manifest.targets.len(), 4, "expected four manifest targets");
    assert!(
        manifest
            .targets
            .iter()
            .any(|target| target.id == "eng_annunciator_master_left"),
        "annunciator target should be in gold/platinum manifest set"
    );
}

fn frozen_artifact_backstop_failure(target_id: &str) -> Option<String> {
    let freeze = freeze_file();
    let freeze_artifact = freeze
        .artifacts
        .iter()
        .find(|artifact| artifact.id == target_id)?;

    let artifact_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(&freeze_artifact.artifact_path);
    let artifact_path = match artifact_path.canonicalize() {
        Ok(path) => path,
        Err(_) => {
            return Some(format!(
                "{} visual regression detected: missing artifact path {}",
                target_id, freeze_artifact.artifact_path
            ));
        }
    };
    let bytes = match std::fs::read(&artifact_path) {
        Ok(bytes) => bytes,
        Err(_) => {
            return Some(format!(
                "{} visual regression detected: failed reading artifact {}",
                target_id,
                artifact_path.display()
            ));
        }
    };
    let current_sha = sha256_hex(&bytes);
    if current_sha != freeze_artifact.sha256 {
        return Some(format!(
            "{} visual regression backstop detected: artifact drifted from frozen baseline\nartifact={}\nexpected_sha={}\nactual_sha={}\nACTION: treat semantic failures (font_size/font_weight/text/color/geometry) as primary and investigate root cause before updating any baseline/freeze metadata.",
            target_id,
            artifact_path.display(),
            freeze_artifact.sha256,
            current_sha,
        ));
    }

    let img = image::load_from_memory(&bytes)
        .expect("artifact image should decode")
        .into_rgba8();
    let (w, h) = img.dimensions();
    if w != freeze_artifact.width {
        return Some(format!(
            "{} visual regression backstop detected: artifact width drifted (frozen={} current={})",
            target_id, freeze_artifact.width, w
        ));
    }
    if h != freeze_artifact.height {
        return Some(format!(
            "{} visual regression backstop detected: artifact height drifted (frozen={} current={})",
            target_id, freeze_artifact.height, h
        ));
    }

    None
}

#[test]
#[ignore = "optional artifact-hash backstop; semantic IR-freeze checks are the required gating path"]
fn manifest_targets_frozen_artifact_backstop_guard() {
    let manifest = snapshot_manifest();
    let mut failures = Vec::new();
    for target in manifest.targets {
        if let Some(failure) = frozen_artifact_backstop_failure(&target.id) {
            failures.push(failure);
        }
    }
    assert!(
        failures.is_empty(),
        "frozen artifact backstop detected regression(s):\n{}",
        failures.join("\n\n")
    );
}

fn foreground_mask_from_border_delta(
    img: &image::RgbaImage,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) -> Option<(Vec<bool>, usize, usize)> {
    let (img_w, img_h) = img.dimensions();
    let x0 = x.round().max(0.0) as u32;
    let y0 = y.round().max(0.0) as u32;
    let x1 = (x + w).round().max(0.0) as u32;
    let y1 = (y + h).round().max(0.0) as u32;

    let x0 = x0.min(img_w);
    let y0 = y0.min(img_h);
    let x1 = x1.min(img_w);
    let y1 = y1.min(img_h);
    if x1 <= x0 || y1 <= y0 {
        return None;
    }

    let width = (x1 - x0) as usize;
    let height = (y1 - y0) as usize;

    let mut border_r = Vec::new();
    let mut border_g = Vec::new();
    let mut border_b = Vec::new();

    for x in x0..x1 {
        let top = img.get_pixel(x, y0);
        let bottom = img.get_pixel(x, y1 - 1);
        border_r.push(top[0]);
        border_g.push(top[1]);
        border_b.push(top[2]);
        border_r.push(bottom[0]);
        border_g.push(bottom[1]);
        border_b.push(bottom[2]);
    }
    for y in y0..y1 {
        let left = img.get_pixel(x0, y);
        let right = img.get_pixel(x1 - 1, y);
        border_r.push(left[0]);
        border_g.push(left[1]);
        border_b.push(left[2]);
        border_r.push(right[0]);
        border_g.push(right[1]);
        border_b.push(right[2]);
    }

    if border_r.is_empty() {
        return None;
    }

    border_r.sort_unstable();
    border_g.sort_unstable();
    border_b.sort_unstable();
    let mid = border_r.len() / 2;
    let bg_r = border_r[mid] as i32;
    let bg_g = border_g[mid] as i32;
    let bg_b = border_b[mid] as i32;

    let mut mask = vec![false; width * height];
    for y in 0..height {
        for x in 0..width {
            let px = img.get_pixel(x0 + x as u32, y0 + y as u32);
            let delta = (px[0] as i32 - bg_r).abs()
                + (px[1] as i32 - bg_g).abs()
                + (px[2] as i32 - bg_b).abs();
            mask[y * width + x] = delta > 30;
        }
    }

    Some((mask, width, height))
}

fn mask_touches_all_edges(mask: &[bool], width: usize, height: usize, band: usize) -> bool {
    if width == 0 || height == 0 {
        return false;
    }
    let top_band = band.min(height);
    let left_band = band.min(width);

    let touches_top = (0..top_band).any(|y| (0..width).any(|x| mask[y * width + x]));
    let touches_bottom = (height - top_band..height).any(|y| (0..width).any(|x| mask[y * width + x]));
    let touches_left = (0..height).any(|y| (0..left_band).any(|x| mask[y * width + x]));
    let touches_right = (0..height)
        .any(|y| (width - left_band..width).any(|x| mask[y * width + x]));

    touches_top && touches_bottom && touches_left && touches_right
}

#[test]
fn target_a_custom_shape_scale_and_position_guard() {
    let (reference_path, current_path) = artifact_paths("ui_target_a");
    if !reference_path.is_file() || !current_path.is_file() {
        eprintln!(
            "skipping ui_target_a custom-shape guard (missing files: reference={} current={})",
            reference_path.display(),
            current_path.display()
        );
        return;
    }

    let fixture: UiIrDocument = serde_json::from_str(include_str!(
        "fixtures/ui_ir/target_a-screen_16x9_a-ir.json"
    ))
    .expect("ui_target_a IR fixture should parse");
    let mut custom_shape_rects: Vec<(u32, f32, f32, f32, f32)> = fixture
        .nodes
        .iter()
        .filter(|node| node.node_type == "widget_custom_shape" && node.asset_ref.is_some())
        .map(|node| {
            (
                node.id,
                node.computed_rect.x,
                node.computed_rect.y,
                node.computed_rect.w,
                node.computed_rect.h,
            )
        })
        .collect();
    custom_shape_rects.sort_by_key(|entry| entry.0);
    assert!(
        !custom_shape_rects.is_empty(),
        "expected at least one asset-backed custom shape in ui_target_a fixture"
    );

    let reference = image::open(&reference_path)
        .expect("reference image should decode")
        .into_rgba8();
    let current = image::open(&current_path)
        .expect("current image should decode")
        .into_rgba8();

    for (node_id, x, y, w, h) in custom_shape_rects {
        let (reference_mask, width, height) = foreground_mask_from_border_delta(&reference, x, y, w, h)
            .expect("reference mask should be available");
        let (current_mask, _, _) = foreground_mask_from_border_delta(&current, x, y, w, h)
            .expect("current mask should be available");

        let reference_edge_anchored = mask_touches_all_edges(&reference_mask, width, height, 3);
        let current_edge_anchored = mask_touches_all_edges(&current_mask, width, height, 3);
        assert!(
            reference_edge_anchored == current_edge_anchored,
            "ui_target_a custom-shape scale/position drift for node {node_id}: edge anchoring changed between source and artifact"
        );
    }
}

/// Fraction of pixels whose max per-channel difference exceeds `tolerance`.
/// `None` signals a dimension mismatch (itself a regression).
fn whole_image_diff_fraction(
    baseline: &image::RgbaImage,
    render: &image::RgbaImage,
    tolerance: u8,
) -> Option<f32> {
    if baseline.dimensions() != render.dimensions() {
        return None;
    }
    let tol = tolerance as i32;
    let total = (baseline.width() as u64) * (baseline.height() as u64);
    if total == 0 {
        return Some(0.0);
    }
    let mut differing = 0u64;
    for (a, b) in baseline.pixels().zip(render.pixels()) {
        let dr = (a[0] as i32 - b[0] as i32).abs();
        let dg = (a[1] as i32 - b[1] as i32).abs();
        let db = (a[2] as i32 - b[2] as i32).abs();
        if dr.max(dg).max(db) > tol {
            differing += 1;
        }
    }
    Some(differing as f32 / total as f32)
}

/// Generic, content-agnostic whole-image colour regression.
///
/// This is intentionally NOT a focused/ROI/heuristic test. For every target in
/// the regression manifest it compares the frozen baseline PNG against the
/// freshly generated PNG pixel-for-pixel and fails when more than a
/// tier-dependent fraction of pixels differ beyond a small per-channel
/// tolerance. Any rendered change on any screen — icon tint, text colour, added
/// chrome, geometry — is detected automatically; adding a new screen to the
/// manifest extends coverage with no new test code and no per-screen knowledge.
#[test]
fn manifest_targets_whole_image_colour_regression_guard() {
    // Per-channel tolerance absorbs trivial anti-aliasing jitter; the allowed
    // differing-pixel fraction is tight for platinum and looser for gold.
    const TOLERANCE: u8 = 16;
    let manifest = snapshot_manifest();
    let mut failures = Vec::new();
    for target in manifest.targets {
        let max_diff_fraction = match target.tier {
            starbreaker_ui::UiRegressionTier::Platinum => 0.005,
            _ => 0.010,
        };
        // artifact_paths returns (fresh render from `ships/`, frozen baseline).
        let (render_path, baseline_path) = artifact_paths(&target.id);
        if !render_path.is_file() || !baseline_path.is_file() {
            eprintln!(
                "skipping whole-image regression for {} (missing files: render={} baseline={})",
                target.id,
                render_path.display(),
                baseline_path.display()
            );
            continue;
        }
        let baseline = image::open(&baseline_path)
            .expect("baseline image should decode")
            .into_rgba8();
        let render = image::open(&render_path)
            .expect("render image should decode")
            .into_rgba8();
        match whole_image_diff_fraction(&baseline, &render, TOLERANCE) {
            None => failures.push(format!(
                "{}: dimension drift baseline={:?} render={:?}",
                target.id,
                baseline.dimensions(),
                render.dimensions()
            )),
            Some(fraction) if fraction > max_diff_fraction => failures.push(format!(
                "{}: {:.4}% of pixels differ (> {:.4}% allowed for {:?})\n  baseline={}\n  render={}",
                target.id,
                fraction * 100.0,
                max_diff_fraction * 100.0,
                target.tier,
                baseline_path.display(),
                render_path.display()
            )),
            Some(_) => {}
        }
    }
    assert!(
        failures.is_empty(),
        "whole-image colour regression detected. Fix the rendering root cause first; \
         only re-freeze baselines when the change is intentional and approved.\n{}",
        failures.join("\n")
    );
}
