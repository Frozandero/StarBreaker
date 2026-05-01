# Socpak Decomposed Export Whitepaper

This document is a code-grounded implementation brief for extending
StarBreaker's standalone `socpak` exporter so it can:

- export every connected `.socpak` needed by a location or object
  container graph,
- emit the same decomposed package contract used by ship/entity exports,
- reuse the existing Blender addon material, texture, light, palette,
  animation, and POM reconstruction pipeline wherever possible.

It is written for future agents and maintainers. The goal is to make the
next implementation pass predictable rather than rediscovering the current
export stack.

## Status

The current standalone socpak CLI path is GLB-only and texture-poor.
It can export multiple `.socpak` files when the user's substring happens
to match them, but it does not walk references from one socpak to another.

The entity decomposed exporter already exports ship interiors that come
from socpaks, including material sidecars, exported textures, light
records, palettes, liveries, and Blender-readable placement records. The
best path forward is to make standalone socpak export produce that same
package shape instead of adding a second Blender importer.

## Relevant Code Map

- `cli/src/entity.rs` - entity CLI export entry point. Handles bundled vs
  decomposed output writing.
- `cli/src/socpak.rs` - current standalone `starbreaker socpak export`
  entry point. Always writes one GLB.
- `cli/src/common.rs` - shared `ExportOpts` mapping into
  `starbreaker_3d::ExportOptions`.
- `crates/starbreaker-3d/src/pipeline.rs` - main export orchestration,
  `assemble_glb_with_loadout_with_progress`, `load_interiors`,
  `build_interiors_from_payloads`, and `socpaks_to_glb`.
- `crates/starbreaker-3d/src/socpak.rs` - `.socpak` ZIP reader, `.soc`
  parser, IncludedObjects and CryXML entity extraction.
- `crates/starbreaker-3d/src/included_objects.rs` - IncludedObjects chunk
  parser.
- `crates/starbreaker-3d/src/decomposed.rs` - decomposed package writer,
  manifest builder, material sidecar writer, texture exporter, interior
  placement writer.
- `docs/decomposed-export-contract.md` - public decomposed package
  contract.
- `blender_addon/starbreaker_addon/manifest.py` - Blender-side
  `scene.json`, `palettes.json`, `liveries.json`, material sidecar, and
  path resolution dataclasses.
- `blender_addon/starbreaker_addon/runtime/importer/orchestration.py` -
  Blender decomposed package import loop, root/child/interior placement,
  light creation.
- `blender_addon/starbreaker_addon/runtime/importer/builders.py` -
  material template wiring, POM support, decal offset behavior.

## Current Entity Bundled And Decomposed Export Flow

Entity export starts in `cli/src/entity.rs`. The CLI resolves an
`EntityClassDefinition`, resolves its loadout tree, maps shared CLI
options into `ExportOptions`, then calls
`assemble_glb_with_loadout_with_progress`.

Inside `pipeline.rs`, the entity flow has these major stages:

1. Resolve root and loadout geometry from DataCore.
2. Export the root payload and attachment payloads.
3. Discover interior object containers via
   `socpak::query_object_containers`, then load each container with
   `socpak::load_interior_from_socpak`.
4. Convert loaded socpak payloads into `LoadedInteriors` through
   `build_interiors_from_payloads`.
5. Branch on `ExportOptions.kind`.

For `ExportKind::Bundled`, the pipeline preloads interior meshes and
textures where useful, then writes a single GLB through the glTF writer.

For `ExportKind::Decomposed`, the pipeline calls
`decomposed::write_decomposed_export`. Decomposed export deliberately
keeps the earlier root and child mesh payload pass light by using
`MaterialMode::Colors`, then performs richer material and texture work
while writing material sidecars. The result is an `ExportResult` whose
`glb` field is empty and whose `decomposed` field contains a list of
`ExportedFile` records.

The CLI writes those `ExportedFile` records into a caller-selected export
root:

```text
<export-root>/
  Packages/<package-name>_LOD<lod>_TEX<mip>/
    scene.json
    palettes.json
    liveries.json
    paints.json              # optional
    animations/*.json        # optional
  Data/
    .../*.glb
    .../*.materials.json
    .../*.png
    .../*.dds                # currently used for light gobos
```

This package contract is what the Blender addon understands today.

## Current Standalone Socpak Export Flow

The standalone CLI path is much smaller:

1. `cli/src/socpak.rs` receives a case-insensitive substring pattern.
2. It lists all P4k entries whose names contain that pattern and end in
   `.socpak`.
3. It calls `starbreaker_3d::socpaks_to_glb`.
4. It writes the returned bytes to `<pattern>.glb` or the user-provided
   output path.

`socpaks_to_glb` currently ignores the decomposed package writer even if
the shared CLI options contain `--kind decomposed`. It returns a `Vec<u8>`
GLB either way.

The socpak loader itself does this:

```text
Data.p4k
  -> outer .socpak entry
    -> inner ZIP/P4k archive
      -> every .soc entry
        -> CrCh chunks
          -> IncludedObjects: static CGF placements, material paths, tint palettes
          -> CryXMLB: entity geometry, EntityClassGUIDs, lights
```

Important current behavior:

- All `.soc` files inside the selected `.socpak` are parsed. This covers
  the common "main soc has static geometry, child socs have lights/VFX"
  pattern.
- Multiple `.socpak` files are exported only when the user's substring
  matches multiple P4k entries.
- References from one `.socpak` to another are not followed.
- DataCore `VehicleComponentParams.objectContainers` is only used by the
  entity export path, not by standalone `socpak export`.
- Standalone `socpaks_to_glb` uses `MaterialMode::Colors` when loading
  interior CGFs and gives the glTF writer a `load_textures` callback that
  always returns `None`. This is why the current output looks like
  colors-only even when shared CLI material options imply textures.

## Fixture Evidence: Orison Landing Zone Interior

The local fixture `D:\SCTOOLS\dev\orison_ind_lz_int.socpak` confirms how
at least one real location socpak expresses connected containers.

Archive summary:

- 105 inner entries.
- 1 main `.soc`: `orison_ind_lz_int.soc` at about 7.9 MB.
- 80 `entdata/*.entxml` CryXmlB entity records.
- 7 embedded brush `.cgf` files and 7 matching `.cgfm` files.
- 1 readable root metadata file: `orison_ind_lz_int.xml`.
- 1 CryXmlB room mapping file: `orison_ind_lz_int.rmxml`.
- 1 editor metadata file: `orison_ind_lz_int_editor.xml`.

The main `.soc` contains a large static-location payload. String-level
inspection found:

- 528 unique-looking geometry references containing `.cgf`, `.cga`, or
  `.skin`.
- 4 material references:
  `objects/buildingsets/human/universal/prop/flag/flag_orison_1_1x4_k.mtl`,
  `objects/buildingsets/human/hightech/prop/sign/sign_infoscreen_4_b.mtl`,
  `objects/buildingsets/human/hightech/loc/stanton/orison/orison_palette_a.mtl`,
  and `materials/vfx/hologram_intersect_glow.mtl`.
- 19 light texture references under `textures/lights/.../*.dds`.

The connected socpak references are not hidden in the main `.soc` static
placement strings. They appear in two higher-level metadata forms that the
current loader ignores:

1. `orison_ind_lz_int.xml` has a top-level `ChildObjectContainers` list
   with six `<Child external="1" ...>` records. Each record has a
   `name="Data/objectcontainers/.../*.socpak"`, `entityName`, `class`,
   `classGuid`, `guid`, `pos`, and `rot`.
2. Six `entdata/*.entxml` CryXmlB entity records repeat the same links as
   `Entity/PropertiesDataCore/EntityComponentObjectContainer` nodes with
   `__type="SObjectContainerComponentParams"` and an `objectContainer`
   attribute such as
   `objectcontainers/pu/shops/admin/admin_small_orison_a.socpak`.

The six confirmed child socpaks are:

- `Data/objectcontainers/setup/spawn_closet/spawncloset_static_elev_orison_007.socpak`
- `Data/objectcontainers/setup/spawn_closet/spawncloset_static_elev_orison_008.socpak`
- `Data/objectcontainers/pu/shops/shippart/cousincrows/cousincrows_orison.socpak`
- `Data/objectcontainers/pu/shops/clothing/providence_surplus/providence_surplus_orison_int.socpak`
- `Data/objectcontainers/pu/shops/courier/covalex/covalex_orison_int.socpak`
- `Data/objectcontainers/pu/shops/admin/admin_small_orison_a.socpak`

This fixture changes the graph-discovery priority. The first connected
socpak implementation should parse `ChildObjectContainers` from the root
metadata XML and `EntityComponentObjectContainer` from `entdata/*.entxml`
before spending time on unknown IncludedObjects object types.

It also exposes a transform choice that implementation must settle:

- `ChildObjectContainers/Child` records carry container-local `pos` and
  `rot` values alongside canonical `Data/.../*.socpak` targets.
- The matching `entdata/*.entxml` entity records carry `Pos` and `Rotate`
  values plus the non-`Data/` `objectContainer` path.

The safest first pass is to treat the root XML `ChildObjectContainers`
list as authoritative for graph edges and transforms, then use `entxml`
records as a cross-check and fallback for socpaks that lack the root XML
list.

## Existing Interior Model

The useful shared model is `LoadedInteriors` in `pipeline.rs`.

`LoadedInteriors` contains:

- `unique_cgfs`: deduplicated `InteriorCgfEntry` records. Each carries a
  CGF path, optional material path, and display name.
- `containers`: one `InteriorContainerData` per socpak payload. Each
  carries the container transform, placements, lights, and an optional
  tint palette.

`build_interiors_from_payloads` is shared by entity interior discovery and
standalone socpak GLB export. It resolves EntityClassGUID geometry through
DataCore and expands loadout-attached child geometry for interior gadgets.
That means standalone socpak decomposed export should reuse this path
rather than inventing a parallel placement model.

There are limitations to fix or at least document before relying on it
for high-fidelity locations:

- CGFs are currently deduplicated by CGF path alone in
  `build_interiors_from_payloads`. If the same CGF appears with different
  material overrides, the first material path can win for later
  placements.
- `included_objects_to_meshes` currently assigns
  `io.material_paths.first()` to every IncludedObjects placement. The
  IncludedObjects chunk has a full material path list, but the code does
  not yet know which object field selects a material index.
- Only the first payload-level tint palette path is used for a container.
  Per-placement palette references in raw IncludedObjects are not modeled.
- IncludedObjects object types `0x7` and `0x10` are skipped. If those
  represent visual geometry or nested container references in some
  locations, the graph will remain incomplete until decoded.

## Blender Addon Contract

The Blender addon imports decomposed packages, not raw socpaks. Its
entry point is `STARBREAKER_OT_import_decomposed_package`, which calls
`PackageBundle.load` and then `PackageImporter.import_scene`.

Current hard requirements:

- `scene.json` must exist.
- sibling `palettes.json` must exist.
- sibling `liveries.json` must exist.
- `paints.json` is optional.
- asset paths are resolved relative to the export root using the package
  rule and a case-insensitive path index.

The addon already imports `scene.interiors`:

- creates an anchor per interior container,
- applies `container_transform`,
- creates an anchor per placement,
- applies each placement `transform`,
- instantiates the referenced mesh GLB,
- rebuilds object materials from each placement's material sidecar,
- applies placement or container palette IDs,
- creates lights from the exported light records.

Therefore, standalone socpak support should target the existing
decomposed manifest shape. A separate `socpak` Blender operator is only
needed as a convenience wrapper, not as a separate data model.

One important compatibility detail: `SceneInstanceRecord.mesh_asset` is
optional. If the root entity has no mesh, `instantiate_scene_instance`
turns it into an empty/sphere anchor. That makes a synthetic socpak root
practical, provided the actual geometry lives in `interiors`.

## Target Architecture

The implementation should add a decomposed socpak package path alongside
the existing GLB path:

```text
starbreaker socpak export <pattern-or-path> <output-root> --kind decomposed
```

For backwards compatibility, `--kind bundled` should keep writing a
single GLB. For `--kind decomposed`, the output argument should be treated
as a shared export root, just like `entity export --kind decomposed`.

The Rust API should not make callers fake an entity. Add an explicit
standalone socpak export function, for example:

```rust
pub fn socpaks_to_decomposed(
    db: &Database,
    p4k: &MappedP4k,
    roots: &[String],
    opts: &ExportOptions,
    graph_options: SocpakGraphOptions,
    existing_asset_paths: Option<&HashSet<String>>,
) -> Result<DecomposedExport, Error>
```

That function should:

1. Resolve the root pattern or explicit paths into canonical `.socpak`
   paths.
2. Build a connected socpak graph.
3. Load every graph node into `InteriorPayload`.
4. Convert the payloads through `build_interiors_from_payloads`.
5. Write a decomposed package with a synthetic scene root and all loaded
   socpaks represented as interior containers.

The current `decomposed::DecomposedInput` requires root geometry,
material, mesh, and entity metadata. Do not work around this by writing
fake geometry paths into the manifest. Prefer one of these refactors:

- Add a `SceneRoot` enum to `DecomposedInput`:
  `EntityRoot { ... }` or `SyntheticRoot { name, source_kind }`.
- Add a separate `SocpakDecomposedInput` and share the lower-level helper
  functions for mesh assets, material sidecars, palette manifests, livery
  manifests, lights, and interior records.
- Extract the interior-writing portion of `write_decomposed_export` into
  a reusable `write_interior_package_assets` helper.

The synthetic root should be explicit in JSON:

```json
{
  "entity_name": "socpak:<label>",
  "geometry_path": null,
  "material_path": null,
  "mesh_asset": null,
  "material_sidecar": null,
  "palette_id": null
}
```

The addon already tolerates a root instance with no mesh asset.

## Connected Socpak Graph Discovery

The missing feature is graph traversal. The current loader only parses
`.soc` members inside the currently selected `.socpak`; it does not find
other `.socpak` archives referenced by those members.

The implementation should introduce a graph discovery pass before loading
payloads:

```text
root pattern/path
  -> canonical root .socpak path(s)
    -> open root socpak
      -> inspect every inner .soc CryXMLB and IncludedObjects chunk
        -> collect referenced socpak/object-container paths
          -> canonicalize to Data/.../*.socpak
            -> repeat until queue is empty
```

The discovery API should be separate from payload loading so it can be
tested on tiny fixtures without decoding every mesh:

```rust
pub struct SocpakGraph {
    pub roots: Vec<SocpakGraphNodeId>,
    pub nodes: Vec<SocpakGraphNode>,
    pub edges: Vec<SocpakGraphEdge>,
    pub warnings: Vec<SocpakGraphWarning>,
}

pub struct SocpakGraphNode {
    pub canonical_path: String,
    pub display_name: String,
}

pub struct SocpakGraphEdge {
    pub from: String,
    pub to: String,
    pub source_soc: Option<String>,
    pub source_kind: SocpakReferenceKind,
    pub transform: [[f32; 4]; 4],
}
```

Reference sources to investigate:

- Root metadata XML files such as `orison_ind_lz_int.xml` with
  `ObjectContainer/ChildObjectContainers/Child` records. In the Orison
  fixture this is the clearest source of connected socpak paths and
  transforms.
- `entdata/*.entxml` CryXmlB entity records with
  `PropertiesDataCore/EntityComponentObjectContainer` nodes whose
  `objectContainer` attribute points at a child `.socpak`. The Orison
  fixture repeats the same six children this way.
- CryXML entities whose class or component represents an object container.
  Current code only extracts geometry, EntityClassGUIDs, and lights, so
  `ObjectContainer` and `ObjectContainerModifier` entities are not modeled
  as graph edges.
- Any other `PropertiesDataCore` child under CryXML that contains a
  `fileName`, `objectContainer`, `ObjectContainer`, or `.socpak` path.
- IncludedObjects object types currently skipped as `0x7` and `0x10`.
  These may encode non-Type1 object references, helper data, portals, or
  container links. The Orison fixture already proves child-container links
  can be recovered without these types, so decode them after the XML and
  entxml paths are covered.
- DataCore object-container records reachable from EntityClassGUIDs found
  inside socpak CryXML. The entity path already knows how to read
  `VehicleComponentParams.objectContainers`; the standalone path may need a
  more generic query helper for non-vehicle records if CIG stores location
  links on other component types.

Graph traversal rules:

- Canonicalize paths before deduplication. Use the same casing and
  `Data/...` path normalization rules used by P4k lookup.
- Track visited canonical paths to avoid cycles.
- Preserve the edge list even when a target is missing, so the manifest can
  include warnings and future debugging has provenance.
- Apply per-edge transforms when the source reference has one. If the
  source reference has no transform, identity is acceptable and should be
  recorded as such.
- Keep root pattern expansion separate from connected traversal. A pattern
  matching ten socpaks and a single root socpak that references nine others
  are different provenance stories.

## Proposed Decomposed Manifest Additions

Do not fork the manifest schema for socpaks. Add fields that older importers
can ignore.

Recommended `scene.json` additions:

```json
{
  "export_kind": "Decomposed",
  "source_kind": "SocpakGraph",
  "socpak_graph": {
    "roots": ["Data/ObjectContainers/.../root.socpak"],
    "nodes": [
      {
        "path": "Data/ObjectContainers/.../root.socpak",
        "name": "root"
      }
    ],
    "edges": [
      {
        "from": "Data/ObjectContainers/.../root.socpak",
        "to": "Data/ObjectContainers/.../child.socpak",
        "source_soc": "root.soc",
        "source_kind": "cryxml_object_container",
        "transform": [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]]
      }
    ],
    "warnings": []
  }
}
```

Recommended interior container additions:

```json
{
  "name": "root",
  "source_socpak": "Data/ObjectContainers/.../root.socpak",
  "source_soc_files": ["root.soc", "root_lights.soc"],
  "palette_id": "palette/...",
  "container_transform": [[...]],
  "placements": [],
  "lights": []
}
```

The Blender addon will ignore unknown fields because `manifest.py` keeps
the raw dictionary and only reads known fields. Future UI can expose graph
provenance from `scene.raw["socpak_graph"]` without changing the importer
core.

Even if a socpak has no palettes or liveries, emit minimal manifests:

```json
{ "version": 1, "palettes": [] }
```

```json
{ "version": 1, "liveries": [] }
```

This keeps `PackageBundle.load` working without weakening the existing
addon contract.

## Texture And Material Strategy

The immediate texture gap is not that socpak data cannot support textures;
it is that the standalone GLB path turns texture loading off.

For decomposed socpak export, use the same strategy as entity decomposed
interiors:

- load each interior CGF enough to obtain mesh, MTL, and NMC data,
- write mesh GLBs without embedding textures,
- write `*.materials.json` sidecars from the MTL,
- export direct texture references from sidecars into `Data/.../*.png`,
- preserve raw DDS for light projector/gobo textures when needed,
- register palettes and livery usage for every placement with a palette.

Do not try to solve every material fidelity issue in the first graph pass.
The Blender addon already knows how to interpret decomposed material
sidecars, POM families, decals, texture transforms, DDNA normal/gloss
alpha, layers, palettes, and livery switching. The exporter must give it
the same sidecar inputs it receives for ships.

Known material correctness work that should be tracked separately:

- Decode IncludedObjects material selection instead of applying the first
  material path to every placement.
- Deduplicate interior assets by `(cgf_path, material_path)` everywhere,
  not only in the decomposed writer's local cache.
- Preserve per-placement palette data if the source format exposes it.
- Revisit skipped IncludedObjects object types once connected-container
  fixtures are available.

## Blender Addon Work

The first decomposed socpak exporter should require little Blender code.
The addon already handles root instances with no mesh and imports
`scene.interiors`.

Recommended addon changes for polish and robustness:

- Add a small UI label or custom property for `source_kind:
  "SocpakGraph"` so users can tell a standalone location package from a
  ship package.
- Add tests with a non-ship decomposed fixture:
  synthetic root, no root mesh, empty palettes/liveries, one interior
  placement, one light.
- Avoid adding path-based socpak special cases to material reconstruction.
  Existing heuristics such as `/ships/`, `/interior/`, and `_int_master`
  should be treated as legacy ship heuristics. New socpak behavior should
  prefer explicit metadata in `scene.json` or material sidecars.
- Watch path-index performance. `PackageBundle._build_path_index` scans
  the entire export root. Large connected location exports may make this
  noticeably slower if the shared `Data/` tree grows very large.

Do not add a second importer that reads `.socpak` directly inside Blender.
That would duplicate Rust's P4k, CryXMLB, IncludedObjects, material, and
texture logic and would lose the reusable package cache that decomposed
export already provides.

## CLI Behavior

Recommended user-facing behavior:

```text
starbreaker socpak export <pattern-or-path> [output] --kind bundled
starbreaker socpak export <pattern-or-path> [output-root] --kind decomposed
```

Bundled mode:

- Keep current behavior by default.
- Continue writing one `.glb`.
- Consider adding `--connected` after the graph traversal code exists.

Decomposed mode:

- Treat output as an export root, not a file path.
- Default output root to the sanitized pattern if omitted.
- Write `Packages/<label>_LOD<lod>_TEX<mip>/scene.json`.
- Write reusable assets under `Data/...`.
- Reuse `--skip-existing-assets` behavior from entity decomposed export.
- Error if the output path is an existing file.
- Print the graph summary: root count, connected socpak count, skipped
  missing references, emitted file count.

Important compatibility fix: if `--kind decomposed` is accepted by the
shared `ExportOpts`, `socpak export` must not silently write a GLB. Until
decomposed socpak export is implemented, the CLI should reject
`--kind decomposed` for `socpak export` with a clear error.

## Implementation Phases

### Phase 1 - Correct The Current CLI Contract

- In `cli/src/socpak.rs`, branch on `export_opts.kind`.
- Keep bundled behavior for `ExportKind::Bundled`.
- Return a clear "not implemented" error for `ExportKind::Decomposed`
  until the decomposed path lands.
- Add a CLI test or command-level unit test if the project has a suitable
  harness.

### Phase 2 - Extract Reusable Decomposed Interior Writing

- Refactor `decomposed.rs` so interior asset writing can run without a real
  entity root.
- Keep entity decomposed output byte-for-byte equivalent where practical.
- Add Rust tests around synthetic-root manifest generation:
  no root mesh, empty palettes/liveries, one interior placement.

### Phase 3 - Standalone Socpak Decomposed Export Without Graph Traversal

- Add `socpaks_to_decomposed` for explicit socpak path lists.
- Use identity transforms for root socpaks, matching today's
  `socpaks_to_glb`.
- Emit the existing decomposed contract plus `source_kind: "SocpakGraph"`
  and a graph with only root nodes.
- Wire `cli/src/socpak.rs` decomposed mode to write the file list.
- Verify import in Blender with `bpy.ops.starbreaker.import_decomposed_package`.

### Phase 4 - Connected Socpak Graph Traversal

- Add a discovery pass that opens socpaks and extracts references without
  loading mesh assets.
- Parse root object-container XML files for
  `ChildObjectContainers/Child` records, including `name`, `entityName`,
  `class`, `classGuid`, `guid`, `pos`, and `rot`.
- Parse `entdata/*.entxml` CryXmlB files for
  `EntityComponentObjectContainer` records, including `objectContainer`
  and the entity-level transform. Use these as a fallback or cross-check
  against the root XML child list.
- Add fixture tests for cycles, missing targets, duplicate references, and
  path canonicalization.
- Preserve graph provenance in `scene.json`.
- Add `--connected` or make connected traversal the decomposed default with
  `--no-connected` as an escape hatch.

### Phase 5 - Material And Palette Fidelity

- Decode IncludedObjects per-object material selection.
- Fix CGF dedupe so material overrides cannot be lost.
- Preserve additional palette routing where the source format supports it.
- Expand tests to cover repeated CGFs with different material paths.

### Phase 6 - Addon UX And Performance

- Add a non-ship fixture package to `blender_addon/tests`.
- Display `source_kind` and graph counts where useful.
- Consider lazy path indexing or manifest-provided asset lists if large
  location packages make import slow.

## Test Plan

Rust unit and integration tests:

- `socpak export --kind decomposed` does not return bundled bytes.
- Synthetic-root decomposed manifest parses as a valid `SceneManifest`.
- Empty `palettes.json` and `liveries.json` are emitted for palette-free
  socpaks.
- Interior placements keep their `cgf_path`, `material_path`,
  `mesh_asset`, `material_sidecar`, `transform`, and `palette_id`.
- Graph traversal deduplicates cycles and records missing targets as
  warnings.
- Repeated CGF with two material paths emits two independent material
  sidecars or otherwise preserves the material distinction.

Blender addon tests:

- `PackageBundle.load` accepts a synthetic-root package.
- `PackageImporter.import_scene` imports an interior-only package without a
  root mesh.
- Materials rebuild from sidecars for interior placements.
- Lights import under interior anchors.
- Palette-free packages do not crash palette/livery UI helpers.

Manual validation:

1. Export a known ship using `entity export --kind decomposed` and confirm
   no regression in Blender.
2. Export one standalone socpak without connected traversal and import it.
3. Export a location with connected traversal and compare container count,
   mesh count, light count, and missing-reference warnings against debug
   logs.
4. Inspect a few materials with diffuse/normal/POM/decal slots to confirm
   sidecar texture paths resolve under `Data/...`.

## Open Questions

- Do the root XML `ChildObjectContainers/Child` transforms or the matching
  `entdata/*.entxml` entity transforms best match in-game placement for
  connected socpaks? The Orison fixture has both, and their values are not
  identical.
- Are IncludedObjects object types `0x7` and `0x10` visual placements,
  object-container links, portals, or something else?
- Should bundled `socpak export` eventually include textures, or should
  high-fidelity work focus exclusively on decomposed export plus Blender?
- Should graph traversal be default for decomposed mode? The likely answer
  is yes once traversal is reliable, with an explicit `--no-connected`
  option for debugging.

## Design Principles

- Reuse the decomposed package contract. Blender should import socpak
  packages through the same path as ship packages.
- Keep graph discovery separate from mesh/material export. Discovery should
  be cheap, testable, and provenance-rich.
- Prefer additive manifest fields. Older importers should keep working.
- Preserve source paths and warnings. Missing or unknown references should
  be visible, not silently ignored.
- Fix format interpretation at the source. Avoid named-asset workarounds or
  path-specific branches for individual locations.
