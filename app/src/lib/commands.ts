import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface DiscoverResult {
  path: string;
  source: string;
}

export interface FileDirEntry {
  kind: "file";
  name: string;
  compressed_size: number;
  uncompressed_size: number;
}

export interface DirectoryDirEntry {
  kind: "directory";
  name: string;
}

export type DirEntry = FileDirEntry | DirectoryDirEntry;

export interface LoadProgress {
  fraction: number;
  message: string;
}

export interface SystemPalette {
  scheme: string;
  background: string;
  foreground: string;
  accent: string;
  success: string;
  warning: string;
  danger: string;
}

/** Get the OS system theme (dark/light, accent, palette). */
export async function getSystemTheme(): Promise<SystemPalette> {
  return invoke<SystemPalette>("get_system_theme");
}

/** Listen for OS theme changes. */
export function onSystemThemeChanged(
  callback: (palette: SystemPalette) => void,
): Promise<UnlistenFn> {
  return listen<SystemPalette>("system-theme-changed", (event) => {
    callback(event.payload);
  });
}

/** Discover all Data.p4k installations across channels. */
export async function discoverP4k(): Promise<DiscoverResult[]> {
  return invoke<DiscoverResult[]>("discover_p4k");
}

export interface P4kInfo {
  entry_count: number;
  total_bytes: number;
}

/** Open a P4k file and load it into the backend. */
export async function openP4k(path: string): Promise<P4kInfo> {
  return invoke<P4kInfo>("open_p4k", { path });
}

/** List directory contents from the loaded P4k. */
export async function listDir(path: string): Promise<DirEntry[]> {
  return invoke<DirEntry[]>("list_dir", { path });
}

/** List only subdirectory names under a path (fast). */
export async function listSubdirs(path: string): Promise<string[]> {
  return invoke<string[]>("list_subdirs", { path });
}

/** Open a file picker for Data.p4k. Returns the selected path or null. */
export async function browseP4k(): Promise<string | null> {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const result = await open({
    title: "Select Data.p4k",
    filters: [{ name: "P4K Archive", extensions: ["p4k"] }],
    multiple: false,
    directory: false,
  });
  return result ?? null;
}

/** Listen for progress events during P4k loading. */
export function onLoadProgress(
  callback: (progress: LoadProgress) => void,
): Promise<UnlistenFn> {
  return listen<LoadProgress>("load-progress", (event) => {
    callback(event.payload);
  });
}

// ── Export types ──

export interface EntityDto {
  name: string;
  id: string;
  display_name: string | null;
  is_npc_or_internal: boolean;
}

export interface CategoryDto {
  name: string;
  entities: EntityDto[];
}

export interface ExportRequest {
  record_ids: string[];
  names: string[];
  output_dir: string;
  lod: number;
  mip: number;
  export_kind: string;
  material_mode: string;
  format: string;
  include_attachments: boolean;
  include_interior: boolean;
  include_lights: boolean;
  threads: number;
  overwrite_existing_assets: boolean;
  include_nodraw: boolean;
  include_animations: boolean;
}

export interface ExportProgress {
  current: number;
  total: number;
  fraction: number;
  entity_name: string;
  entity_id: string;
  stage: string;
  error: string | null;
}

export interface ExportDone {
  success: number;
  errors: number;
  succeeded_ids: string[];
}

// ── Export commands ──

/** Scan DataCore for entity categories. Requires P4k to be loaded. */
export async function scanCategories(): Promise<CategoryDto[]> {
  return invoke<CategoryDto[]>("scan_categories");
}

/** Start batch export. Progress reported via events. */
export async function startExport(request: ExportRequest): Promise<void> {
  return invoke<void>("start_export", { request });
}

/** Cancel an in-progress export. */
export async function cancelExport(): Promise<void> {
  return invoke<void>("cancel_export");
}

/** Listen for export progress events. */
export function onExportProgress(
  callback: (progress: ExportProgress) => void,
): Promise<UnlistenFn> {
  return listen<ExportProgress>("export-progress", (event) => {
    callback(event.payload);
  });
}

/** Listen for export completion. */
export function onExportDone(
  callback: (result: ExportDone) => void,
): Promise<UnlistenFn> {
  return listen<ExportDone>("export-done", (event) => {
    callback(event.payload);
  });
}

/** Open a folder picker for the export output directory. */
export async function browseOutputDir(): Promise<string | null> {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const result = await open({
    title: "Select output directory",
    directory: true,
    multiple: false,
  });
  return result ?? null;
}

// ── DataCore types ──

export interface SearchResultDto {
  name: string;
  struct_type: string;
  path: string;
  id: string;
}

export interface TreeFolderEntry {
  kind: "folder";
  name: string;
}

export interface TreeRecordEntry {
  kind: "record";
  name: string;
  struct_type: string;
  id: string;
}

export type TreeEntryDto = TreeFolderEntry | TreeRecordEntry;

export interface RecordDto {
  name: string;
  struct_type: string;
  path: string;
  id: string;
  json: string;
}

// ── DataCore commands ──

/** Search records by name substring. Returns up to 500 results. */
export async function dcSearch(query: string): Promise<SearchResultDto[]> {
  return invoke<SearchResultDto[]>("dc_search", { query });
}

/** List tree entries (folders + records) at a given path. */
export async function dcListTree(path: string): Promise<TreeEntryDto[]> {
  return invoke<TreeEntryDto[]>("dc_list_tree", { path });
}

/** Get a record's full data for the property inspector. */
export async function dcGetRecord(recordId: string): Promise<RecordDto> {
  return invoke<RecordDto>("dc_get_record", { recordId });
}

/** Export a record as JSON, saving to the given path. */
export async function dcExportJson(recordId: string, outputPath: string): Promise<void> {
  return invoke<void>("dc_export_json", { recordId, outputPath });
}

/** Export a record as XML, saving to the given path. */
export async function dcExportXml(recordId: string, outputPath: string): Promise<void> {
  return invoke<void>("dc_export_xml", { recordId, outputPath });
}

/** Export all records under a folder path. Returns count of exported records. */
export async function dcExportFolder(
  pathPrefix: string,
  format: "json" | "xml",
  outputDir: string,
): Promise<number> {
  return invoke<number>("dc_export_folder", { pathPrefix, format, outputDir });
}

export interface BacklinkDto {
  name: string;
  id: string;
}

/** Get records that reference the given record. */
export async function dcGetBacklinks(recordId: string): Promise<BacklinkDto[]> {
  return invoke<BacklinkDto[]>("dc_get_backlinks", { recordId });
}

// ── Audio types ──

export interface AudioInitResult {
  trigger_count: number;
  bank_count: number;
}

export interface AudioBankResult {
  name: string;
  trigger_count: number;
}

export interface AudioEntityResult {
  name: string;
  record_path: string;
  trigger_count: number;
}

export interface AudioTriggerResult {
  trigger_name: string;
  bank_name: string;
  duration_type: string;
  radius_max: number | null;
}

export interface AudioTriggerDetail {
  trigger_name: string;
  bank_name: string;
  duration_type: string;
  sound_count: number;
}

export interface AudioSoundResult {
  media_id: number;
  source_type: string;
  bank_name: string;
  path_description: string;
}

// ── Audio commands ──

/** Build ATL index from P4k. Called once, cached. */
export async function audioInit(): Promise<AudioInitResult> {
  return invoke<AudioInitResult>("audio_init");
}

/** Search DataCore for entities with audio triggers matching query. */
export async function audioSearchEntities(query: string): Promise<AudioEntityResult[]> {
  return invoke<AudioEntityResult[]>("audio_search_entities", { query });
}

/** Search ATL index by trigger name substring. */
export async function audioSearchTriggers(query: string): Promise<AudioTriggerResult[]> {
  return invoke<AudioTriggerResult[]>("audio_search_triggers", { query });
}

/** List all soundbanks with trigger counts. */
export async function audioListBanks(): Promise<AudioBankResult[]> {
  return invoke<AudioBankResult[]>("audio_list_banks");
}

/** Get all triggers for a specific bank. */
export async function audioBankTriggers(bankName: string): Promise<AudioTriggerDetail[]> {
  return invoke<AudioTriggerDetail[]>("audio_bank_triggers", { bankName });
}

/** List all media in a bank by scanning HIRC directly (bypasses event resolution). */
export async function audioBankMedia(bankName: string): Promise<AudioSoundResult[]> {
  return invoke<AudioSoundResult[]>("audio_bank_media", { bankName });
}

/** Get all triggers for a specific entity, with resolved sound counts. */
export async function audioEntityTriggers(entityName: string): Promise<AudioTriggerDetail[]> {
  return invoke<AudioTriggerDetail[]>("audio_entity_triggers", { entityName });
}

/** Resolve a trigger to its leaf sounds via ATL -> bank -> HIRC. */
export async function audioResolveTrigger(triggerName: string): Promise<AudioSoundResult[]> {
  return invoke<AudioSoundResult[]>("audio_resolve_trigger", { triggerName });
}

/** Decode a WEM to Ogg bytes for browser playback. */
export async function audioDecodeWem(
  mediaId: number,
  sourceType: string,
  bankName: string,
): Promise<number[]> {
  return invoke<number[]>("audio_decode_wem", { mediaId, sourceType, bankName });
}

export interface FolderExtractProgress {
  current: number;
  total: number;
  name: string;
}

/** Listen for folder extract progress events. */
export function onFolderExtractProgress(
  callback: (progress: FolderExtractProgress) => void,
): Promise<UnlistenFn> {
  return listen<FolderExtractProgress>("folder-extract-progress", (event) => {
    callback(event.payload);
  });
}

/** Extract files under a P4k folder path to disk. Optional filter by extension (e.g. "mtl,xml"). */
export async function extractP4kFolder(
  pathPrefix: string,
  outputDir: string,
  filter?: string,
): Promise<number> {
  return invoke<number>("extract_p4k_folder", { pathPrefix, outputDir, filter: filter ?? null });
}

// ── Raw file access ──

/** Read a raw file from the P4K. */
export async function readP4kFile(path: string): Promise<ArrayBuffer> {
  const bytes = await invoke<number[]>("read_p4k_file", { path });
  return new Uint8Array(bytes).buffer;
}

// ── Geometry preview ──

/** Generate a GLB preview for a geometry file. Returns raw GLB bytes. */
export async function previewGeometry(path: string): Promise<ArrayBuffer> {
  const bytes = await invoke<number[]>("preview_geometry", { path });
  return new Uint8Array(bytes).buffer;
}

// ── XML preview ──

/** Decode a CryXMLB file and return formatted XML text. */
export async function previewXml(path: string): Promise<string> {
  return invoke<string>("preview_xml", { path });
}

// ── DDS preview ──

export interface DdsPreviewResult {
  png: number[];
  width: number;
  height: number;
  mip_level: number;
  mip_count: number;
}

/** Decode a DDS texture and return PNG bytes + metadata. */
export async function previewDds(
  path: string,
  mip?: number,
): Promise<DdsPreviewResult> {
  return invoke<DdsPreviewResult>("preview_dds", { path, mip: mip ?? null });
}

/** Save a DDS texture from P4K as a PNG file to disk. */
export async function exportDdsPng(
  path: string,
  outputPath: string,
  mip?: number,
): Promise<void> {
  return invoke<void>("export_dds_png", { path, outputPath, mip: mip ?? null });
}

/** Extract a single file from P4K to disk. */
export async function extractP4kFile(
  path: string,
  outputPath: string,
): Promise<void> {
  return invoke<void>("extract_p4k_file", { path, outputPath });
}

// ── Diff types ──

export interface ArchiveEntryInventory {
  path: string;
  normalized_path: string;
  crc32: number;
  compressed_size: number;
  uncompressed_size: number;
  compression_method: number;
  encrypted: boolean;
  last_modified: number;
}

export interface DataCoreRecordInventory {
  id: string;
  record_type: string;
  name: string;
  path: string;
  content_hash: string;
}

export type DataCoreInventory =
  | { source_path: string | null; status: "present"; records: DataCoreRecordInventory[] }
  | { source_path: string | null; status: "skipped" };

export interface InventoryReport {
  schema_version: number;
  mode: "full" | "p4k_only";
  generated_by: string;
  generated_at_unix: number;
  source: {
    label: string;
    source_file: {
      path_hint: string;
      size: number | null;
      modified_unix: number | null;
      entry_count: number;
    } | null;
    build_manifest?: {
      branch: string | null;
      version: string | null;
      requested_p4_change_num: number | null;
      channel_hint: string | null;
    };
    warnings: string[];
  };
  archive: ArchiveEntryInventory[];
  datacore: DataCoreInventory;
  inventory_hash: string;
}

export interface DiffInventoryHandle {
  id: string;
  label: string;
  mode: "full" | "p4k_only";
  path_hint: string | null;
  archive_count: number;
  datacore_count: number;
  inventory_hash: string;
  warnings: string[];
}

export type DiffStatus = "added" | "removed" | "modified" | "metadata_changed" | "unchanged";
export type DiffTier = "p4k" | "data_core";

export type DiffSide =
  | { archive: ArchiveEntryInventory }
  | { data_core: DataCoreRecordInventory };

export interface DiffItem {
  tier: DiffTier;
  status: DiffStatus;
  key: string;
  display: string;
  old: DiffSide | null;
  new: DiffSide | null;
  reasons: string[];
}

export interface DiffSummary {
  added: number;
  removed: number;
  modified: number;
  metadata_changed: number;
  unchanged: number;
  p4k_items: number;
  datacore_items: number;
}

export interface DiffReport {
  schema_version: number;
  old_label: string;
  new_label: string;
  old_inventory_hash: string;
  new_inventory_hash: string;
  summary: DiffSummary;
  items: DiffItem[];
}

export interface DiffFilter {
  search?: string | null;
  tiers: DiffTier[];
  statuses: DiffStatus[];
  extensions: string[];
  record_types: string[];
  path_prefixes: string[];
  include_unchanged: boolean;
}

export interface DiffInventoryProgress {
  job_id: string;
  phase: string;
  current: number | null;
  total: number | null;
  message: string;
}

export async function browseDiffSource(): Promise<string | null> {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const result = await open({
    title: "Select P4k or inventory report",
    filters: [
      { name: "Diff sources", extensions: ["p4k", "json"] },
      { name: "P4k archive", extensions: ["p4k"] },
      { name: "Inventory report", extensions: ["json"] },
    ],
    multiple: false,
    directory: false,
  });
  return result ?? null;
}

export async function browseInventorySavePath(defaultPath?: string): Promise<string | null> {
  const { save } = await import("@tauri-apps/plugin-dialog");
  const result = await save({
    title: "Save inventory report",
    defaultPath,
    filters: [{ name: "Inventory report", extensions: ["json"] }],
  });
  return result ?? null;
}

export async function browseDiffSavePath(defaultPath?: string): Promise<string | null> {
  const { save } = await import("@tauri-apps/plugin-dialog");
  const result = await save({
    title: "Save diff report",
    defaultPath,
    filters: [{ name: "Diff report", extensions: ["json"] }],
  });
  return result ?? null;
}

export function onDiffInventoryProgress(
  callback: (progress: DiffInventoryProgress) => void,
): Promise<UnlistenFn> {
  return listen<DiffInventoryProgress>("diff-inventory-progress", (event) => {
    callback(event.payload);
  });
}

export async function diffGenerateInventory(
  path: string,
  skipDatacore: boolean,
  jobId: string,
  label?: string,
): Promise<DiffInventoryHandle> {
  return invoke<DiffInventoryHandle>("diff_generate_inventory", {
    path,
    skipDatacore,
    label: label ?? null,
    jobId,
  });
}

export async function diffCancelInventory(): Promise<void> {
  return invoke<void>("diff_cancel_inventory");
}

export async function diffLoadInventoryReport(path: string): Promise<DiffInventoryHandle> {
  return invoke<DiffInventoryHandle>("diff_load_inventory_report", { path });
}

export async function diffSaveInventoryReport(
  path: string,
  id: string,
): Promise<void> {
  return invoke<void>("diff_save_inventory_report", { path, id });
}

export async function diffCompareReports(
  oldId: string,
  newId: string,
  includeUnchanged: boolean,
  filter?: DiffFilter,
  maxItems = 5000,
): Promise<DiffReport> {
  return invoke<DiffReport>("diff_compare_reports", {
    oldId,
    newId,
    includeUnchanged,
    filter: filter ?? null,
    maxItems,
  });
}

export async function diffSaveDiffReport(path: string, report: DiffReport): Promise<void> {
  return invoke<void>("diff_save_diff_report", { path, report });
}
