# GLB Export Parity

## Goal

The bundled GLB exporter is a one-shot, software-agnostic consumer of the same
source material and scene data used by the decomposed Blender workflow. The GLB
must not require the StarBreaker Blender add-on or any sidecar files at runtime.

The decomposed material contract remains the high-fidelity reference. GLB maps
that contract to core glTF metallic-roughness PBR, standard Khronos extensions,
embedded PNGs, and diagnostic `extras` for source semantics that glTF cannot
represent directly.

## Phase 1: Materials And Textures

Status: implemented for the portable PBR surface and texture path.

The bundled exporter now:

- resolves the equipped paint/material override before decoding root textures;
- bakes the selected concrete palette into layered source textures;
- embeds diffuse, normal, DDNA-derived metallic-roughness, emissive, occlusion,
  decal, and generated UI images in the GLB;
- preserves authored `TexMod` scale and offset with `KHR_texture_transform`,
  including normal and occlusion texture infos;
- preserves secondary UV selection;
- maps palette finish glossiness to PBR roughness when no roughness map exists;
- maps authored/palette specular response through `KHR_materials_specular`;
- maps authored glass values through `KHR_materials_transmission`,
  `KHR_materials_ior`, and `KHR_materials_volume`;
- maps high emission through `KHR_materials_emissive_strength`;
- renders source-backed UI bindings once per unique binding, then specializes
  shared screen materials by NMC helper so different screens can carry different
  embedded images;
- preserves shader family, palette routing, layers, public parameters, authored
  XML fields, activation state, and source identity in material `extras`;
- omits invented screen images and unauthored glass volume values.

`--materials textures` is the normal portable export mode. `all` remains for
compatibility with callers that explicitly request every available reconstruction
path.

### Portable Representation Limits

One static GLB cannot retain editable palettes or arbitrary Blender/Cycles node
graphs. It contains the concrete equipped paint and baked portable maps. POM,
multi-layer wear, stencil composition, and other Star Engine shader behavior are
flattened where source data permits and retained as semantic `extras` otherwise.

Animated `TexMod` oscillator blocks remain metadata-only; static scale and offset
are exported. The legacy metallic classifier in `mtl.rs` also remains an evidence
gap: it derives conductor state from authored response values because no explicit
Star Engine metallic flag has yet been identified. Replace that classifier when
the engine rule or an explicit source field is verified; do not add asset-name
exceptions or more thresholds.

## Phase 2: Animation Parity Plan

Phase 2 exports standard glTF skins and animation channels into the same GLB.
It reuses the existing `.chrparams` / DBA / CAF / Mannequin discovery and clip
decoding used by decomposed animation sidecars.

| Work item | Deliverable | Depends on |
|---|---|---|
| 2A Skin data | Preserve source joint indices and normalized weights in `Mesh`; emit `JOINTS_0`, `WEIGHTS_0`, inverse bind matrices, and glTF `skins` for root and child entities. | Phase 1 stable mesh packing |
| 2B Shared clip model | Refactor clip discovery/decoding into a serializer-neutral animation set used by both decomposed JSON and GLB. Preserve clip names, source timing, bone hashes, fragments, tags, and events. | Existing animation decoder |
| 2C Joint binding | Resolve every clip channel to the correct entity-local skin joint by source skeleton identity and bone hash/name. Report unresolved and ambiguous channels; never bind by a guessed global node name. | 2A, 2B |
| 2D glTF channels | Emit animation samplers and translation/rotation channels with timestamps in seconds, glTF quaternion ordering, and coordinate conversion consistent with the exported bind pose. | 2C |
| 2E Scene coverage | Add clips for root, attachments, landing gear, and animated interior CGA/CHR entities without node-name collisions between entity instances. | 2D |
| 2F Semantic metadata | Store Mannequin fragments, tags, scopes, source paths, and event markers in animation `extras`; standard playback remains available to any glTF consumer. | 2B, 2D |
| 2G Validation | Compare first/final sampled GLB poses against decomposed sidecars, run glTF validation, import in Blender without the add-on, and test at least one root, attachment, and interior animation. | 2E, 2F |

Dependencies are sequential through 2D. After 2D, 2E and 2F can proceed
independently; 2G requires both.

### Phase 2 Acceptance Criteria

- A bundled export contains at least one valid glTF skin when the source mesh is
  skinned, with no joint index outside the skin's joint array.
- Every exported animation sampler has monotonic input times and matching input
  and output counts.
- Bone transforms at the first and final source samples match the decomposed
  animation path within documented floating-point tolerances.
- Duplicate bone names in different entity instances do not cross-bind.
- Unresolved channels are counted and preserved in diagnostics, not silently
  attached to another node.
- Blender and a second glTF consumer can play the standard channels without the
  StarBreaker add-on.

## Validation Commands

```bash
cargo test -p starbreaker-3d --lib
cargo test --workspace
cargo build --release -p starbreaker
SC_DATA_P4K=<path-to-Data.p4k> \
  ./target/release/starbreaker entity export <entity> <output> \
  --kind bundled --materials textures
```
