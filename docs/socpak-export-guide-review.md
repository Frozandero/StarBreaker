# SOCPAK Structure Export Guide — Review & Gap Tracker

Source document: `docs/socpak-structure-export-guide.md`
Reviewed: 2026-05-05
Reviewer context: assessed against the needs of building a socpak-to-exporter plumbing
layer that connects to the existing decomposed export pipeline (`InteriorPayload`,
`InteriorContainerData`, `scene.json`, material sidecars).

## Overall Assessment

The guide is thorough as a research/exploration artifact. It does an excellent job
documenting what exists inside socpaks and what the high-level composition model
looks like. However, as a **specification for building a plumbing layer**, several
sections stop at "this thing exists and here's why it matters" without providing
the concrete data shapes, XML element names, binary offsets, or mapping rules that
a developer would need to write code against.

The gaps fall into three categories:

1. **Missing concrete schemas** — describes what data exists but not its exact
   structure/encoding
2. **Missing mapping rules** — how socpak data maps into the existing
   `InteriorPayload` / decomposed-export types
3. **Missing error/edge-case guidance** — what happens with malformed data,
   cycles, missing references, etc.

---

## Section-by-Section Review

### Scope (lines 1–23) — GOOD

Clear, well-scoped, lists evidence sources. No gaps.

### Sample Set (lines 24–59) — GOOD

Excellent table with rationale, file counts, and second-tier references.
Sufficient for a developer to reproduce the investigation.

### High-Level Model (lines 67–93) — MINOR GAP

**What's good**: Clearly describes the ZIP-within-P4K archive shape and lists
each file type.

**What's missing**:

- The exact `.soc` chunk structure — the document says "Contains
  `IncludedObjects` chunks with..." but doesn't enumerate all possible chunk
  types and their CrCh type IDs. A developer needs to know what chunk type IDs
  to watch for. The code in `included_objects.rs` only handles types `0x0001`,
  `0x0007`, `0x0010` — the document should state this is the known set.
- No mention of whether multiple `.soc` files can appear in a single socpak and
  how to determine which is "primary" vs "child". The existing code
  (`socpak.rs` line 137–141) iterates ALL `.soc` entries — the document should
  clarify this is intentional.

### File Roles (lines 95–109) — MODERATE GAP

**What's good**: The table maps each file extension to its header magic,
observed structure, and export relevance.

**What's missing**:

- **`.soc` CryXMLB entity chunk structure**: The document says the `.soc`
  contains "CryXMLB → entities, lights, possible decals/screens/text candidates"
  but doesn't show the actual XML element hierarchy. A developer needs to know:
  what is the root tag? What are the child tags? Where exactly do `Entity`
  elements live? The code shows it looks for `<Entities>` or `<SCOC_Entities>`
  containers — this should be in the document.
- **`entdata/*.entxml` structure**: Same issue — described as "per-entity
  records" but no example XML structure showing what attributes/children an
  entxml entity actually has. The code in `socpak.rs` reads `EntityClass`,
  `Name`, `Pos`, `Rotate`, `Scale`, `Material`, `EntityClassGUID`, and nested
  `PropertiesDataCore` children — none of this is shown.
- **`.rmxml`, `.brmp`, `.altg`**: All described as "not currently understood."
  This is acceptable for a first pass but the document should explicitly flag
  these as needing reverse-engineering if they turn out to carry
  culling/grouping data.

### Object-Container Composition (lines 131–221) — MODERATE GAP

**What's good**: Excellent examples of child references, instancing, and the
`ObjectContainer` vs `ObjectContainerModifier` distinction. The coordinate-frame
mismatch between root manifest and entdata is correctly noted.

**What's missing**:

- **How to parse the root XML `<ChildObjectContainers>`**: The document shows
  XML snippets but doesn't specify the complete attribute set. What attributes
  does a `<Child>` element have besides `name`, `class`, `entityName`, `pos`,
  `rot`, `external`? Are there optional attributes like `scale`, `layer`,
  `flags`? The developer needs the full schema.
- **Transform composition formula**: The document says "compose parent transform
  with child pos/rot/scale" but doesn't state the formula. Is it
  `parent_world × child_local`? What about scale inheritance? The existing code
  uses `build_container_transform` with Ang3 Euler rotation (degrees, ZYX order)
  — but the root manifest child `pos`/`rot` are CSV f64 position + quaternion,
  not Euler. The code in `socpak.rs` line 1013 uses Euler degrees from
  DataCore's `ObjectContainerRef`, while the XML uses quaternions. This
  discrepancy needs to be documented and resolved.
- **Cycle detection strategy**: Mentioned as "detect cycles by path stack" but
  doesn't specify: what constitutes a cycle? Is it same socpak path appearing
  twice? Same entity GUID? What's the expected behavior when a cycle is found
  — skip, error, or warn?
- **Maximum depth or instance count guidance**: The hangar example places one
  child twice, the rest stop places one child eight times. Is there a practical
  limit the pipeline should enforce? No guidance.

### Mesh Sources (lines 223–286) — MODERATE GAP

**What's good**: Correctly identifies the three mesh paths (IncludedObjects,
entity geometry, local brush).

**What's missing**:

- **IncludedObjects material-per-object mapping**: The document notes the
  current code "assigns the first IncludedObjects material path to every object
  placement" and calls this "too coarse." But it doesn't explain what the
  CORRECT mapping is. Looking at the `IncludedObjects` struct, there's a flat
  list of material paths and a flat list of objects — how should a specific
  object's material be determined? Is there a material index on each object?
  The `IncludedObject` struct has `unknown2` which might be a material index.
  The document should investigate and specify this.
- **Type 7 and Type 10 objects**: Documented as "currently skipped and still
  unknown" — but this is critical data. The document should at minimum provide
  hex dumps or byte-by-byte analysis of a few Type 7/10 objects so the
  implementer has something to work from. Without knowing what these represent,
  the pipeline will silently drop unknown visual data.
- **Entity geometry resolution path**: The document mentions
  `PropertiesDataCore → EntityGeometryResource → Geometry → Geometry → Geometry @path`
  but doesn't show what other geometry paths exist. Some entities might use
  `SGeometryResourceParams` instead (the DataCore path used by
  `resolve_guid_geometry`). The document should enumerate all known geometry
  resolution paths.

### Vertex Colors (lines 287–296) — GOOD

Short but sufficient. Correctly identifies that vertex colors come from the mesh
loader, not the socpak, and that materials with `VERTCOLORS`/`VCOL` flags need
them.

### Materials and Textures (lines 298–331) — MODERATE GAP

**What's good**: Lists all material sources and texture lookup priority.

**What's missing**:

- **Inner-archive texture resolution mechanics**: The document says "check the
  inner socpak" but doesn't specify how to match. Is it by exact relative path?
  By filename? The inner archive has paths like
  `admin_small_orison_a/cubemaps/asoa_light_int/cm_shop_admin_orison_small_int_probe_001_cm.dds`
  — but the material `.mtl` might reference just the DDS filename. How should
  the resolution work?
- **No example of a material-to-socpak-local-texture binding**: The document
  shows the embedded cubemap paths but never shows the actual `.mtl` or material
  chunk that references them. Without seeing the binding, a developer can't
  write the lookup logic.

### Tint Palettes (lines 333–353) — MINOR GAP

**What's good**: Correctly identifies per-container palette scope and child
palette independence.

**What's missing**:

- **Palette-to-mesh binding**: How does a specific mesh instance know which
  palette to use? The IncludedObjects has a flat list of palette paths — is it
  one palette per container? Per object? The code in `interiors.rs` takes only
  the first palette (`payload.tint_palette_names.first()`) — is that correct?
  The document should clarify.

### Decals (lines 355–414) — SIGNIFICANT GAP

**What's good**: Distinguishes mesh decals from decal projectors. Notes that
editor XML decal counts are insufficient.

**What's missing**:

- **Decal projector data structure**: The document says "The actual transform,
  projection size, material, and flags are likely in the `.soc` CryXMLB entity
  payload" but doesn't confirm or provide the structure. This is critical — a
  developer needs to know: what CryXMLB element represents a decal? What
  attributes does it have? What's the entity class name? (`Decal`?
  `DecalEntity`?) Without this, the pipeline cannot implement decal support.
- **No `.soc` CryXMLB decal example**: The document provides extensive examples
  for lights, screens, and text but not a single example of an actual decal
  entity from `.soc` CryXMLB data.

### POM (lines 416–441) — GOOD

Correctly identifies POM as a material feature, not a socpak concern. Sufficient
guidance.

### Text, Signs, and Screens (lines 443–704) — THOROUGH but has gaps

**What's good**: This is the most detailed section. The `text_surface` schema
(lines 674–700) is excellent — it provides a concrete data shape for the
plumbing layer. The four categories are well-distinguished. The DataCore entity
class defaults for screens are thoroughly researched.

**What's missing**:

- **`UserVariablesComponentParams` schema**: Referenced repeatedly as the source
  for `locStrings`, `fontsize`, `Color_R/G/B/A`, etc., but no example of what
  this component actually looks like in CryXMLB. A developer needs to know the
  exact XML element name, attribute names, and child structure.
- **`UICanvasDecalDescriptorEntityComponentParams` schema**: Same issue —
  mentioned as carrying the canvas reference but no example.
- **`LiteralStringProviderComponentParams`**: Mentioned but not shown.
- **Building Blocks canvas resolution**: The document says "parse
  `UIBuildingBlocksEntityComponentParams`, canvas records" but doesn't show how
  to find the canvas DataCore record, what struct it uses, or how to extract
  variable bindings from it. The example with
  `BuildingBlocks_BindingsLocalizedVariable` and
  `BuildingBlocks_BindingsLocalizedField` is mentioned in prose but not shown as
  a data structure.
- **Localization key resolution**: The document says to look up `@key` in
  `global.ini` but doesn't specify the P4K path to that file
  (`Data\Localization\english\global.ini` — mentioned in sample set but not in
  the pipeline section).

### Lights (lines 706–727) — MINOR GAP

**What's good**: Correctly notes editor counts aren't render data, identifies
the `.soc` CryXMLB as the correct source.

**What's missing**:

- **No light entity XML example**: For all the detailed light-parsing code that
  exists, the document doesn't show what a CryXMLB light entity actually looks
  like. A developer implementing the socpak plumbing layer would benefit from a
  concrete example of a `Light`, `LightBox`, and `LightGroup` entity from `.soc`
  data, with the `EntityComponentLight` child structure.

### Non-Visual Data to Filter (lines 728–746) — GOOD

Clear, practical list. The existing `SKIP_ENTITY_CLASSES` in `socpak.rs` aligns
well with this guidance.

### Recommended Decomposed Export Pipeline (lines 748–827) — MODERATE GAP

**What's good**: The 17-step pipeline is well-structured and covers the full
flow from P4K lookup to scene graph emission.

**What's missing**:

- **Step ordering dependencies**: Some steps can run in parallel (e.g., steps
  8–11), some are strictly sequential (steps 4–6). The document doesn't specify
  which, making it hard to design for performance.
- **Data flow between steps**: No specification of what data structure each step
  produces and passes to the next. For example, what does step 5 "Build a
  container node" produce? What are its fields? The existing code uses
  `InteriorPayload` and `InteriorContainerData` — the document doesn't mention
  these or specify how the new pipeline's data shapes relate to them.
- **Integration with existing decomposed types**: The document doesn't explain
  how socpak container nodes should map to the existing `scene.json` schema from
  `decomposed-export-contract.md`. Should each socpak become a scene? A
  sub-scene? Additional `interior_containers` entries? This mapping is critical
  for the plumbing layer.
- **Step 6 specifics on IncludedObjects parsing**: Says "parse as CrCh chunk
  file" but doesn't mention the existing `IncludedObjects::from_bytes()` parser.
  A developer would benefit from knowing that this parser already exists and
  what it returns.
- **Step 12 inner-archive vs outer-P4K priority**: Mentioned but the priority is
  unclear. Should inner-archive always be checked first, or only when the path
  looks relative?

### Current StarBreaker Gaps (lines 829–866) — GOOD

The strongest section for implementation guidance. Accurately identifies real
code gaps against the desired pipeline. Minor note:

- It lists gaps but doesn't prioritize them. Which are blocking vs nice-to-have?
- Some gaps (like "does not understand `.brmp` or `.altg`") may never need to be
  addressed for visual export — the document should flag these.

### Practical Rules (lines 868–900) — GOOD

Clear, actionable rules. Well-aligned with the coding practices in AGENTS.md.

---

## Gap Tracker

### P0 — Blocking (must resolve before plumbing work starts)

| # | Section | Gap | What's needed | Status |
|---|---------|-----|---------------|--------|
| G1 | Object-Container Composition | Transform composition between root XML children and existing code | Exact formula showing how root XML `pos`/`rot` quaternion maps to the Euler-based `build_container_transform`, or a decision to use quaternion-based composition throughout | Open |
| G2 | Decals | Decal projector entity structure | At least one concrete `.soc` CryXMLB decal entity example with full attribute list; the entity class name, attributes, and child structure for `Decal` / `DecalEntity` | Open |
| G3 | Recommended Pipeline | Mapping to existing decomposed types | Explicit mapping: socpak container → `scene.json` fields, socpak mesh → `InteriorPayload`/`InteriorMesh`, socpak child → what scene graph node. Which existing types are reused vs which are new | Open |

### P1 — Important (resolve during plumbing implementation)

| # | Section | Gap | What's needed | Status |
|---|---------|-----|---------------|--------|
| G4 | Mesh Sources | Material-per-object mapping in IncludedObjects | Investigation of Type 1 object `unknown2` field — is it a material index? Alternative mapping strategy if not. Current code assigns first material to all objects, which is wrong for multi-material containers | Open |
| G5 | Mesh Sources | Type 7 / Type 10 object structure | Hex dump or structural analysis of a few Type 7 and Type 10 objects from sampled socpaks. What data do they carry? Are they visual? | Open |
| G6 | File Roles | CryXMLB entity structure for `.soc` | Example showing `<Entities>` / `<SCOC_Entities>` → `<Entity>` hierarchy with all common attributes (`EntityClass`, `Name`, `Pos`, `Rotate`, `Scale`, `Material`, `EntityClassGUID`, `PropertiesDataCore` children) | Open |
| G7 | Text/Screens | Component schemas for text entities | At least one concrete CryXMLB example showing `UserVariablesComponentParams`, `UICanvasDecalDescriptorEntityComponentParams`, and `LiteralStringProviderComponentParams` — exact XML element names, attribute names, and child structure | Open |
| G8 | Object-Container Composition | Full `<Child>` attribute schema | Complete list of attributes on `<Child>` elements in `<ChildObjectContainers>` — is there `scale`, `layer`, `flags`, or other attributes beyond `name`, `class`, `entityName`, `pos`, `rot`, `external`? | Open |

### P2 — Deferred (nice to have, can investigate later)

| # | Section | Gap | What's needed | Status |
|---|---------|-----|---------------|--------|
| G9 | Materials | Inner-archive texture resolution | Example of a material referencing a socpak-embedded DDS and the lookup path — how does the `.mtl` reference the inner-archive texture? | Open |
| G10 | File Roles | `.brmp` / `.altg` format notes | Binary header magic, rough byte structure, even if "skip for now" | Open |
| G11 | Object-Container Composition | Cycle detection and max depth | Concrete strategy (same socpak path in ancestor stack? entity GUID?) and practical limits (max recursion depth, max instance count) | Open |
| G12 | Lights | Light entity CryXMLB examples | Concrete XML examples of `Light`, `LightBox`, `LightGroup` entities from `.soc` CryXMLB showing `EntityComponentLight` child structure | Open |
| G13 | High-Level Model | Multiple `.soc` files in one socpak | Clarify that iterating all `.soc` entries is intentional, and explain why some socpaks have more than one | Open |
| G14 | Tint Palettes | Palette-to-mesh binding | Clarify: is it one palette per container, or per object? How should multi-palette IncludedObjects be handled? | Open |
| G15 | Text/Screens | Building Blocks canvas resolution | Data structure for `BuildingBlocks_Canvas` records, `BuildingBlocks_BindingsLocalizedVariable` / `BuildingBlocks_BindingsLocalizedField` — how to walk from entity to resolved text | Open |
| G16 | Recommended Pipeline | Step parallelism and data flow | Which steps can run concurrently, and what data shape each step produces (or point to existing types like `InteriorPayload` that should be reused) | Open |

### Not a gap — no action needed

- Scope, Sample Set, Vertex Colors, POM, Non-Visual Data to Filter, Practical
  Rules, Current StarBreaker Gaps — all sufficient as-is.
