import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { SocpakCategoryDto, SocpakExportDone } from "../lib/commands";
import { tauriStorage } from "../lib/tauri-storage";

interface SocpakState {
  categories: SocpakCategoryDto[];
  categoriesLoading: boolean;
  activeCategory: number;
  setActiveCategory: (index: number) => void;
  setCategories: (categories: SocpakCategoryDto[]) => void;
  setCategoriesLoading: (loading: boolean) => void;

  selected: Set<string>;
  toggleSocpak: (path: string) => void;
  selectAllFiltered: (paths: string[]) => void;
  clearFiltered: (paths: string[]) => void;

  search: string;
  setSearch: (query: string) => void;

  lod: number;
  mip: number;
  exportKind: string;
  materialMode: string;
  includeLights: boolean;
  connected: boolean;
  overwriteExistingAssets: boolean;
  includeNodraw: boolean;
  threads: number;
  outputDir: string | null;
  setLod: (v: number) => void;
  setMip: (v: number) => void;
  setExportKind: (v: string) => void;
  setMaterialMode: (v: string) => void;
  setIncludeLights: (v: boolean) => void;
  setConnected: (v: boolean) => void;
  setOverwriteExistingAssets: (v: boolean) => void;
  setIncludeNodraw: (v: boolean) => void;
  setThreads: (v: number) => void;
  setOutputDir: (dir: string | null) => void;

  exporting: boolean;
  progressFraction: number;
  progress: number;
  progressTotal: number;
  progressLabel: string;
  progressStage: string;
  exportErrors: string[];
  result: SocpakExportDone | null;
  setExporting: (v: boolean) => void;
  setProgress: (fraction: number, current: number, total: number, label: string, stage: string) => void;
  addExportError: (msg: string) => void;
  setResult: (result: SocpakExportDone | null) => void;
  deselectPaths: (paths: string[]) => void;
}

type PersistedSocpakState = Pick<
  SocpakState,
  | "lod"
  | "mip"
  | "exportKind"
  | "materialMode"
  | "includeLights"
  | "connected"
  | "overwriteExistingAssets"
  | "includeNodraw"
  | "threads"
  | "outputDir"
>;

export const useSocpakStore = create<SocpakState>()(
  persist<SocpakState, [], [], PersistedSocpakState>(
    (set) => ({
      categories: [],
      categoriesLoading: false,
      activeCategory: 0,
      setActiveCategory: (index) => set({ activeCategory: index }),
      setCategories: (categories) => set({ categories, categoriesLoading: false }),
      setCategoriesLoading: (loading) => set({ categoriesLoading: loading }),

      selected: new Set(),
      toggleSocpak: (path) =>
        set((s) => {
          const next = new Set(s.selected);
          if (next.has(path)) next.delete(path);
          else next.add(path);
          return { selected: next };
        }),
      selectAllFiltered: (paths) =>
        set((s) => {
          const next = new Set(s.selected);
          for (const path of paths) next.add(path);
          return { selected: next };
        }),
      clearFiltered: (paths) =>
        set((s) => {
          const next = new Set(s.selected);
          for (const path of paths) next.delete(path);
          return { selected: next };
        }),

      search: "",
      setSearch: (query) => set({ search: query }),

      lod: 1,
      mip: 2,
      exportKind: "decomposed",
      materialMode: "textures",
      includeLights: true,
      connected: true,
      overwriteExistingAssets: true,
      includeNodraw: false,
      threads: 0,
      outputDir: null,
      setLod: (v) => set({ lod: v }),
      setMip: (v) => set({ mip: v }),
      setExportKind: (v) => set({ exportKind: v }),
      setMaterialMode: (v) => set({ materialMode: v }),
      setIncludeLights: (v) => set({ includeLights: v }),
      setConnected: (v) => set({ connected: v }),
      setOverwriteExistingAssets: (v) => set({ overwriteExistingAssets: v }),
      setIncludeNodraw: (v) => set({ includeNodraw: v }),
      setThreads: (v) => set({ threads: v }),
      setOutputDir: (dir) => set({ outputDir: dir }),

      exporting: false,
      progressFraction: 0,
      progress: 0,
      progressTotal: 0,
      progressLabel: "",
      progressStage: "",
      exportErrors: [],
      result: null,
      setExporting: (v) =>
        set({
          exporting: v,
          ...(v
            ? {
                result: null,
                exportErrors: [],
                progressFraction: 0,
                progress: 0,
                progressTotal: 0,
                progressLabel: "",
                progressStage: "",
              }
            : {}),
        }),
      setProgress: (fraction, current, total, label, stage) =>
        set({
          progressFraction: fraction,
          progress: current,
          progressTotal: total,
          progressLabel: label,
          progressStage: stage,
        }),
      addExportError: (msg) =>
        set((s) => ({ exportErrors: [...s.exportErrors, msg] })),
      setResult: (result) => set({ result, exporting: false }),
      deselectPaths: (paths) =>
        set((s) => {
          const next = new Set(s.selected);
          for (const path of paths) next.delete(path);
          return { selected: next };
        }),
    }),
    {
      name: "socpak-export",
      storage: tauriStorage,
      partialize: (s) => ({
        lod: s.lod,
        mip: s.mip,
        exportKind: s.exportKind,
        materialMode: s.materialMode,
        includeLights: s.includeLights,
        connected: s.connected,
        overwriteExistingAssets: s.overwriteExistingAssets,
        includeNodraw: s.includeNodraw,
        threads: s.threads,
        outputDir: s.outputDir,
      }),
    },
  ),
);
