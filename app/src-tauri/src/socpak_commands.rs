use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use starbreaker_datacore::database::Database;
use starbreaker_p4k::MappedP4k;

use crate::commands::{
    bundled_extension, collect_existing_decomposed_assets, decomposed_package_directory_name,
    prepare_decomposed_output_root, sanitize_export_name, sanitize_filename, write_decomposed_file,
};
use crate::error::AppError;
use crate::state::AppState;

#[derive(Clone, Serialize)]
pub struct SocpakDto {
    pub id: String,
    pub name: String,
    pub path: String,
    pub category: String,
    pub size: u64,
}

#[derive(Clone, Serialize)]
pub struct SocpakCategoryDto {
    pub name: String,
    pub socpaks: Vec<SocpakDto>,
}

#[derive(Clone, Serialize)]
pub struct SocpakExportProgress {
    pub current: usize,
    pub total: usize,
    pub fraction: f32,
    pub socpak_name: String,
    pub socpak_path: String,
    pub stage: String,
    pub error: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct SocpakExportDone {
    pub success: usize,
    pub errors: usize,
    pub succeeded_paths: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct SocpakExportRequest {
    pub socpak_paths: Vec<String>,
    pub output_dir: String,
    pub lod: u32,
    pub mip: u32,
    pub export_kind: String,
    pub material_mode: String,
    pub format: String,
    pub include_lights: bool,
    pub connected: bool,
    pub overwrite_existing_assets: bool,
    pub include_nodraw: bool,
    pub threads: usize,
}

#[tauri::command]
pub async fn scan_socpak_categories(
    state: State<'_, AppState>,
) -> Result<Vec<SocpakCategoryDto>, AppError> {
    let p4k = {
        let guard = state.p4k.lock();
        guard
            .as_ref()
            .ok_or_else(|| AppError::Internal("P4k not loaded".into()))?
            .clone()
    };

    tokio::task::spawn_blocking(move || {
        let mut grouped: BTreeMap<String, Vec<SocpakDto>> = BTreeMap::new();

        for entry in p4k.entries() {
            if !entry.name.to_ascii_lowercase().ends_with(".socpak") {
                continue;
            }

            let path = entry.name.replace('\\', "/");
            let category = socpak_category(&entry.name);
            let socpak = SocpakDto {
                id: path.clone(),
                name: socpak_display_name(&entry.name),
                path,
                category: category.clone(),
                size: entry.uncompressed_size,
            };
            grouped.entry(category).or_default().push(socpak);
        }

        let mut categories = grouped
            .into_iter()
            .map(|(name, mut socpaks)| {
                socpaks.sort_by(|a, b| {
                    a.name
                        .to_ascii_lowercase()
                        .cmp(&b.name.to_ascii_lowercase())
                        .then_with(|| {
                            a.path
                                .to_ascii_lowercase()
                                .cmp(&b.path.to_ascii_lowercase())
                        })
                });
                SocpakCategoryDto { name, socpaks }
            })
            .collect::<Vec<_>>();

        categories.sort_by(|a, b| {
            category_rank(&a.name)
                .cmp(&category_rank(&b.name))
                .then_with(|| a.name.cmp(&b.name))
        });

        Ok::<_, AppError>(categories)
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
}

#[tauri::command]
pub async fn start_socpak_export(
    app: AppHandle,
    state: State<'_, AppState>,
    request: SocpakExportRequest,
) -> Result<(), AppError> {
    if request.socpak_paths.is_empty() {
        return Err(AppError::Internal("No SOCPAKs selected".into()));
    }

    state.export_cancel.store(false, Ordering::SeqCst);

    let p4k = {
        let guard = state.p4k.lock();
        guard
            .as_ref()
            .ok_or_else(|| AppError::Internal("P4k not loaded".into()))?
            .clone()
    };
    let dcb_bytes = {
        let guard = state.dcb_bytes.lock();
        guard
            .as_ref()
            .ok_or_else(|| AppError::Internal("DataCore not loaded".into()))?
            .clone()
    };

    let cancel = state.export_cancel.clone();
    let opts = export_options_from_request(&request);
    let connected = request.connected;
    let overwrite_existing_assets = request.overwrite_existing_assets;
    let requested_threads = request.threads;
    let socpak_paths = request.socpak_paths;
    let output_dir = PathBuf::from(request.output_dir);

    tokio::task::spawn_blocking(move || {
        let total = socpak_paths.len();

        let db = match Database::from_bytes(&dcb_bytes) {
            Ok(db) => db,
            Err(error) => {
                emit_progress(
                    &app,
                    0,
                    total,
                    "",
                    "Failed to load DataCore",
                    Some(error.to_string()),
                );
                let _ = app.emit(
                    "socpak-export-done",
                    SocpakExportDone {
                        success: 0,
                        errors: total,
                        succeeded_paths: Vec::new(),
                    },
                );
                return;
            }
        };

        let existing_asset_paths =
            if opts.kind == starbreaker_3d::ExportKind::Decomposed && !overwrite_existing_assets {
                match collect_existing_decomposed_assets(&output_dir) {
                    Ok(paths) => Some(paths),
                    Err(error) => {
                        emit_progress(
                            &app,
                            0,
                            total,
                            "",
                            "Failed to inspect output directory",
                            Some(error.to_string()),
                        );
                        let _ = app.emit(
                            "socpak-export-done",
                            SocpakExportDone {
                                success: 0,
                                errors: total,
                                succeeded_paths: Vec::new(),
                            },
                        );
                        return;
                    }
                }
            } else {
                None
            };

        let success = AtomicUsize::new(0);
        let errors = AtomicUsize::new(0);
        let succeeded_paths = Mutex::new(Vec::new());
        let existing_asset_paths = existing_asset_paths.map(Arc::new);
        let num_threads = if requested_threads > 0 {
            requested_threads
        } else {
            (std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4)
                / 2)
            .max(2)
        };
        let pool = match rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
        {
            Ok(pool) => pool,
            Err(error) => {
                emit_progress(
                    &app,
                    0,
                    total,
                    "",
                    "Failed to start worker threads",
                    Some(error.to_string()),
                );
                let _ = app.emit(
                    "socpak-export-done",
                    SocpakExportDone {
                        success: 0,
                        errors: total,
                        succeeded_paths: Vec::new(),
                    },
                );
                return;
            }
        };

        pool.install(|| {
            use rayon::prelude::*;

            socpak_paths.par_iter().for_each(|socpak_path| {
                if cancel.load(Ordering::Relaxed) {
                    return;
                }

                let completed = success.load(Ordering::Relaxed) + errors.load(Ordering::Relaxed);
                emit_progress(&app, completed, total, socpak_path, "Preparing export", None);
                let existing = existing_asset_paths.as_deref();
                match export_single_socpak(
                    &db,
                    &p4k,
                    socpak_path,
                    &output_dir,
                    &opts,
                    connected,
                    overwrite_existing_assets,
                    existing,
                ) {
                    Ok(()) => {
                        let current = success.fetch_add(1, Ordering::Relaxed)
                            + errors.load(Ordering::Relaxed)
                            + 1;
                        succeeded_paths.lock().unwrap().push(socpak_path.clone());
                        emit_progress(&app, current, total, socpak_path, "Done", None);
                    }
                    Err(error) => {
                        let current = errors.fetch_add(1, Ordering::Relaxed)
                            + success.load(Ordering::Relaxed)
                            + 1;
                        emit_progress(
                            &app,
                            current,
                            total,
                            socpak_path,
                            "Failed",
                            Some(error.to_string()),
                        );
                    }
                }
            });
        });

        let _ = app.emit(
            "socpak-export-done",
            SocpakExportDone {
                success: success.load(Ordering::Relaxed),
                errors: errors.load(Ordering::Relaxed),
                succeeded_paths: succeeded_paths.into_inner().unwrap(),
            },
        );
    });

    Ok(())
}

fn emit_progress(
    app: &AppHandle,
    current: usize,
    total: usize,
    socpak_path: &str,
    stage: &str,
    error: Option<String>,
) {
    let fraction = if total == 0 {
        1.0
    } else {
        (current as f32 / total as f32).clamp(0.0, 1.0)
    };
    let _ = app.emit(
        "socpak-export-progress",
        SocpakExportProgress {
            current,
            total,
            fraction,
            socpak_name: socpak_display_name(socpak_path),
            socpak_path: socpak_path.replace('\\', "/"),
            stage: stage.to_string(),
            error,
        },
    );
}

fn export_single_socpak(
    db: &Database,
    p4k: &MappedP4k,
    socpak_path: &str,
    output_dir: &Path,
    opts: &starbreaker_3d::ExportOptions,
    connected: bool,
    overwrite_existing_assets: bool,
    existing_asset_paths: Option<&HashSet<String>>,
) -> Result<(), AppError> {
    std::fs::create_dir_all(output_dir)?;
    let roots = vec![socpak_path.to_string()];
    let export_name = socpak_export_name(socpak_path);

    match opts.kind {
        starbreaker_3d::ExportKind::Bundled => {
            let bytes = starbreaker_3d::socpaks_to_glb(db, p4k, &roots, opts)?;
            let filename = format!(
                "{}.{}",
                sanitize_filename(&export_name),
                bundled_extension(opts.format)
            );
            std::fs::write(output_dir.join(filename), bytes)?;
        }
        starbreaker_3d::ExportKind::Decomposed => {
            let result = starbreaker_3d::socpaks_to_decomposed(
                db,
                p4k,
                &roots,
                opts,
                starbreaker_3d::SocpakGraphOptions { connected },
                existing_asset_paths,
            )?;
            let fallback_package_name = format!(
                "{}_LOD{}_TEX{}",
                sanitize_export_name(&format!("socpak {export_name}")),
                opts.lod_level,
                opts.texture_mip,
            );
            let package_name =
                decomposed_package_directory_name(&result.export.files, &fallback_package_name);
            prepare_decomposed_output_root(output_dir, &package_name)?;
            for file in &result.export.files {
                let file_path = output_dir.join(&file.relative_path);
                if let Some(parent) = file_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let _ = write_decomposed_file(file, &file_path, overwrite_existing_assets)?;
            }
        }
    }

    Ok(())
}

fn export_options_from_request(request: &SocpakExportRequest) -> starbreaker_3d::ExportOptions {
    let kind = match request.export_kind.to_lowercase().as_str() {
        "decomposed" => starbreaker_3d::ExportKind::Decomposed,
        _ => starbreaker_3d::ExportKind::Bundled,
    };
    let material_mode = match request.material_mode.to_lowercase().as_str() {
        "none" => starbreaker_3d::MaterialMode::None,
        "colors" => starbreaker_3d::MaterialMode::Colors,
        "all" => starbreaker_3d::MaterialMode::All,
        _ => starbreaker_3d::MaterialMode::Textures,
    };
    let format = match request.format.to_lowercase().as_str() {
        "stl" => starbreaker_3d::ExportFormat::Stl,
        _ => starbreaker_3d::ExportFormat::Glb,
    };

    starbreaker_3d::ExportOptions {
        kind,
        format,
        material_mode,
        include_attachments: false,
        include_interior: true,
        include_lights: request.include_lights,
        include_nodraw: request.include_nodraw,
        include_shields: false,
        texture_mip: request.mip,
        lod_level: request.lod,
        include_animations: false,
        apply_default_animation_pose: false,
        default_animation_tags: Vec::new(),
    }
}

fn socpak_category(path: &str) -> String {
    let parts = path
        .split(['\\', '/'])
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let lower = parts
        .iter()
        .map(|part| part.to_ascii_lowercase())
        .collect::<Vec<_>>();

    let Some(index) = lower.iter().position(|part| part == "objectcontainers") else {
        return "Other".to_string();
    };
    let parts = &parts[index + 1..];
    let lower = &lower[index + 1..];

    if lower.is_empty() {
        return "Other".to_string();
    }

    match lower[0].as_str() {
        "pu" => categorize_pu_socpak(parts, lower),
        "ships" => "Ships".to_string(),
        "sm" => "Simulation".to_string(),
        "ea" => "Electronic Access".to_string(),
        _ => title_segment(parts[0]),
    }
}

fn categorize_pu_socpak(parts: &[&str], lower: &[String]) -> String {
    if lower.len() < 2 {
        return "PU".to_string();
    }

    match lower[1].as_str() {
        "loc" => "PU Locations".to_string(),
        "system" => "PU System".to_string(),
        "shops" => "PU Shops".to_string(),
        "missions" => "PU Missions".to_string(),
        "derelict" => "PU Derelicts".to_string(),
        "surfaceop" => "PU Surface Ops".to_string(),
        "asteroid" | "asteroidcluster" => "PU Asteroids".to_string(),
        "modular" => "PU Modular".to_string(),
        "design" => "PU Design".to_string(),
        _ => format!("PU {}", title_segment(parts[1])),
    }
}

fn category_rank(category: &str) -> usize {
    match category {
        "PU Locations" => 0,
        "PU System" => 1,
        "PU Shops" => 2,
        "PU Missions" => 3,
        "PU Derelicts" => 4,
        "PU Surface Ops" => 5,
        "PU Asteroids" => 6,
        "PU Modular" => 7,
        "PU Design" => 8,
        "Ships" => 9,
        "Simulation" => 10,
        "Electronic Access" => 11,
        "Other" => 99,
        _ => 50,
    }
}

fn title_segment(segment: &str) -> String {
    let words = segment
        .split(['_', '-', ' '])
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();

    if words.is_empty() {
        return segment.to_string();
    }

    words
        .into_iter()
        .map(|word| {
            let mut chars = word.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };
            format!(
                "{}{}",
                first.to_ascii_uppercase(),
                chars.as_str().to_ascii_lowercase()
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn socpak_display_name(path: &str) -> String {
    let file_name = path.rsplit(['\\', '/']).next().unwrap_or(path).to_string();
    let lower = file_name.to_ascii_lowercase();
    if lower.ends_with(".socpak") {
        file_name[..file_name.len() - ".socpak".len()].to_string()
    } else {
        file_name
    }
}

fn socpak_export_name(path: &str) -> String {
    sanitize_export_name(&socpak_display_name(path))
}

#[cfg(test)]
mod tests {
    use super::{socpak_category, socpak_display_name, title_segment};

    #[test]
    fn socpak_category_groups_known_object_container_families() {
        assert_eq!(
            socpak_category(
                r"Data\ObjectContainers\PU\loc\flagship\stanton\orison\orison_ind.socpak",
            ),
            "PU Locations"
        );
        assert_eq!(
            socpak_category(r"Data\ObjectContainers\PU\Shops\admin\admin_base.socpak"),
            "PU Shops"
        );
        assert_eq!(
            socpak_category(r"Data\ObjectContainers\Ships\AEGS\Javelin\javelin.socpak"),
            "Ships"
        );
    }

    #[test]
    fn socpak_category_falls_back_to_readable_path_segment() {
        assert_eq!(
            socpak_category(r"Data\ObjectContainers\PU\surfaceop\ht\delta\a.socpak"),
            "PU Surface Ops"
        );
        assert_eq!(
            socpak_category(r"Data\ObjectContainers\Sandbox\foo.socpak"),
            "Sandbox"
        );
        assert_eq!(socpak_category(r"Data\Objects\foo.socpak"), "Other");
    }

    #[test]
    fn socpak_display_name_strips_path_and_extension() {
        assert_eq!(
            socpak_display_name(r"Data\ObjectContainers\PU\loc\orison_ind_lz_int.socpak"),
            "orison_ind_lz_int"
        );
    }

    #[test]
    fn title_segment_formats_underscore_and_dash_names() {
        assert_eq!(title_segment("asteroidCluster"), "Asteroidcluster");
        assert_eq!(title_segment("surface-op_large"), "Surface Op Large");
    }
}
