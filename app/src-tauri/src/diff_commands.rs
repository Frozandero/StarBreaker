use std::path::PathBuf;
use std::sync::atomic::Ordering;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::error::AppError;
use crate::state::AppState;

#[derive(Clone, Serialize)]
pub struct DiffInventoryProgress {
    pub job_id: String,
    pub phase: String,
    pub current: Option<usize>,
    pub total: Option<usize>,
    pub message: String,
}

#[derive(Clone, Serialize)]
pub struct DiffInventoryHandle {
    pub id: String,
    pub label: String,
    pub mode: starbreaker_diff::InventoryMode,
    pub path_hint: Option<String>,
    pub archive_count: usize,
    pub datacore_count: usize,
    pub inventory_hash: String,
    pub warnings: Vec<String>,
}

#[tauri::command]
pub async fn diff_generate_inventory(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    skip_datacore: bool,
    label: Option<String>,
    job_id: String,
) -> Result<DiffInventoryHandle, AppError> {
    state.diff_cancel.store(false, Ordering::Relaxed);
    let cancel = state.diff_cancel.clone();
    let app_clone = app.clone();
    let options = starbreaker_diff::InventoryOptions {
        skip_datacore,
        label,
        ..Default::default()
    };
    let report = tokio::task::spawn_blocking(move || {
        let mut progress = |event: starbreaker_diff::ProgressEvent| {
            let _ = app_clone.emit("diff-inventory-progress", DiffInventoryProgress {
                job_id: job_id.clone(),
                phase: format!("{:?}", event.phase),
                current: event.current,
                total: event.total,
                message: event.message,
            });
        };
        starbreaker_diff::generate_inventory_from_p4k_with_progress(
            PathBuf::from(path),
            &options,
            Some(&mut progress),
            Some(cancel.as_ref()),
        )
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    .map_err(AppError::from)?;
    let id = new_inventory_id();
    let handle = inventory_handle(&id, &report);
    log::info!(
        "diff inventory ready: id={} label={} mode={:?} archive_count={} datacore_count={} hash={}",
        handle.id,
        handle.label,
        handle.mode,
        handle.archive_count,
        handle.datacore_count,
        handle.inventory_hash
    );
    state.diff_inventories.lock().insert(id, report);
    Ok(handle)
}

#[tauri::command]
pub fn diff_cancel_inventory(state: State<'_, AppState>) {
    state.diff_cancel.store(true, Ordering::Relaxed);
}

#[tauri::command]
pub fn diff_load_inventory_report(
    state: State<'_, AppState>,
    path: String,
) -> Result<DiffInventoryHandle, AppError> {
    let report = starbreaker_diff::report::read_inventory_report(path)?;
    let id = new_inventory_id();
    let handle = inventory_handle(&id, &report);
    log::info!(
        "diff inventory loaded: id={} label={} mode={:?} archive_count={} datacore_count={} hash={}",
        handle.id,
        handle.label,
        handle.mode,
        handle.archive_count,
        handle.datacore_count,
        handle.inventory_hash
    );
    state.diff_inventories.lock().insert(id, report);
    Ok(handle)
}

#[tauri::command]
pub fn diff_save_inventory_report(
    state: State<'_, AppState>,
    path: String,
    id: String,
) -> Result<(), AppError> {
    let inventories = state.diff_inventories.lock();
    let report = inventories
        .get(&id)
        .ok_or_else(|| AppError::Internal(format!("inventory not found: {id}")))?;
    Ok(starbreaker_diff::report::write_inventory_report(path, report)?)
}

#[tauri::command]
pub fn diff_compare_reports(
    state: State<'_, AppState>,
    old_id: String,
    new_id: String,
    include_unchanged: bool,
    filter: Option<starbreaker_diff::DiffFilter>,
    max_items: Option<usize>,
) -> Result<starbreaker_diff::DiffReport, AppError> {
    let inventories = state.diff_inventories.lock();
    let old = inventories
        .get(&old_id)
        .ok_or_else(|| AppError::Internal(format!("old inventory not found: {old_id}")))?;
    let new = inventories
        .get(&new_id)
        .ok_or_else(|| AppError::Internal(format!("new inventory not found: {new_id}")))?;
    log::info!(
        "diff compare started: old_id={} old_label={} new_id={} new_label={} include_unchanged={} filter={:?} max_items={:?}",
        old_id,
        old.source.label,
        new_id,
        new.source.label,
        include_unchanged,
        filter,
        max_items
    );
    let mut report = starbreaker_diff::compare_reports(old, new, include_unchanged);
    let unfiltered_items = report.items.len();
    if let Some(filter) = filter {
        report.items = starbreaker_diff::filter_diff_items(&report.items, &filter)
            .into_iter()
            .cloned()
            .collect();
    }
    let filtered_items = report.items.len();
    let max_items = max_items.unwrap_or(5000);
    if report.items.len() > max_items {
        report.items.truncate(max_items);
    }
    log::info!(
        "diff compare finished: old_id={} new_id={} summary_added={} summary_removed={} summary_modified={} summary_metadata={} summary_unchanged={} unfiltered_items={} filtered_items={} returned_items={}",
        old_id,
        new_id,
        report.summary.added,
        report.summary.removed,
        report.summary.modified,
        report.summary.metadata_changed,
        report.summary.unchanged,
        unfiltered_items,
        filtered_items,
        report.items.len()
    );
    Ok(report)
}

#[tauri::command]
pub fn diff_query_report_page(
    state: State<'_, AppState>,
    old_id: String,
    new_id: String,
    filter: starbreaker_diff::DiffFilter,
    offset: usize,
    limit: usize,
) -> Result<starbreaker_diff::DiffPage, AppError> {
    let inventories = state.diff_inventories.lock();
    let old = inventories
        .get(&old_id)
        .ok_or_else(|| AppError::Internal(format!("old inventory not found: {old_id}")))?;
    let new = inventories
        .get(&new_id)
        .ok_or_else(|| AppError::Internal(format!("new inventory not found: {new_id}")))?;
    log::info!(
        "diff page query started: old_id={} old_label={} new_id={} new_label={} filter={:?} offset={} limit={}",
        old_id,
        old.source.label,
        new_id,
        new.source.label,
        filter,
        offset,
        limit
    );
    let page = starbreaker_diff::compare_report_page(old, new, &filter, offset, limit);
    log::info!(
        "diff page query finished: old_id={} new_id={} total_matching={} returned_items={} summary_added={} summary_removed={} summary_modified={} summary_metadata={} summary_unchanged={}",
        old_id,
        new_id,
        page.total_matching,
        page.items.len(),
        page.summary.added,
        page.summary.removed,
        page.summary.modified,
        page.summary.metadata_changed,
        page.summary.unchanged
    );
    Ok(page)
}

#[tauri::command]
pub fn diff_save_diff_report(
    path: String,
    report: starbreaker_diff::DiffReport,
) -> Result<(), AppError> {
    Ok(starbreaker_diff::report::write_diff_report(path, &report)?)
}

fn new_inventory_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("inv-{nanos}")
}

fn inventory_handle(id: &str, report: &starbreaker_diff::InventoryReport) -> DiffInventoryHandle {
    DiffInventoryHandle {
        id: id.to_string(),
        label: report.source.label.clone(),
        mode: report.mode,
        path_hint: report
            .source
            .source_file
            .as_ref()
            .map(|source| source.path_hint.clone()),
        archive_count: report.archive.len(),
        datacore_count: report.datacore.status.records().len(),
        inventory_hash: report.inventory_hash.clone(),
        warnings: report.source.warnings.clone(),
    }
}
