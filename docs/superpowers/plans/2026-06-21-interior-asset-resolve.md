# Interior Asset Resolve Optimization — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Cut the ~16s serial `interior_asset_resolve` (1675 unique material-sidecar builds) without changing exported bytes, in two phases: (1) make output-path case canonicalization O(1), (2) build the interior sidecars in parallel.

**Architecture:** Phase 1 wraps the export's `files` map in an `OutputFiles` struct that maintains a lowercase-prefix→canonical-segment index, replacing the O(files²) per-segment key scan in `canonicalize_output_path_case` (~3s on Idris) with O(depth) lookups. Phase 2 pre-builds each unique interior sidecar in a rayon pass in `blend_assembly` (where the preloaded meshes already live), each task writing into a LOCAL `OutputFiles` while reading the shared pre-warmed decode cache read-only; results merge into the shared map in first-occurrence order. Both are byte-identical: Phase 1 produces the same canonical paths, Phase 2 just reorders independent work.

**Tech Stack:** Rust, rayon (already a dependency).

## Global Constraints

- **Byte-identical output is mandatory.** Each behaviour-changing task gates on `diff -rq` vs baseline being empty except `.export_stamp.json`. Any other diff ⇒ STOP.
- **Build:** `cargo build --release -p starbreaker`. **Data:** `SC_DATA_P4K="$HOME/Games/star-citizen/drive_c/Program Files/Roberts Space Industries/StarCitizen/LIVE/Data.p4k"`. **Profile:** `AEGS_Idris_P --kind decomposed --lod 0`.
- **No hard-coded game-data values; every new fn has a `///` doc.**
- **Profiling baseline (current HEAD `476a9503e`):** interior_asset_resolve ~16s = mtl reload 0.7s + extract 8.7s + build_insert 7.5s; `canonicalize_output_path_case` ~3.1s/4885 calls; final files ~3053; `write_material_sidecar` 1675 calls. wall ~52.4s.

---

## File Structure

- `crates/starbreaker-3d/src/decomposed.rs` — owns the writer. Phase 1 adds `OutputFiles` and migrates the 10 functions that take `files: &mut BTreeMap<String, Vec<u8>>` (`insert_binary_file`, `insert_json_file`, `write_mesh_asset`, `write_material_sidecar`, `extract_material_entry`, `build_slot_export_value`, `export_texture_asset`, `generated_ui_texture_for_binding`, `finalize_palette_records`, `insert_ui_export_stamp`) plus the local `let mut files = BTreeMap::new()` in `write_decomposed_export` and the final `files.iter()`→`ExportedFile` conversion. Phase 2 adds `prewarmed: &PngCache` threading + a parallel pre-build pass.
- `crates/starbreaker-3d/src/pipeline/blend_assembly.rs` — Phase 2 adds the parallel interior-sidecar pre-pass next to the existing texture prewarm.

Baseline produced in Task 0 from HEAD `476a9503e`.

---

### Task 0: Baseline

- [ ] **Step 1:** `cargo build --release -p starbreaker 2>&1 | tail -1` → `Finished`.
- [ ] **Step 2:** Export + record:
```bash
export SC_DATA_P4K="$HOME/Games/star-citizen/drive_c/Program Files/Roberts Space Industries/StarCitizen/LIVE/Data.p4k"
rm -rf target/tmp/iar_baseline && mkdir -p target/tmp/iar_baseline
/usr/bin/time -v env RUST_LOG=info ./target/release/starbreaker entity export "AEGS_Idris_P" target/tmp/iar_baseline --kind decomposed --lod 0 > target/tmp/iar_baseline.log 2>&1
grep -aE "interior_asset_resolve:|decomposed\] total:|Elapsed \(wall" target/tmp/iar_baseline.log
```
PASS check used throughout: `diff -rq target/tmp/iar_baseline "$ROOT" | grep -v export_stamp` empty.

---

## PHASE 1 — O(1) case canonicalization

### Task 1: `OutputFiles` wrapper with a case-index

**Files:** Modify `crates/starbreaker-3d/src/decomposed.rs`.

**Interfaces:**
- Produces: `pub(crate) struct OutputFiles { files: BTreeMap<String, Vec<u8>>, case_index: HashMap<String, String> }` with methods: `new() -> Self`, `contains_key(&self, &str) -> bool`, `get(&self, &str) -> Option<&Vec<u8>>`, `insert_canonical(&mut self, requested_path: String, bytes: Vec<u8>) -> String` (the old `insert_binary_file` body), `iter(&self) -> impl Iterator<Item=(&String,&Vec<u8>)>`, `len(&self) -> usize`. The index maps each inserted path's lowercase prefix (`a`, `a/b`, `a/b/c`…) to that prefix's canonical final segment.

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)] mod tests` block:
```rust
#[test]
fn output_files_canonicalizes_segment_case_from_first_insert() {
    let mut of = OutputFiles::new();
    // First insert establishes "Data/Foo" casing.
    of.insert_canonical("Data/Foo/a.bin".to_string(), vec![1]);
    // A later path with different case on existing segments adopts the first casing.
    let p = of.insert_canonical("data/foo/b.bin".to_string(), vec![2]);
    assert_eq!(p, "Data/Foo/b.bin");
    // Identical bytes at an existing path dedupe to that path.
    let p2 = of.insert_canonical("Data/Foo/a.bin".to_string(), vec![1]);
    assert_eq!(p2, "Data/Foo/a.bin");
    assert_eq!(of.len(), 2);
}
```

- [ ] **Step 2: Run it** — `cargo test -p starbreaker-3d --lib output_files_canonicalizes 2>&1 | tail -6` → FAIL (`OutputFiles` undefined).

- [ ] **Step 3: Implement `OutputFiles`**

Add near `insert_binary_file` in `decomposed.rs`:
```rust
/// The export's output file map plus an index that makes per-segment case
/// canonicalization O(path depth) instead of O(files) (the old code scanned
/// every key for every inserted segment — quadratic over an export). `case_index`
/// maps a lowercase path prefix to the canonical-cased final segment of the
/// FIRST key that introduced it, exactly matching the previous "first key wins"
/// behaviour.
pub(crate) struct OutputFiles {
    files: BTreeMap<String, Vec<u8>>,
    case_index: HashMap<String, String>,
}

impl OutputFiles {
    pub(crate) fn new() -> Self {
        Self { files: BTreeMap::new(), case_index: HashMap::new() }
    }
    pub(crate) fn contains_key(&self, key: &str) -> bool {
        self.files.contains_key(key)
    }
    pub(crate) fn get(&self, key: &str) -> Option<&Vec<u8>> {
        self.files.get(key)
    }
    pub(crate) fn len(&self) -> usize {
        self.files.len()
    }
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&String, &Vec<u8>)> {
        self.files.iter()
    }
    /// Canonicalize `requested_path`'s segment case against prior inserts, then
    /// insert `bytes` (deduping identical content, hashing genuine collisions).
    /// Returns the stored path.
    pub(crate) fn insert_canonical(&mut self, requested_path: String, bytes: Vec<u8>) -> String {
        let requested_path = self.canonicalize_case(&requested_path);
        if let Some(existing) = self.files.get(&requested_path) {
            if existing == &bytes {
                return requested_path;
            }
        }
        let mut candidate = requested_path.clone();
        while let Some(existing) = self.files.get(&candidate) {
            if existing == &bytes {
                return candidate;
            }
            candidate = hashed_variant_path(&requested_path, &bytes);
        }
        self.record_case(&candidate);
        self.files.insert(candidate.clone(), bytes);
        candidate
    }
    /// O(depth) replacement for the old `canonicalize_output_path_case` scan.
    fn canonicalize_case(&self, requested_path: &str) -> String {
        let mut lower_prefix = String::new();
        let mut parts = Vec::new();
        for (depth, part) in requested_path.split('/').enumerate() {
            if depth > 0 {
                lower_prefix.push('/');
            }
            lower_prefix.push_str(&part.to_ascii_lowercase());
            let canonical = self
                .case_index
                .get(&lower_prefix)
                .cloned()
                .unwrap_or_else(|| part.to_string());
            parts.push(canonical);
        }
        parts.join("/")
    }
    /// Record each prefix of a freshly stored path so later paths adopt its case.
    fn record_case(&mut self, stored_path: &str) {
        let mut lower_prefix = String::new();
        for (depth, part) in stored_path.split('/').enumerate() {
            if depth > 0 {
                lower_prefix.push('/');
            }
            lower_prefix.push_str(&part.to_ascii_lowercase());
            self.case_index
                .entry(lower_prefix.clone())
                .or_insert_with(|| part.to_string());
        }
    }
}
```

- [ ] **Step 4: Run the test** — `cargo test -p starbreaker-3d --lib output_files_canonicalizes 2>&1 | tail -6` → `ok. 1 passed`.

- [ ] **Step 5: Commit** — `git add -A && git commit -m "Add OutputFiles with O(depth) case canonicalization index"`

---

### Task 2: Migrate the writers to `OutputFiles`

**Files:** Modify `crates/starbreaker-3d/src/decomposed.rs`.

**Interfaces:**
- Consumes: `OutputFiles` (Task 1).
- Produces: every `files: &mut BTreeMap<String, Vec<u8>>` parameter becomes `files: &mut OutputFiles`; every `files: &BTreeMap<String, Vec<u8>>` (the `canonicalize_output_path_case` reader) is removed (folded into the struct).

- [ ] **Step 1: Delete the now-unused free functions and migrate `insert_binary_file`/`insert_json_file`**

Delete `fn canonicalize_output_path_case`, `fn existing_segment_case`, and the body of `fn insert_binary_file`. Replace `insert_binary_file` and `insert_json_file` with thin shims:
```rust
fn insert_binary_file(files: &mut OutputFiles, requested_path: String, bytes: Vec<u8>) -> String {
    files.insert_canonical(requested_path, bytes)
}

fn insert_json_file(files: &mut OutputFiles, requested_path: String, value: serde_json::Value) -> String {
    let bytes = serde_json::to_vec_pretty(&value).unwrap_or_else(|_| b"{}".to_vec());
    files.insert_canonical(requested_path, bytes)
}
```
(Keep `hashed_variant_path` — it's used by `insert_canonical`.)

- [ ] **Step 2: Change every other writer's `files` parameter type**

In each of these functions, change the parameter `files: &mut BTreeMap<String, Vec<u8>>` to `files: &mut OutputFiles`: `write_mesh_asset`, `write_material_sidecar`, `extract_material_entry`, `build_slot_export_value`, `export_texture_asset`, `generated_ui_texture_for_binding`, `finalize_palette_records`, `insert_ui_export_stamp`. Their bodies already call only `files.contains_key`, `files.get`, and `insert_binary_file`/`insert_json_file` — all of which now operate on `OutputFiles` — so no body changes are needed beyond the type.

- [ ] **Step 3: Migrate `write_decomposed_export`'s local map + final conversion**

In `write_decomposed_export`, change `let mut files = BTreeMap::new();` to `let mut files = OutputFiles::new();`. The final conversion (the `.map(|(relative_path, bytes)| ExportedFile { ... })` near the end, ~line 2025) iterates `files`; change its source from `files` / `files.into_iter()` to `files.iter()` and clone (`relative_path.clone()`, `bytes.clone()`) since `iter()` borrows. Any other in-function `files.insert(...)` / `files.contains_key(...)` calls stay valid (methods exist on `OutputFiles`); a raw `files.insert(k, v)` with no canonicalization must become `insert_binary_file(&mut files, k, v)` to preserve behaviour — grep `files.insert(` in the function and convert each.

- [ ] **Step 4: Fix the unit tests that build a raw `files` map**

Grep `let mut files = BTreeMap::new();` and `insert_binary_file_reuses_identical_content` in the test module; change those `files` to `OutputFiles::new()` and any direct `files.insert`/`files.get`/`files.keys` assertions to the `OutputFiles` methods (`get`, `contains_key`, `iter`). For the existing `insert_binary_file_reuses_identical_content_and_hashes_collisions` test, it should pass unchanged in behaviour through the shim.

- [ ] **Step 5: Build** — `cargo build --release -p starbreaker 2>&1 | grep -E "error|warning:|Finished"` → `Finished`, no errors/warnings. Fix any remaining `BTreeMap`/method mismatches the compiler flags (they are mechanical).

- [ ] **Step 6: Run the lib tests** — `cargo test -p starbreaker-3d --lib 2>&1 | grep "test result:"` → all `ok`.

- [ ] **Step 7: Byte-identical gate + canonicalize timing**

```bash
export SC_DATA_P4K="$HOME/Games/star-citizen/drive_c/Program Files/Roberts Space Industries/StarCitizen/LIVE/Data.p4k"
rm -rf target/tmp/iar_p1 && mkdir -p target/tmp/iar_p1
/usr/bin/time -v env RUST_LOG=info ./target/release/starbreaker entity export "AEGS_Idris_P" target/tmp/iar_p1 --kind decomposed --lod 0 > target/tmp/iar_p1.log 2>&1
diff -rq target/tmp/iar_baseline target/tmp/iar_p1 | grep -v export_stamp
grep -aE "interior_asset_resolve:|decomposed\] total:|Elapsed \(wall" target/tmp/iar_p1.log
```
Expected: diff **empty**; interior_asset_resolve down ~3s; wall ~49s.
If diff non-empty, the index produced a different canonical case than the old scan — STOP and compare `canonicalize_case` vs the old `canonicalize_output_path_case` for the differing path.

- [ ] **Step 8: Commit** — `git add -A && git commit -m "Use OutputFiles index for O(depth) path canonicalization"`

---

## PHASE 2 — Parallel interior sidecar build

### Task 3: Thread a read-only pre-warmed cache through the sidecar writers

**Files:** Modify `crates/starbreaker-3d/src/decomposed.rs`.

**Interfaces:**
- Produces: `write_material_sidecar`, `extract_material_entry`, `build_slot_export_value`, `export_texture_asset` each gain a `prewarmed: &PngCache` parameter (inserted right after their existing `png_cache: &mut PngCache` parameter). In `export_texture_asset`, the decode consults `prewarmed` first.

- [ ] **Step 1: Add `prewarmed` to `export_texture_asset` and use it**

Add `prewarmed: &PngCache,` after `png_cache: &mut PngCache,`. In the body, the `let bytes = match flavor { ... cached_load_keyed(... png_cache ...) }?;` block becomes (read the pre-warmed map first; only fall back to the live/`png_cache` decode on a miss):
```rust
    let (discriminator, loader): (&str, fn(&MappedP4k, &str, u32) -> Option<Vec<u8>>) =
        match flavor {
            TextureFlavor::Generic => ("", crate::pipeline::load_diffuse_texture),
            TextureFlavor::Normal => ("@n", crate::pipeline::load_normal_texture),
        };
    let prewarm_key = format!("{source_path}@mip{texture_mip}{discriminator}");
    let bytes = match prewarmed.get(&prewarm_key) {
        Some(cached) => cached.clone(),
        None => crate::pipeline::cached_load_keyed(p4k, source_path, texture_mip, discriminator, png_cache, loader),
    }?;
```
(This preserves the exact key format from `prewarm_decomposed_textures`.)

- [ ] **Step 2: Thread `prewarmed` through the three callers**

Add `prewarmed: &PngCache,` after `png_cache: &mut PngCache,` to `write_material_sidecar`, `extract_material_entry`, and `build_slot_export_value`. At each internal call, pass `prewarmed` in the matching position: `extract_material_entry(files, p4k, png_cache, prewarmed, texture_cache, ...)`, `build_slot_export_value(files, p4k, png_cache, prewarmed, texture_cache, ...)`, `export_texture_asset(files, p4k, png_cache, prewarmed, texture_cache, ...)`. (`build_slot_export_value` also calls `export_texture_asset` for the direct-diffuse path and `extract_material_entry` calls `export_texture_asset` for direct diffuse/normal — pass `prewarmed` at all of them.)

- [ ] **Step 3: Update the serial call sites of `write_material_sidecar`**

`write_material_sidecar` is called in three places in `write_decomposed_export` (root, child, interior). Each currently passes `&mut png_cache`. Pass the same `&png_cache` again as the new `prewarmed` argument for now — i.e. `write_material_sidecar(&mut files, p4k, &mut png_cache, &png_cache, &mut texture_cache, ...)`. (Borrow-checker note: `&mut png_cache` and `&png_cache` cannot co-exist. Instead pass an empty placeholder for `prewarmed` at the serial sites: define `let empty_prewarm = PngCache::new();` once near the top of `write_decomposed_export` and pass `&empty_prewarm` as `prewarmed`. The serial path then behaves exactly as before — `prewarmed` always misses, falls back to `png_cache`.)

- [ ] **Step 4: Build** — `cargo build --release -p starbreaker 2>&1 | grep -E "error|warning:|Finished"` → `Finished`.

- [ ] **Step 5: Byte-identical gate** (the prewarm is empty at serial sites, so output must be unchanged):
```bash
export SC_DATA_P4K="$HOME/Games/star-citizen/drive_c/Program Files/Roberts Space Industries/StarCitizen/LIVE/Data.p4k"
rm -rf target/tmp/iar_p2a && mkdir -p target/tmp/iar_p2a
./target/release/starbreaker entity export "AEGS_Idris_P" target/tmp/iar_p2a --kind decomposed --lod 0 >/dev/null 2>&1
diff -rq target/tmp/iar_baseline target/tmp/iar_p2a | grep -v export_stamp
```
Expected: **empty**.

- [ ] **Step 6: Commit** — `git add -A && git commit -m "Thread read-only prewarmed cache through sidecar writers"`

---

### Task 4: Make `write_material_sidecar` callable into a local map and expose a per-asset builder

**Files:** Modify `crates/starbreaker-3d/src/decomposed.rs`.

**Interfaces:**
- Produces: `pub(crate) fn build_interior_sidecar(p4k: &MappedP4k, prewarmed: &PngCache, entry: &InteriorCgfEntry, sidecar_materials: &MtlFile, sidecar_original_indices: &[u32], palettes_manifest_path: &str, lod_level: u32, texture_mip: u32, format: ExportFormat, existing_asset_paths: Option<&HashSet<String>>) -> (String /*mesh_asset*/, Option<String> /*material_sidecar*/, OutputFiles)` — builds ONE unique interior asset's mesh-asset placeholder + material sidecar into a fresh `OutputFiles`, using local png/texture/mtl caches and the shared read-only `prewarmed`. Mirrors the cache-miss arm of the interior loop.

- [ ] **Step 1: Implement `build_interior_sidecar`**

Extract the cache-miss body of the interior loop (the `else { ... }` arm that builds `mesh_asset` + `material_sidecar` for a fresh asset) into this function, operating on a local `let mut files = OutputFiles::new();`, `let mut png_cache = PngCache::new();`, `let mut texture_cache = HashMap::new();`, `let mut mtl_cache = HashMap::new();`, and the passed `prewarmed`. Return `(mesh_asset, material_sidecar, files)`. The exact body is the existing `write_mesh_asset(...)` + the `material_sidecar` `write_material_sidecar(...)` block (now passing `&prewarmed`) + the `reuse_existing_mesh_asset` logic. Copy it verbatim from the interior loop's cache-miss arm, substituting the local `files`/caches and `prewarmed`.

- [ ] **Step 2: Build** — `cargo build --release -p starbreaker 2>&1 | grep -E "error|warning:|Finished"`. Expected `Finished` (a `never used` warning on `build_interior_sidecar` is acceptable until Task 6).

- [ ] **Step 3: Commit** — `git add -A && git commit -m "Extract build_interior_sidecar for one unique interior asset"`

---

### Task 5: Parallel pre-build of unique interior sidecars in `blend_assembly`

**Files:** Modify `crates/starbreaker-3d/src/pipeline/blend_assembly.rs`, `crates/starbreaker-3d/src/decomposed.rs` (`write_decomposed_export` signature + interior loop consumption).

**Interfaces:**
- Consumes: `build_interior_sidecar` (Task 4), the existing `prewarmed_png_cache` already built in `blend_assembly`.
- Produces: a new parameter on `write_decomposed_export`: `prebuilt_interior_assets: HashMap<String, (String, Option<String>, OutputFiles)>` keyed by `interior_asset_lookup_key(normalized_cgf_path, normalized_material_path)` (the SAME key the interior loop computes). The interior loop, on a cache miss, looks this up; on hit it merges the asset's `OutputFiles` into the shared `files` and reuses the `(mesh_asset, material_sidecar)`; on miss (not prebuilt) it falls back to the existing inline build.

- [ ] **Step 1: Build the map in `blend_assembly`**

After the texture prewarm block (after `prewarmed_png_cache` is built, before `write_decomposed_export`), iterate `input.interiors.unique_cgfs` in parallel, building each asset's sidecar with `crate::decomposed::build_interior_sidecar(p4k, &prewarmed_png_cache, entry, &sidecar_materials, &sidecar_original_indices, …)`, keyed by `interior_asset_lookup_key`. Reuse the `build_decomposed_material_view` call already added for the prewarm to get `sidecar_materials`/`sidecar_original_indices`. Collect into `HashMap<String, (String, Option<String>, OutputFiles)>`. Log `[timing][blend] prebuild_interior_sidecars: {:.2}s ({} assets)`. Pass this map into `write_decomposed_export`.

- [ ] **Step 2: Consume the map in the interior loop**

In `write_decomposed_export`'s interior loop, in the cache-miss arm, first check `prebuilt_interior_assets.get(&cache_key)`. On hit: for each `(path, bytes)` in the prebuilt `OutputFiles.iter()`, `insert_binary_file(&mut files, path.clone(), bytes.clone())` (merges in first-occurrence order, deduping); set `(mesh_asset, material_sidecar)` from the prebuilt tuple; insert into `interior_asset_cache`. On miss: keep the existing inline build.

- [ ] **Step 3: Build** — `cargo build --release -p starbreaker 2>&1 | grep -E "error|warning:|Finished"` → `Finished`.

- [ ] **Step 4: Byte-identical gate + speedup**
```bash
export SC_DATA_P4K="$HOME/Games/star-citizen/drive_c/Program Files/Roberts Space Industries/StarCitizen/LIVE/Data.p4k"
rm -rf target/tmp/iar_p2 && mkdir -p target/tmp/iar_p2
/usr/bin/time -v env RUST_LOG=info ./target/release/starbreaker entity export "AEGS_Idris_P" target/tmp/iar_p2 --kind decomposed --lod 0 > target/tmp/iar_p2.log 2>&1
diff -rq target/tmp/iar_baseline target/tmp/iar_p2 | grep -v export_stamp
grep -aE "prebuild_interior_sidecars:|interior_asset_resolve:|decomposed\] total:|Elapsed \(wall" target/tmp/iar_p2.log
```
Expected: diff **empty**; interior_asset_resolve drops sharply; wall well below ~49s.
If the diff is non-empty, the merge order or a per-task path diverged from serial — STOP, identify the differing file, and check (a) the merge is in `unique_cgfs` first-occurrence order, (b) the local-vs-shared canonicalization matches (case conflicts across assets).

- [ ] **Step 5: Commit** — `git add -A && git commit -m "Build interior sidecars in parallel; merge into the writer"`

---

### Task 6: Cross-ship verification + suites

- [ ] **Step 1:** Verify byte-identical on `RSI_Aurora_Mk2` and `DRAK_Cutlass_Black` (export on `HEAD~5` and `HEAD`, diff — same pattern as prior tasks).
- [ ] **Step 2:** `cargo test -p starbreaker-3d -p starbreaker-ui --release 2>&1 | grep -E "test result:|FAILED" | grep -v "0 failed"` → only the pre-existing `manifest_targets_whole_image_colour_regression_guard` staleness failure.
- [ ] **Step 3:** Update `~/.claude/projects/-home-tom-projects-scorg-tools/memory/idris-export-perf.md`; `git push origin feature/ui`.

---

## Self-Review

**Spec coverage:** Phase 1 (Tasks 1-2) = O(1) canonicalization via `OutputFiles`. Phase 2 (Tasks 3-5) = thread read-only prewarm → extract per-asset builder → parallel pre-build + merge. Task 6 = verification. Matches the chosen "both, sequentially" scope.

**Placeholder scan:** Task 4 Step 1 and Task 5 Steps 1-2 say "copy the existing cache-miss arm verbatim / reuse the existing block" — these reference concrete, named existing code (the interior loop's cache-miss arm; the prewarm's `build_decomposed_material_view`) rather than vague TODOs; the migration in Task 2 is a uniform, fully-specified type change. No "TBD"/"handle errors".

**Type consistency:** `OutputFiles` methods (`new`, `contains_key`, `get`, `len`, `iter`, `insert_canonical`) consistent Tasks 1-5. `prewarmed: &PngCache` inserted after `png_cache: &mut PngCache` consistently across `export_texture_asset`/`extract_material_entry`/`build_slot_export_value`/`write_material_sidecar` (Task 3) and consumed in `build_interior_sidecar` (Task 4). `prebuilt_interior_assets: HashMap<String, (String, Option<String>, OutputFiles)>` keyed by `interior_asset_lookup_key` consistent between Task 5 Steps 1 (produced in blend_assembly) and 2 (consumed in the loop). The prewarm key format `"{source_path}@mip{texture_mip}{discriminator}"` (Task 3 Step 1) matches `prewarm_decomposed_textures` (existing).
