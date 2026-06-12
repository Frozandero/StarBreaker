# Scene Generation from JSON — `generate_scene_from_json.py`

This Blender Python script generates complete `scene.blend` files from StarBreaker's decomposed `scene.json` export format.

## Problem Solved

Blender 5.1 moved from a legacy block-based format to an incompatible binary format. Manual `.blend` file construction via Python block APIs is no longer viable. This script uses Blender's native Python API (`bpy`) to properly construct scenes.

## Requirements

- **Blender 5.1+** (5.1.1 verified)
- Python 3.10+
- A valid `scene.json` file from StarBreaker decomposed export

## Usage

### Command Line

```bash
blender --background --python generate_scene_from_json.py -- \
  --scene-json <path/to/scene.json> \
  --output <path/to/scene.blend>
```

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `--scene-json` | Yes | Path to the `scene.json` file containing hierarchy and metadata |
| `--output` | Yes | Output path for the generated `scene.blend` file |

### Examples

#### Generate Aurora Mk2 Scene

```bash
blender --background --python scripts/generate_scene_from_json.py -- \
  --scene-json ships/Packages/RSI\ Aurora\ Mk2_LOD0_TEX0/scene.json \
  --output aurora_mk2_scene.blend
```

#### Generate Other Ships

```bash
# Mole
blender --background --python scripts/generate_scene_from_json.py -- \
  --scene-json target/tmp/mole_clean_export_20260421/Packages/DRAK\ Mole/scene.json \
  --output mole_scene.blend

# Talon
blender --background --python scripts/generate_scene_from_json.py -- \
  --scene-json target/tmp/talon_clean_export_20260421/Packages/ANVL\ Talon/scene.json \
  --output talon_scene.blend
```

## What Gets Generated

### Collections

The script organizes objects into hierarchical collections:

```
StarBreaker/
  ├── Package Root (empty object)
  ├── children/
  │   ├── Child 1 (empty, with geometry_path, mesh_asset metadata)
  │   ├── Child 2
  │   └── ...
  ├── interiors_<interior_name>/
  │   ├── Interior Anchor (empty container)
  │   ├── Light objects (with proper types, colors, energy)
  │   ├── Placement anchors (interior instances)
  │   └── ...
  └── interiors_<other_interior>/
```

### Objects

**Package Root:**
- Empty object (CUBE display type) named `StarBreaker <EntityName>`
- Marked with `starbreaker_package_root = True`

**Child Attachments:**
- Empty objects (PLAIN_AXES) with transforms from `local_transform_sc` or fallback offset
- Metadata stored as custom properties:
  - `geometry_path` — P4k path to the source geometry
  - `mesh_asset` — Path to the mesh .blend/.glb asset
  - `material_sidecar` — Material metadata JSON path
  - `starbreaker_entity_name` — Original entity name

**Interior Containers:**
- Empty objects (CUBE) named `interior_<name>`
- Transform applied from `container_transform`
- Children include lights and placement anchors

**Lights:**
- Proper Blender light objects (not empties) with datablocks
- Light type mapped from CryEngine types:
  - `Omni` / `SoftOmni` → `POINT`
  - `Projector` → `SPOT`
  - `Ambient` → `SUN`
- Properties set:
  - **Color** — Linear RGB from scene.json `color`
  - **Energy** — Converted from `intensity_candela_proxy` (rough approximation)
  - **Position** — From CryEngine to Blender coordinates
  - **Rotation** — Quaternion with axis conversion
  - **Radius** — Soft shadow size
  - **Spot angles** — For spotlights (inner/outer angle)
- Parented to interior container
- Metadata stored as custom properties:
  - `starbreaker_light_type` — Original CryEngine type (`Omni`, `Ambient`, etc.)
  - `starbreaker_light_name` — Light name

**Placements (Interior Instances):**
- Empty objects inside interior containers
- Transform from `transform` field
- `starbreaker_placement_entity` custom property

## Coordinate System Conversion

The script handles CryEngine ↔ Blender coordinate conversion:

- **Position:** `(x, y, z)_ce` → `(x, y, -z)_blender`
- **Quaternion:** `(w, x, y, z)_ce` → `(w, x, y, -z)_blender`
- **Matrix:** Applies Z-flip axis conversion via `axis_conversion @ matrix @ axis_conversion`

## Metadata Preservation

The script stores scene.json data as custom properties on objects for later retrieval:

```python
anchor["starbreaker_entity_name"] = "rsi_aurora_mk2_landing_gear_front.cga"
anchor["geometry_path"] = "Data/Objects/Spaceships/..."
anchor["mesh_asset"] = "Data/Objects/.../aurora_mk2_landing_gear_front_LOD0.blend"
anchor["material_sidecar"] = "Data/Objects/.../rsi_aurora_mk2_TEX0.materials.json"
```

Blender addons can use `starbreaker_scene_path` custom property to later load and materialize these references.

## Energy Conversion Strategy

The script uses a simple energy mapping (subject to refinement):

| Light Type | Conversion |
|-----------|-----------|
| `POINT` | `intensity_candela_proxy / 100.0` |
| `SUN` | `intensity_candela_proxy / 10000.0` |
| `SPOT` | `intensity_candela_proxy / 100.0` |

This is a rough approximation and may need adjustment based on reference renders in-engine vs. Blender.

## Error Handling

The script gracefully handles:

- Missing `scene.json` file → `FileNotFoundError` with path
- Malformed JSON → JSON decode error with traceback
- Missing optional fields in scene.json → Falls back to defaults (empty transforms, no lights, etc.)
- Invalid matrix/quaternion data → Defaults to identity matrix/quaternion

All errors are printed to stderr with full Python traceback for debugging.

## Output File Format

The generated `.blend` file is:

- **Blender 5.1 native format** (Zstandard-compressed binary)
- **Immediately importable** via `bpy.ops.wm.open_mainfile()` or File > Open
- **Ready for further processing** by Blender addons (material reconstruction, mesh linkage, etc.)

## Limitations & Future Work

### Current Limitations

1. **Mesh instances** — Created as empty placeholders (MESH objects with empty datablocks). The actual geometry must be linked/instantiated separately.
2. **Material instances** — Metadata stored but not materialized. Material reconstruction is left to the Blender addon.
3. **Energy mapping** — Simple linear conversion; may need per-light-type refinement.
4. **Light cookies/gobos** — `projector_texture` metadata stored but not applied to light node-graph.
5. **Light states** — Only the active state is imported. State-switching UI is left to the addon.

### Future Enhancements

- [ ] Link actual mesh assets from `.blend`/`.glb` files
- [ ] Apply materials from sidecar JSON (integrate with `material_contract.py`)
- [ ] Enhanced energy mapping based on reference renders
- [ ] Light cookie texture application (gobo node-graph setup)
- [ ] State-switcher UI controls for lights

## Testing

### Quick Verification

After generation, verify the scene structure in Blender:

```python
import bpy
scene = bpy.context.scene
print(f"Objects: {len(bpy.data.objects)}")
print(f"Collections: {len(bpy.data.collections)}")
lights = [obj for obj in bpy.data.objects if obj.type == "LIGHT"]
print(f"Lights: {len(lights)}")
```

### Known Good Output

**Aurora Mk2 (LOD0, TEX0):**
- Total objects: ~132
- Child attachments: 67
- Lights: 62
- Collections: 3 (StarBreaker, children, interiors_x2)

## Integration with Blender Addon

The generated scene is designed for integration with StarBreaker's Blender addon:

1. **`package_ops.py`** can detect `starbreaker_scene_path` custom property to load instances
2. **Importer orchestration** can materialize geometry and materials from stored paths
3. **Light runtime setup** can apply state-switching UI and gobo textures

See `StarBreaker/blender_addon/AGENTS.md` for addon integration details.

## Troubleshooting

### "scene.json not found"
- Verify the path is correct and file exists
- Use absolute paths to avoid cwd issues

### "OSError: Python file not found"
- Ensure `--` separator is used before script arguments
- Example: `blender --background --python script.py -- --arg value`

### Empty or minimal scene
- Check that scene.json is not truncated or malformed
- Use `python3 -m json.tool scene.json` to validate JSON structure

### Lights not appearing
- Verify scene.json includes an `interiors` section with `lights` entries
- Check light positions are not all at origin (may be outside viewport)

## See Also

- `docs/decomposed-export-contract.md` — Scene.json schema and format
- `docs/StarBreaker/lights-research.md` — Light property reference
- `blender_addon/starbreaker_addon/runtime/importer/` — Material and geometry integration
- `AGENTS.md` — Blender addon and exporter architecture
