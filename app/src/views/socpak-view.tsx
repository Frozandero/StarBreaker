import { useEffect, useState } from "react";
import { FolderOpen } from "lucide-react";
import { ResizeHandle } from "../components/resize-handle";
import { useSocpakStore } from "../stores/socpak-store";
import {
  browseOutputDir,
  cancelExport,
  onSocpakExportDone,
  onSocpakExportProgress,
  scanSocpakCategories,
  startSocpakExport,
  type SocpakExportRequest,
} from "../lib/commands";

function formatSize(bytes: number): string {
  if (bytes >= 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${bytes} B`;
}

export function SocpakView() {
  const [optionsWidth, setOptionsWidth] = useState(260);
  const categories = useSocpakStore((s) => s.categories);
  const categoriesLoading = useSocpakStore((s) => s.categoriesLoading);
  const activeCategory = useSocpakStore((s) => s.activeCategory);
  const setActiveCategory = useSocpakStore((s) => s.setActiveCategory);
  const setCategories = useSocpakStore((s) => s.setCategories);
  const setCategoriesLoading = useSocpakStore((s) => s.setCategoriesLoading);

  const selected = useSocpakStore((s) => s.selected);
  const toggleSocpak = useSocpakStore((s) => s.toggleSocpak);
  const selectAllFiltered = useSocpakStore((s) => s.selectAllFiltered);
  const clearFiltered = useSocpakStore((s) => s.clearFiltered);

  const search = useSocpakStore((s) => s.search);
  const setSearch = useSocpakStore((s) => s.setSearch);

  const lod = useSocpakStore((s) => s.lod);
  const mip = useSocpakStore((s) => s.mip);
  const exportKind = useSocpakStore((s) => s.exportKind);
  const materialMode = useSocpakStore((s) => s.materialMode);
  const includeLights = useSocpakStore((s) => s.includeLights);
  const connected = useSocpakStore((s) => s.connected);
  const overwriteExistingAssets = useSocpakStore((s) => s.overwriteExistingAssets);
  const includeNodraw = useSocpakStore((s) => s.includeNodraw);
  const threads = useSocpakStore((s) => s.threads);
  const outputDir = useSocpakStore((s) => s.outputDir);
  const setLod = useSocpakStore((s) => s.setLod);
  const setMip = useSocpakStore((s) => s.setMip);
  const setExportKind = useSocpakStore((s) => s.setExportKind);
  const setMaterialMode = useSocpakStore((s) => s.setMaterialMode);
  const setIncludeLights = useSocpakStore((s) => s.setIncludeLights);
  const setConnected = useSocpakStore((s) => s.setConnected);
  const setOverwriteExistingAssets = useSocpakStore((s) => s.setOverwriteExistingAssets);
  const setIncludeNodraw = useSocpakStore((s) => s.setIncludeNodraw);
  const setThreads = useSocpakStore((s) => s.setThreads);
  const setOutputDir = useSocpakStore((s) => s.setOutputDir);

  const exporting = useSocpakStore((s) => s.exporting);
  const progressFraction = useSocpakStore((s) => s.progressFraction);
  const progress = useSocpakStore((s) => s.progress);
  const progressTotal = useSocpakStore((s) => s.progressTotal);
  const progressLabel = useSocpakStore((s) => s.progressLabel);
  const progressStage = useSocpakStore((s) => s.progressStage);
  const exportErrors = useSocpakStore((s) => s.exportErrors);
  const result = useSocpakStore((s) => s.result);
  const setExporting = useSocpakStore((s) => s.setExporting);
  const setProgress = useSocpakStore((s) => s.setProgress);
  const addExportError = useSocpakStore((s) => s.addExportError);
  const setResult = useSocpakStore((s) => s.setResult);
  const deselectPaths = useSocpakStore((s) => s.deselectPaths);

  useEffect(() => {
    setCategoriesLoading(true);
    scanSocpakCategories()
      .then((cats) => setCategories(cats))
      .catch((err) => {
        console.error("Failed to scan SOCPAK categories:", err);
        setCategoriesLoading(false);
      });
  }, [setCategoriesLoading, setCategories]);

  useEffect(() => {
    if (categories.length > 0 && activeCategory >= categories.length) {
      setActiveCategory(0);
    }
  }, [categories.length, activeCategory, setActiveCategory]);

  useEffect(() => {
    let cancelled = false;
    const unlisteners: Array<() => void> = [];

    onSocpakExportProgress((p) => {
      if (!cancelled) {
        setProgress(p.fraction, p.current, p.total, p.socpak_name, p.stage);
        if (p.error) {
          addExportError(p.error);
        }
      }
    }).then((unlisten) => {
      if (cancelled) unlisten();
      else unlisteners.push(unlisten);
    });

    onSocpakExportDone((r) => {
      if (!cancelled) {
        if (r.succeeded_paths.length > 0) {
          deselectPaths(r.succeeded_paths);
        }
        setResult(r);
      }
    }).then((unlisten) => {
      if (cancelled) unlisten();
      else unlisteners.push(unlisten);
    });

    return () => {
      cancelled = true;
      for (const fn of unlisteners) fn();
    };
  }, [setProgress, addExportError, setResult, deselectPaths]);

  const category = categories[activeCategory] ?? categories[0];
  const filtered = category
    ? category.socpaks.filter((socpak) => {
        if (search === "") return true;
        const q = search.toLowerCase();
        return (
          socpak.name.toLowerCase().includes(q) ||
          socpak.path.toLowerCase().includes(q) ||
          socpak.category.toLowerCase().includes(q)
        );
      })
    : [];

  const selectedInCategory = filtered.filter((socpak) => selected.has(socpak.path)).length;
  const totalSelected = selected.size;
  const canExport = totalSelected > 0 && outputDir !== null && !exporting;
  const progressPercent = Math.round(progressFraction * 100);

  const handleExport = () => {
    const allSocpaks = categories.flatMap((cat) => cat.socpaks);
    const selectedSocpaks = allSocpaks.filter((socpak) => selected.has(socpak.path));
    const request: SocpakExportRequest = {
      socpak_paths: selectedSocpaks.map((socpak) => socpak.path),
      output_dir: outputDir!,
      lod,
      mip,
      export_kind: exportKind,
      material_mode: materialMode,
      format: "glb",
      include_lights: includeLights,
      connected,
      overwrite_existing_assets: overwriteExistingAssets,
      include_nodraw: includeNodraw,
      threads,
    };

    setExporting(true);
    setProgress(
      0,
      0,
      selectedSocpaks.length,
      selectedSocpaks.length === 1 ? selectedSocpaks[0].name : "Batch export",
      "Preparing export",
    );
    startSocpakExport(request).catch((err) => {
      console.error("SOCPAK export failed:", err);
      addExportError(String(err));
      setResult({ success: 0, errors: selectedSocpaks.length, succeeded_paths: [] });
    });
  };

  const handleCancel = () => {
    cancelExport().catch((err) => console.error("Cancel failed:", err));
  };

  const handleBrowse = () => {
    browseOutputDir().then((dir) => {
      if (dir !== null) setOutputDir(dir);
    });
  };

  return (
    <div className="flex-1 flex overflow-hidden relative">
      {exporting && (
        <div className="absolute inset-0 z-10 bg-bg/80 backdrop-blur-sm flex items-center justify-center">
          <div className="w-[360px] bg-bg-alt border border-border rounded-lg p-6 flex flex-col gap-4 shadow-lg">
            <h3 className="text-sm font-semibold text-text">
              Exporting SOCPAKs...
            </h3>

            <div className="flex flex-col gap-1.5">
              <div className="w-full bg-surface rounded-full h-2 overflow-hidden">
                <div
                  className="bg-accent h-full rounded-full transition-all duration-300"
                  style={{ width: `${progressFraction * 100}%` }}
                />
              </div>
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0 flex-1">
                  <p className="text-[11px] text-text-dim truncate">
                    {progressLabel || "Preparing export..."}
                  </p>
                  {progressStage && (
                    <p className="text-[10px] text-text-faint truncate mt-0.5">
                      {progressStage}
                    </p>
                  )}
                </div>
                <div className="text-right shrink-0">
                  <p className="text-[11px] text-text tabular-nums">
                    {progressPercent}%
                  </p>
                  <p className="text-[10px] text-text-faint tabular-nums mt-0.5">
                    {progress}/{progressTotal}
                  </p>
                </div>
              </div>
            </div>

            {exportErrors.length > 0 && (
              <div className="max-h-24 overflow-y-auto rounded bg-danger/5 border border-danger/20 px-3 py-2">
                {exportErrors.map((err, i) => (
                  <p key={i} className="text-[11px] text-danger/80 leading-relaxed">
                    {err}
                  </p>
                ))}
              </div>
            )}

            <button
              onClick={handleCancel}
              className="w-full py-2 rounded-md text-xs font-medium bg-danger/15 text-danger
                         hover:bg-danger/25 transition-colors cursor-pointer"
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      <div className="flex-1 flex flex-col min-w-0">
        <div className="flex items-center gap-2 px-3 border-b border-border bg-bg-alt shrink-0" style={{ height: "var(--toolbar-height)" }}>
          <input
            type="text"
            placeholder="Search SOCPAKs..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="flex-1 bg-surface rounded-md px-3 py-1.5 text-sm text-text placeholder:text-text-faint outline-none focus:ring-1 focus:ring-ring"
          />
          <button
            onClick={() => selectAllFiltered(filtered.map((socpak) => socpak.path))}
            className="text-[11px] text-text-dim hover:text-text px-2 py-1 rounded-md hover:bg-surface/60
                       transition-colors cursor-pointer shrink-0"
          >
            All
          </button>
          <button
            onClick={() => clearFiltered(filtered.map((socpak) => socpak.path))}
            className="text-[11px] text-text-dim hover:text-text px-2 py-1 rounded-md hover:bg-surface/60
                       transition-colors cursor-pointer shrink-0"
          >
            None
          </button>
          <span className="text-[11px] text-text-faint tabular-nums shrink-0">
            {selectedInCategory}/{filtered.length}
          </span>
          <div className="flex gap-1 shrink min-w-0 overflow-x-auto">
            {categoriesLoading ? (
              <span className="text-xs text-text-dim px-3 py-1 shrink-0">
                Scanning...
              </span>
            ) : (
              categories.map((cat, i) => (
                <button
                  key={cat.name}
                  onClick={() => setActiveCategory(i)}
                  className={`
                    px-3 py-1 rounded-md text-xs font-medium transition-colors cursor-pointer shrink-0
                    ${
                      i === activeCategory
                        ? "bg-primary/15 text-text"
                        : "bg-surface text-text-dim hover:bg-surface-hi hover:text-text"
                    }
                  `}
                >
                  {cat.name}
                  <span className="ml-1.5 opacity-60">
                    {cat.socpaks.length}
                  </span>
                </button>
              ))
            )}
          </div>
        </div>

        <div className="flex-1 overflow-y-auto px-1">
          {filtered.length === 0 && !categoriesLoading && (
            <div className="h-full flex items-center justify-center text-sm text-text-dim">
              No SOCPAKs match current filters.
            </div>
          )}
          {filtered.map((socpak) => {
            const isSelected = selected.has(socpak.path);
            return (
              <label
                key={socpak.path}
                className={`
                  flex items-center gap-2.5 px-3 py-[6px] rounded-md cursor-pointer text-xs
                  transition-colors select-none
                  ${isSelected ? "bg-primary/8 text-text" : "text-text-sub hover:bg-surface/40"}
                `}
              >
                <input
                  type="checkbox"
                  checked={isSelected}
                  onChange={() => toggleSocpak(socpak.path)}
                  className="accent-accent w-3.5 h-3.5 rounded shrink-0"
                />
                <span className="min-w-0 flex-1 flex flex-col gap-0.5">
                  <span className="truncate text-text">{socpak.name}</span>
                  <span className="truncate text-[10px] text-text-faint">
                    {socpak.path}
                  </span>
                </span>
                <span className="text-[10px] text-text-faint tabular-nums shrink-0">
                  {formatSize(socpak.size)}
                </span>
              </label>
            );
          })}
        </div>
      </div>

      <ResizeHandle width={optionsWidth} onResize={setOptionsWidth} side="left" min={200} max={400} />
      <div className="shrink-0 border-l border-border bg-bg-alt flex flex-col" style={{ width: optionsWidth }}>
        <div className="flex-1 overflow-y-auto p-4 flex flex-col gap-5">
          <h2 className="text-xs font-semibold text-primary uppercase tracking-wider">
            SOCPAK Export
          </h2>

          <div className="flex flex-col gap-1.5">
            <div className="flex items-center justify-between">
              <span className="text-xs text-text-sub">LOD Level</span>
              <span className="text-xs text-text-faint tabular-nums">{lod}</span>
            </div>
            <input
              type="range"
              min={0}
              max={4}
              value={lod}
              onChange={(e) => setLod(Number(e.target.value))}
              className="w-full accent-accent h-1.5"
            />
            <div className="flex justify-between text-[10px] text-text-faint">
              <span>Highest</span>
              <span>Lowest</span>
            </div>
          </div>

          <div className="flex flex-col gap-1.5">
            <div className="flex items-center justify-between">
              <span className="text-xs text-text-sub">Texture Mip</span>
              <span className="text-xs text-text-faint tabular-nums">{mip}</span>
            </div>
            <input
              type="range"
              min={0}
              max={6}
              value={mip}
              onChange={(e) => setMip(Number(e.target.value))}
              className="w-full accent-accent h-1.5"
            />
            <div className="flex justify-between text-[10px] text-text-faint">
              <span>Full res</span>
              <span>Smallest</span>
            </div>
          </div>

          <div className="flex flex-col gap-1.5">
            <div className="flex items-center justify-between">
              <span className="text-xs text-text-sub">Threads</span>
              <span className="text-xs text-text-faint tabular-nums">
                {threads === 0 ? "Auto" : threads}
              </span>
            </div>
            <input
              type="range"
              min={0}
              max={navigator.hardwareConcurrency || 16}
              value={threads}
              onChange={(e) => setThreads(Number(e.target.value))}
              className="w-full accent-accent h-1.5"
            />
            <div className="flex justify-between text-[10px] text-text-faint">
              <span>Auto</span>
              <span>{navigator.hardwareConcurrency || 16}</span>
            </div>
          </div>

          <div className="flex flex-col gap-1.5">
            <span className="text-xs text-text-sub">Package</span>
            <div className="flex flex-col gap-1">
              {([
                { value: "bundled", label: "Bundled - .glb per SOCPAK" },
                { value: "decomposed", label: "Structured package" },
              ] as const).map((opt) => (
                <label key={opt.value} className="flex items-center gap-2 cursor-pointer group">
                  <input
                    type="radio"
                    name="socpakExportKind"
                    value={opt.value}
                    checked={exportKind === opt.value}
                    onChange={() => setExportKind(opt.value)}
                    className="accent-accent w-3 h-3"
                  />
                  <span className="text-xs text-text-sub group-hover:text-text transition-colors">
                    {opt.label}
                  </span>
                </label>
              ))}
            </div>
          </div>

          {exportKind === "decomposed" && (
            <div className="flex flex-col gap-3">
              <label className="flex items-center gap-2.5 cursor-pointer group">
                <input
                  type="checkbox"
                  checked={connected}
                  onChange={(e) => setConnected(e.target.checked)}
                  className="accent-accent w-3.5 h-3.5 rounded"
                />
                <span className="text-xs text-text-sub group-hover:text-text transition-colors">
                  Include connected SOCPAKs
                </span>
              </label>

              <label className="flex items-center gap-2.5 cursor-pointer group">
                <input
                  type="checkbox"
                  checked={overwriteExistingAssets}
                  onChange={(e) => setOverwriteExistingAssets(e.target.checked)}
                  className="accent-accent w-3.5 h-3.5 rounded"
                />
                <span className="text-xs text-text-sub group-hover:text-text transition-colors">
                  Overwrite existing meshes and textures
                </span>
              </label>

              <label className="flex items-center gap-2.5 cursor-pointer group">
                <input
                  type="checkbox"
                  checked={includeNodraw}
                  onChange={(e) => setIncludeNodraw(e.target.checked)}
                  className="accent-accent w-3.5 h-3.5 rounded"
                />
                <span className="text-xs text-text-sub group-hover:text-text transition-colors">
                  Include NoDraw faces and sidecars
                </span>
              </label>
            </div>
          )}

          <div className="flex flex-col gap-1.5">
            <span className="text-xs text-text-sub">Materials</span>
            <div className="flex flex-col gap-1">
              {([
                { value: "none", label: "None" },
                { value: "colors", label: "Colors" },
                { value: "textures", label: "Textures" },
                { value: "all", label: "All (experimental)" },
              ] as const).map((opt) => (
                <label key={opt.value} className="flex items-center gap-2 cursor-pointer group">
                  <input
                    type="radio"
                    name="socpakMaterialMode"
                    value={opt.value}
                    checked={materialMode === opt.value}
                    onChange={() => setMaterialMode(opt.value)}
                    className="accent-accent w-3 h-3"
                  />
                  <span className="text-xs text-text-sub group-hover:text-text transition-colors">
                    {opt.label}
                  </span>
                </label>
              ))}
            </div>
          </div>

          <div className="flex flex-col gap-2">
            <label className="flex items-center gap-2.5 cursor-pointer group">
              <input
                type="checkbox"
                checked={includeLights}
                onChange={(e) => setIncludeLights(e.target.checked)}
                className="accent-accent w-3.5 h-3.5 rounded"
              />
              <span className="text-xs text-text-sub group-hover:text-text transition-colors">
                Include lights
              </span>
            </label>
          </div>

          <div className="flex flex-col gap-1.5">
            <span className="text-xs text-text-sub">Output directory</span>
            <button
              onClick={handleBrowse}
              className="flex items-center gap-2 w-full bg-surface/50 border border-border rounded-md
                         px-3 py-2 text-xs text-left cursor-pointer hover:bg-surface/80 transition-colors"
            >
              <FolderOpen size={14} className="text-text-faint shrink-0" />
              <span className={outputDir ? "text-text truncate" : "text-text-faint"}>
                {outputDir ?? "Choose folder..."}
              </span>
            </button>
          </div>
        </div>

        <div className="p-4 border-t border-border flex flex-col gap-3">
          {!exporting && result && (
            <div className="flex flex-col gap-1.5">
              <p className={`text-[11px] ${result.errors > 0 ? "text-warning" : "text-success"}`}>
                {result.errors > 0
                  ? `Exported ${result.success} SOCPAK${result.success !== 1 ? "s" : ""}, ${result.errors} failed`
                  : `Exported ${result.success} SOCPAK${result.success !== 1 ? "s" : ""} successfully`}
              </p>
              {exportErrors.length > 0 && (
                <div className="max-h-20 overflow-y-auto rounded bg-danger/5 border border-danger/20 px-2 py-1.5">
                  {exportErrors.map((err, i) => (
                    <p key={i} className="text-[10px] text-danger/80 leading-relaxed">
                      {err}
                    </p>
                  ))}
                </div>
              )}
            </div>
          )}

          <button
            onClick={handleExport}
            disabled={!canExport}
            className={`
              w-full py-2 rounded-md text-xs font-medium transition-colors cursor-pointer
              ${
                canExport
                  ? "bg-accent text-on-accent hover:brightness-110"
                  : "bg-surface text-text-faint cursor-not-allowed"
              }
            `}
          >
            {totalSelected === 0
              ? "Select SOCPAKs to export"
              : outputDir === null
                ? "Choose output directory"
                : `Export ${totalSelected} SOCPAK${totalSelected !== 1 ? "s" : ""}`}
          </button>
        </div>
      </div>
    </div>
  );
}
