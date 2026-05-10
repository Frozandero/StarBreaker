import { useMemo, useRef, useState } from "react";
import {
  AlertTriangle,
  Download,
  FileJson,
  GitCompare,
  Loader2,
  Save,
  Search,
  X,
} from "lucide-react";
import {
  browseDiffSavePath,
  browseDiffSource,
  browseInventorySavePath,
  diffCancelInventory,
  diffCompareReports,
  diffGenerateInventory,
  diffLoadInventoryReport,
  diffSaveDiffReport,
  diffSaveInventoryReport,
  onDiffInventoryProgress,
  type DiffInventoryProgress,
  type DiffFilter,
  type DiffItem,
  type DiffReport,
  type DiffStatus,
  type DiffTier,
  type DiffInventoryHandle,
} from "../lib/commands";

type SlotId = "old" | "new";

interface SourceSlot {
  path: string | null;
  report: DiffInventoryHandle | null;
  loading: boolean;
  error: string | null;
  progress: DiffInventoryProgress | null;
}

type StatusFilter = DiffStatus | "all";
type TierFilter = DiffTier | "all";

const emptySlot: SourceSlot = {
  path: null,
  report: null,
  loading: false,
  error: null,
  progress: null,
};

function isInventoryPath(path: string): boolean {
  return path.toLowerCase().endsWith(".starbreaker-inventory.json");
}

function formatCount(value: number): string {
  return value.toLocaleString();
}

function statusLabel(status: DiffStatus): string {
  switch (status) {
    case "metadata_changed": return "metadata";
    default: return status;
  }
}

function tierLabel(tier: DiffTier): string {
  return tier === "data_core" ? "DataCore" : "P4k";
}

function itemPath(item: DiffItem): string {
  const side = item.new ?? item.old;
  if (!side) return item.display;
  if ("archive" in side) return side.archive.path;
  return side.data_core.path;
}

function itemType(item: DiffItem): string {
  const side = item.new ?? item.old;
  if (!side) return "";
  if ("archive" in side) {
    const match = side.archive.path.match(/\.([^.\\/]+)$/);
    return match ? `.${match[1].toLowerCase()}` : "";
  }
  return side.data_core.record_type;
}

export function DiffView() {
  const [oldSlot, setOldSlot] = useState<SourceSlot>(emptySlot);
  const [newSlot, setNewSlot] = useState<SourceSlot>(emptySlot);
  const [diff, setDiff] = useState<DiffReport | null>(null);
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [includeUnchanged, setIncludeUnchanged] = useState(false);
  const [search, setSearch] = useState("");
  const [tier, setTier] = useState<TierFilter>("all");
  const [status, setStatus] = useState<StatusFilter>("all");
  const [extension, setExtension] = useState("");
  const [recordType, setRecordType] = useState("");
  const [pathPrefix, setPathPrefix] = useState("");
  const [scrollTop, setScrollTop] = useState(0);
  const scrollRef = useRef<HTMLDivElement | null>(null);

  const setSlot = (slot: SlotId, next: SourceSlot) => {
    if (slot === "old") setOldSlot(next);
    else setNewSlot(next);
  };

  const updateSlot = (slot: SlotId, update: (current: SourceSlot) => SourceSlot) => {
    if (slot === "old") setOldSlot(update);
    else setNewSlot(update);
  };

  const loadSource = async (slot: SlotId) => {
    const path = await browseDiffSource();
    if (!path) return;
    setError(null);
    setDiff(null);
    setSelectedKey(null);

    if (isInventoryPath(path)) {
      setSlot(slot, { ...emptySlot, path, loading: true });
      try {
        const report = await diffLoadInventoryReport(path);
        setSlot(slot, { ...emptySlot, path, report });
      } catch (err) {
        setSlot(slot, { ...emptySlot, path, error: String(err) });
      }
      return;
    }

    setSlot(slot, { ...emptySlot, path, loading: true });
    const jobId = `${slot}-${Date.now()}-${Math.random().toString(16).slice(2)}`;
    const unlisten = await onDiffInventoryProgress((progress) => {
      if (progress.job_id !== jobId) return;
      updateSlot(slot, (current) => ({ ...current, loading: true, progress }));
    });
    try {
      const report = await diffGenerateInventory(path, false, jobId);
      setSlot(slot, { ...emptySlot, path, report });
    } catch (err) {
      setSlot(slot, { ...emptySlot, path, error: String(err) });
    } finally {
      unlisten();
    }
  };

  const compare = async () => {
    if (!oldSlot.report || !newSlot.report) return;
    setError(null);
    try {
      const filter = buildBackendFilter({
        search,
        tier,
        status,
        extension,
        recordType,
        pathPrefix,
      });
      console.info("diff compare request", {
        oldId: oldSlot.report.id,
        oldLabel: oldSlot.report.label,
        newId: newSlot.report.id,
        newLabel: newSlot.report.label,
        filter,
        maxItems: 5000,
      });
      const next = await diffCompareReports(oldSlot.report.id, newSlot.report.id, false, filter, 5000);
      console.info("diff compare response", {
        summary: next.summary,
        returnedItems: next.items.length,
      });
      setDiff(next);
      setSelectedKey(next.items[0]?.key ?? null);
      setScrollTop(0);
      if (scrollRef.current) scrollRef.current.scrollTop = 0;
    } catch (err) {
      console.error("diff compare failed", err);
      setError(String(err));
    }
  };

  const saveInventory = async (slot: SourceSlot) => {
    if (!slot.report) return;
    const safeLabel = slot.report.label.replace(/[\\/:*?"<>|]/g, "_");
    const path = await browseInventorySavePath(`${safeLabel}.starbreaker-inventory.json`);
    if (path) await diffSaveInventoryReport(path, slot.report.id);
  };

  const saveDiff = async () => {
    if (!diff) return;
    const path = await browseDiffSavePath(`${diff.old_label}-to-${diff.new_label}.starbreaker-diff.json`);
    if (path) await diffSaveDiffReport(path, diff);
  };

  const filteredItems = useMemo(() => {
    if (!diff) return [];
    const text = search.trim().toLowerCase();
    const ext = extension.trim().toLowerCase();
    const recType = recordType.trim().toLowerCase();
    const prefix = pathPrefix.trim().replaceAll("\\", "/").toLowerCase();
    return diff.items.filter((item) => {
      if (tier !== "all" && item.tier !== tier) return false;
      if (status !== "all" && item.status !== status) return false;
      if (ext && (item.tier !== "p4k" || itemType(item) !== (ext.startsWith(".") ? ext : `.${ext}`))) {
        return false;
      }
      if (recType && (item.tier !== "data_core" || !itemType(item).toLowerCase().includes(recType))) {
        return false;
      }
      if (prefix && !itemPath(item).replaceAll("\\", "/").toLowerCase().startsWith(prefix)) {
        return false;
      }
      if (!text) return true;
      const fields = [
        item.display,
        item.key,
        itemPath(item),
        itemType(item),
        item.reasons.join(" "),
      ].join(" ").toLowerCase();
      return fields.includes(text);
    });
  }, [diff, extension, includeUnchanged, pathPrefix, recordType, search, status, tier]);

  const selected = filteredItems.find((item) => item.key === selectedKey) ?? filteredItems[0] ?? null;
  const totalChanged = diff
    ? diff.summary.added + diff.summary.removed + diff.summary.modified + diff.summary.metadata_changed
    : 0;
  const rowHeight = 34;
  const viewportHeight = 560;
  const start = Math.max(0, Math.floor(scrollTop / rowHeight) - 6);
  const visibleCount = Math.ceil(viewportHeight / rowHeight) + 12;
  const visibleItems = filteredItems.slice(start, start + visibleCount);

  return (
    <div className="flex flex-col h-full bg-bg overflow-hidden">
      <header className="h-[var(--toolbar-height)] border-b border-border px-4 flex items-center justify-between bg-bg-alt">
        <div className="flex items-center gap-2">
          <GitCompare size={18} className="text-primary" />
          <h1 className="font-semibold text-sm">Diff Viewer</h1>
        </div>
        <div className="flex items-center gap-2">
          <label className="flex items-center gap-2 text-xs text-text-sub">
            <input
              type="checkbox"
              checked={includeUnchanged}
              disabled
              onChange={(event) => setIncludeUnchanged(event.target.checked)}
            />
            Unchanged counted in summary
          </label>
          <button
            onClick={compare}
            disabled={!oldSlot.report || !newSlot.report}
            className="px-3 py-1.5 rounded-md bg-primary/15 text-text text-xs disabled:opacity-40"
          >
            Compare
          </button>
          <button
            onClick={saveDiff}
            disabled={!diff}
            title="Export diff"
            className="p-1.5 rounded-md hover:bg-surface disabled:opacity-40"
          >
            <Download size={15} />
          </button>
        </div>
      </header>

      <section className="grid grid-cols-2 gap-3 p-3 border-b border-border">
        <SourcePanel slotId="old" slot={oldSlot} onLoad={() => loadSource("old")} onSave={() => saveInventory(oldSlot)} />
        <SourcePanel slotId="new" slot={newSlot} onLoad={() => loadSource("new")} onSave={() => saveInventory(newSlot)} />
      </section>

      {error && (
        <div className="mx-3 mt-3 p-2 border border-danger/30 bg-danger/10 text-danger text-xs rounded-md">
          {error}
        </div>
      )}

      {diff && (
        <section className="px-3 py-2 border-b border-border">
          <div className="grid grid-cols-5 gap-2">
            <SummaryCell label="Added" value={diff.summary.added} tone="text-success" />
            <SummaryCell label="Removed" value={diff.summary.removed} tone="text-danger" />
            <SummaryCell label="Modified" value={diff.summary.modified} tone="text-warning" />
            <SummaryCell label="Metadata" value={diff.summary.metadata_changed} tone="text-info" />
            <SummaryCell label="Unchanged" value={diff.summary.unchanged} tone="text-text-dim" />
          </div>
          {diff.items.length < totalChanged && (
            <p className="mt-2 text-xs text-text-dim">
              Showing {formatCount(diff.items.length)} rows. Narrow filters and compare again for a focused result set.
            </p>
          )}
        </section>
      )}

      <div className="flex flex-1 min-h-0">
        <aside className="w-[240px] border-r border-border p-3 flex flex-col gap-3 bg-bg-alt/40">
          <div className="relative">
            <Search size={14} className="absolute left-2 top-2.5 text-text-dim" />
            <input
              value={search}
              onChange={(event) => setSearch(event.target.value)}
              placeholder="Search"
              className="w-full pl-7 pr-2 py-2 bg-surface border border-border rounded-md text-sm outline-none focus:border-primary"
            />
          </div>
          <FilterSelect label="Tier" value={tier} onChange={(value) => setTier(value as TierFilter)} options={[
            ["all", "All"],
            ["p4k", "P4k files"],
            ["data_core", "DataCore"],
          ]} />
          <FilterSelect label="Status" value={status} onChange={(value) => setStatus(value as StatusFilter)} options={[
            ["all", "All"],
            ["added", "Added"],
            ["removed", "Removed"],
            ["modified", "Modified"],
            ["metadata_changed", "Metadata"],
            ["unchanged", "Unchanged"],
          ]} />
          <TextFilter label="Extension" value={extension} onChange={setExtension} placeholder=".dds" />
          <TextFilter label="Record type" value={recordType} onChange={setRecordType} placeholder="EntityClassDefinition" />
          <TextFilter label="Path prefix" value={pathPrefix} onChange={setPathPrefix} placeholder="Data/Objects" />
        </aside>

        <main className="flex-1 min-w-0 flex flex-col">
          <div className="grid grid-cols-[80px_76px_minmax(140px,1fr)_130px_minmax(120px,0.8fr)] gap-3 px-3 py-2 text-[11px] uppercase tracking-wide text-text-dim border-b border-border">
            <span>Status</span>
            <span>Tier</span>
            <span>Name</span>
            <span>Type</span>
            <span>Reasons</span>
          </div>
          <div
            ref={scrollRef}
            className="flex-1 overflow-auto"
            onScroll={(event) => setScrollTop(event.currentTarget.scrollTop)}
          >
            <div style={{ height: filteredItems.length * rowHeight, position: "relative" }}>
              <div style={{ transform: `translateY(${start * rowHeight}px)` }}>
                {visibleItems.map((item) => (
                  <button
                    key={`${item.tier}:${item.key}`}
                    onClick={() => setSelectedKey(item.key)}
                    className={`grid grid-cols-[80px_76px_minmax(140px,1fr)_130px_minmax(120px,0.8fr)] gap-3 w-full px-3 text-left text-xs border-b border-border/60 hover:bg-surface/60 ${
                      selected?.key === item.key ? "bg-primary/10" : ""
                    }`}
                    style={{ height: rowHeight }}
                  >
                    <span className={`self-center ${statusTone(item.status)}`}>{statusLabel(item.status)}</span>
                    <span className="self-center text-text-sub">{tierLabel(item.tier)}</span>
                    <span className="self-center truncate" title={item.display}>{item.display}</span>
                    <span className="self-center truncate text-text-sub" title={itemType(item)}>{itemType(item)}</span>
                    <span className="self-center truncate text-text-dim" title={item.reasons.join(", ")}>{item.reasons.join(", ")}</span>
                  </button>
                ))}
              </div>
            </div>
          </div>
        </main>

        <aside className="w-[320px] border-l border-border p-3 bg-bg-alt/40 overflow-auto">
          <DetailPanel item={selected} />
        </aside>
      </div>
    </div>
  );
}

function buildBackendFilter(input: {
  search: string;
  tier: TierFilter;
  status: StatusFilter;
  extension: string;
  recordType: string;
  pathPrefix: string;
}): DiffFilter {
  return {
    search: input.search.trim() || null,
    tiers: input.tier === "all" ? [] : [input.tier],
    statuses: input.status === "all" ? [] : [input.status],
    extensions: input.extension.trim() ? [input.extension.trim()] : [],
    record_types: input.recordType.trim() ? [input.recordType.trim()] : [],
    path_prefixes: input.pathPrefix.trim() ? [input.pathPrefix.trim()] : [],
    include_unchanged: false,
  };
}

function SourcePanel({ slotId, slot, onLoad, onSave }: {
  slotId: SlotId;
  slot: SourceSlot;
  onLoad: () => void;
  onSave: () => void;
}) {
  return (
    <div className="border border-border rounded-md bg-bg-alt p-3 min-w-0">
      <div className="flex items-center justify-between gap-2">
        <div className="min-w-0">
          <p className="text-[11px] uppercase tracking-wide text-text-dim">{slotId === "old" ? "Old" : "New"}</p>
          <p className="text-sm font-medium truncate">{slot.report?.label ?? "No source selected"}</p>
        </div>
        <div className="flex items-center gap-1">
          {slot.loading && (
            <button onClick={() => diffCancelInventory()} title="Cancel" className="p-1.5 rounded-md hover:bg-surface">
              <X size={14} />
            </button>
          )}
          <button onClick={onSave} disabled={!slot.report} title="Save inventory" className="p-1.5 rounded-md hover:bg-surface disabled:opacity-40">
            <Save size={14} />
          </button>
          <button onClick={onLoad} title="Load source" className="p-1.5 rounded-md hover:bg-surface">
            <FileJson size={14} />
          </button>
        </div>
      </div>
      {slot.path && <p className="mt-2 text-xs text-text-dim truncate" title={slot.path}>{slot.path}</p>}
      {slot.loading && (
        <div className="mt-3 grid grid-cols-[16px_minmax(0,1fr)_96px] items-center gap-2 text-xs text-text-sub">
          <Loader2 size={14} className="animate-spin" />
          <span className="truncate">{slot.progress?.message ?? "Generating inventory"}</span>
          {slot.progress?.total != null && (
            <span className="text-text-dim text-right tabular-nums">
              {formatCount(slot.progress.current ?? 0)}/{formatCount(slot.progress.total)}
            </span>
          )}
        </div>
      )}
      {slot.report && (
        <p className="mt-2 text-xs text-text-dim">
          {formatCount(slot.report.archive_count)} files · {formatCount(slot.report.datacore_count)} records
        </p>
      )}
      {slot.error && (
        <div className="mt-2 flex gap-2 text-xs text-danger">
          <AlertTriangle size={14} />
          <span>{slot.error}</span>
        </div>
      )}
    </div>
  );
}

function SummaryCell({ label, value, tone }: { label: string; value: number; tone: string }) {
  return (
    <div className="bg-bg-alt border border-border rounded-md px-3 py-2">
      <p className="text-[11px] uppercase tracking-wide text-text-dim">{label}</p>
      <p className={`text-lg font-semibold ${tone}`}>{formatCount(value)}</p>
    </div>
  );
}

function FilterSelect({ label, value, onChange, options }: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  options: Array<[string, string]>;
}) {
  return (
    <label className="flex flex-col gap-1 text-xs text-text-dim">
      {label}
      <select
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className="bg-surface border border-border rounded-md px-2 py-2 text-sm text-text outline-none focus:border-primary"
      >
        {options.map(([value, label]) => <option key={value} value={value}>{label}</option>)}
      </select>
    </label>
  );
}

function TextFilter({ label, value, onChange, placeholder }: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder: string;
}) {
  return (
    <label className="flex flex-col gap-1 text-xs text-text-dim">
      {label}
      <input
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder={placeholder}
        className="bg-surface border border-border rounded-md px-2 py-2 text-sm text-text outline-none focus:border-primary"
      />
    </label>
  );
}

function DetailPanel({ item }: { item: DiffItem | null }) {
  if (!item) {
    return <p className="text-sm text-text-dim">Select a changed item.</p>;
  }
  return (
    <div className="flex flex-col gap-3">
      <div>
        <p className="text-[11px] uppercase tracking-wide text-text-dim">{tierLabel(item.tier)}</p>
        <h2 className="text-sm font-semibold break-words">{item.display}</h2>
      </div>
      <DetailRow label="Status" value={statusLabel(item.status)} />
      <DetailRow label="Key" value={item.key} />
      <DetailRow label="Path" value={itemPath(item)} />
      <DetailRow label="Type" value={itemType(item)} />
      <div>
        <p className="text-[11px] uppercase tracking-wide text-text-dim mb-1">Reasons</p>
        {item.reasons.length === 0 ? (
          <p className="text-xs text-text-dim">No change reasons.</p>
        ) : (
          <div className="flex flex-wrap gap-1">
            {item.reasons.map((reason) => (
              <span key={reason} className="px-2 py-1 bg-surface border border-border rounded text-xs">
                {reason}
              </span>
            ))}
          </div>
        )}
      </div>
      <SideBlock title="Old" side={item.old} />
      <SideBlock title="New" side={item.new} />
    </div>
  );
}

function DetailRow({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-[11px] uppercase tracking-wide text-text-dim">{label}</p>
      <p className="text-xs break-words">{value || "n/a"}</p>
    </div>
  );
}

function SideBlock({ title, side }: { title: string; side: DiffItem["old"] }) {
  if (!side) return <DetailRow label={title} value="missing" />;
  if ("archive" in side) {
    const entry = side.archive;
    return (
      <div className="border border-border rounded-md p-2">
        <p className="text-[11px] uppercase tracking-wide text-text-dim mb-1">{title}</p>
        <p className="text-xs break-words">{entry.path}</p>
        <p className="text-xs text-text-dim mt-1">CRC32 {entry.crc32} · {formatCount(entry.uncompressed_size)} bytes</p>
      </div>
    );
  }
  const record = side.data_core;
  return (
    <div className="border border-border rounded-md p-2">
      <p className="text-[11px] uppercase tracking-wide text-text-dim mb-1">{title}</p>
      <p className="text-xs break-words">{record.name}</p>
      <p className="text-xs text-text-dim mt-1">{record.record_type}</p>
      <p className="text-xs text-text-dim break-words mt-1">{record.content_hash}</p>
    </div>
  );
}

function statusTone(status: DiffStatus): string {
  switch (status) {
    case "added": return "text-success";
    case "removed": return "text-danger";
    case "modified": return "text-warning";
    case "metadata_changed": return "text-info";
    case "unchanged": return "text-text-dim";
  }
}
