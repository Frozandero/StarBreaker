#!/usr/bin/env python3
"""
Blender Python script to generate scene.blend from scene.json exported data.

Usage:
    blender --background --python generate_scene_from_json.py -- \
        --scene-json <path/to/scene.json> \
        --output <path/to/scene.blend>

This script creates a Blender scene with:
- Empty objects for hierarchy containers
- Mesh instances (with placeholder empty geometry) for each child
- Light objects with proper rotation and colors
- Proper parenting and collections
"""

import sys
import json
import argparse
import traceback
from pathlib import Path
from typing import Any, Optional

import bpy
from mathutils import Matrix, Quaternion, Vector


def parse_arguments() -> tuple[str, str]:
    """Parse command-line arguments for Blender --python mode."""
    # In Blender --python mode, arguments come after '--'
    if "--" in sys.argv:
        argv = sys.argv[sys.argv.index("--") + 1 :]
    else:
        argv = sys.argv[1:]

    parser = argparse.ArgumentParser(
        description="Generate Blender scene from scene.json export"
    )
    parser.add_argument(
        "--scene-json",
        required=True,
        type=str,
        help="Path to scene.json file",
    )
    parser.add_argument(
        "--output",
        required=True,
        type=str,
        help="Path to output scene.blend file",
    )

    args = parser.parse_args(argv)
    return args.scene_json, args.output


def scene_position_to_blender(position: list[float]) -> Vector:
    """Convert CryEngine position (X, Y, Z) to Blender (X, Y, -Z)."""
    if not position or len(position) < 3:
        return Vector((0.0, 0.0, 0.0))
    x, y, z = position[0], position[1], position[2]
    return Vector((x, y, -z))


def scene_quaternion_to_blender(rotation: list[float]) -> Quaternion:
    """
    Convert CryEngine quaternion to Blender quaternion with axis conversion.
    CryEngine uses (W, X, Y, Z) format.
    Blender uses (W, X, Y, Z) but with axis conversion: (W, X, Y, -Z).
    """
    if not rotation or len(rotation) < 4:
        return Quaternion((1.0, 0.0, 0.0, 0.0))

    w, x, y, z = rotation[0], rotation[1], rotation[2], rotation[3]
    # Apply axis conversion for CryEngine Z-up to Blender Z-up
    return Quaternion((w, x, y, -z))


def scene_matrix_to_blender(matrix_rows: list[list[float]]) -> Matrix:
    """Convert a 4x4 matrix from CryEngine format to Blender format."""
    if not matrix_rows or len(matrix_rows) != 4:
        return Matrix.Identity(4)

    # Construct matrix from rows
    try:
        mat = Matrix(
            [
                (matrix_rows[0][0], matrix_rows[0][1], matrix_rows[0][2], matrix_rows[0][3]),
                (matrix_rows[1][0], matrix_rows[1][1], matrix_rows[1][2], matrix_rows[1][3]),
                (matrix_rows[2][0], matrix_rows[2][1], matrix_rows[2][2], matrix_rows[2][3]),
                (matrix_rows[3][0], matrix_rows[3][1], matrix_rows[3][2], matrix_rows[3][3]),
            ]
        )

        # Apply axis conversion: negate Z column and row
        # This converts from CryEngine Z-up to Blender Z-up coordinate system
        axis_conversion = Matrix(
            (
                (1.0, 0.0, 0.0, 0.0),
                (0.0, 1.0, 0.0, 0.0),
                (0.0, 0.0, -1.0, 0.0),
                (0.0, 0.0, 0.0, 1.0),
            )
        )
        # Result = axis_conversion @ matrix @ axis_conversion
        return axis_conversion @ mat @ axis_conversion

    except (TypeError, ValueError):
        return Matrix.Identity(4)


def light_type_to_blender(light_type: str) -> str:
    """Map CryEngine light type to Blender light type."""
    type_map = {
        "Omni": "POINT",
        "SoftOmni": "POINT",
        "Projector": "SPOT",
        "Ambient": "SUN",
    }
    return type_map.get(light_type, "POINT")


def create_root_collection(scene: bpy.types.Scene) -> bpy.types.Collection:
    """Create or get the StarBreaker collection."""
    root_col = bpy.data.collections.get("StarBreaker")
    if root_col is None:
        root_col = bpy.data.collections.new("StarBreaker")
        scene.collection.children.link(root_col)
    return root_col


def ensure_collection(
    parent_col: bpy.types.Collection, name: str
) -> bpy.types.Collection:
    """Ensure a collection exists under parent."""
    col = bpy.data.collections.get(name)
    if col is not None:
        # Collection exists, but make sure it's linked if needed
        if col.name not in parent_col.children:
            parent_col.children.link(col)
        return col

    col = bpy.data.collections.new(name)
    parent_col.children.link(col)
    return col


def create_empty_mesh() -> bpy.types.Mesh:
    """Create an empty mesh datablock."""
    mesh = bpy.data.meshes.new("Mesh")
    return mesh


def create_mesh_instance(
    name: str,
    position: Vector,
    rotation: Quaternion,
    scale: Vector,
    collection: bpy.types.Collection,
) -> bpy.types.Object:
    """Create a mesh object instance with placeholder empty geometry."""
    mesh = create_empty_mesh()
    obj = bpy.data.objects.new(name, mesh)

    collection.objects.link(obj)

    obj.location = position
    obj.rotation_quaternion = rotation
    obj.scale = scale

    return obj


def create_light_object(
    light_data: dict[str, Any],
    interior_anchor: Optional[bpy.types.Object],
    collection: bpy.types.Collection,
) -> bpy.types.Object:
    """Create a light object from exported light data."""
    light_name = light_data.get("name", "Light")
    light_type_str = light_data.get("light_type", "Omni")
    blender_light_type = light_type_to_blender(light_type_str)

    # Create light datablock
    light = bpy.data.lights.new(name=light_name, type=blender_light_type)

    # Set color (linear RGB)
    color = light_data.get("color", [1.0, 1.0, 1.0])
    if len(color) >= 3:
        light.color = color[:3]

    # Set intensity/energy
    intensity = light_data.get("intensity_candela_proxy")
    if intensity is None:
        intensity = light_data.get("intensity", 1000.0)
    
    # Convert to Blender watts (rough approximation)
    # This depends on light type; for now use a simple conversion
    if blender_light_type == "POINT":
        light.energy = max(0.1, intensity / 100.0)
    elif blender_light_type == "SUN":
        light.energy = max(0.1, intensity / 10000.0)
    elif blender_light_type == "SPOT":
        light.energy = max(0.1, intensity / 100.0)
    else:
        light.energy = 1.0

    # Set radius (size for area lights)
    radius = light_data.get("radius", 1.0)
    if blender_light_type in ("POINT", "SPOT"):
        light.shadow_soft_size = max(0.01, radius)
    
    # Set spot angles if projector/spotlight
    if blender_light_type == "SPOT":
        outer_angle = light_data.get("outer_angle")
        inner_angle = light_data.get("inner_angle")
        if outer_angle is not None:
            # CryEngine uses degrees; Blender spot_size is the full cone angle in radians
            light.spot_size = (outer_angle * 2) * (3.14159 / 180.0)
        if inner_angle is not None and outer_angle is not None:
            # spot_blend is blend factor between inner and outer cone
            blend = 1.0 - (inner_angle / outer_angle) if outer_angle > 0 else 0.5
            light.spot_blend = max(0.0, min(1.0, blend))

    # Create object for the light
    obj = bpy.data.objects.new(light_name, light)
    collection.objects.link(obj)

    # Set position from light data
    position = light_data.get("position", [0.0, 0.0, 0.0])
    obj.location = scene_position_to_blender(position)

    # Set rotation
    rotation = light_data.get("rotation", [1.0, 0.0, 0.0, 0.0])
    obj.rotation_quaternion = scene_quaternion_to_blender(rotation)

    # Parent to interior anchor if provided
    if interior_anchor is not None:
        obj.parent = interior_anchor

    # Store metadata
    obj["starbreaker_light_type"] = light_type_str
    obj["starbreaker_light_name"] = light_name

    return obj


def import_child_attachment(
    child_data: dict[str, Any],
    parent_obj: bpy.types.Object,
    collection: bpy.types.Collection,
) -> bpy.types.Object:
    """Import a single child attachment."""
    entity_name = child_data.get("entity_name", "Child")

    # Create anchor for the child
    anchor = bpy.data.objects.new(entity_name, None)
    anchor.empty_display_type = "PLAIN_AXES"
    collection.objects.link(anchor)

    # Set parent relationship
    anchor.parent = parent_obj

    # Apply transform using local_transform_sc if available
    if "local_transform_sc" in child_data:
        anchor.matrix_local = scene_matrix_to_blender(child_data["local_transform_sc"])
    else:
        # Fallback: use offset_position and offset_rotation
        pos = child_data.get("offset_position", [0.0, 0.0, 0.0])
        rot = child_data.get("offset_rotation", [0.0, 0.0, 0.0])

        anchor.location = scene_position_to_blender(pos)

        # For offset_rotation in Euler angles, convert to quaternion
        if rot and all(abs(r) < 0.0001 for r in rot):
            # No rotation
            anchor.rotation_quaternion = Quaternion((1.0, 0.0, 0.0, 0.0))
        else:
            # Simple Euler to Quat conversion (assuming XYZ order)
            from math import radians
            anchor.rotation_euler = (radians(rot[0]), radians(rot[1]), radians(rot[2]))

    # Store metadata
    anchor["starbreaker_entity_name"] = entity_name
    if "geometry_path" in child_data:
        anchor["geometry_path"] = child_data["geometry_path"]
    if "mesh_asset" in child_data:
        anchor["mesh_asset"] = child_data["mesh_asset"]
    if "material_sidecar" in child_data:
        anchor["material_sidecar"] = child_data["material_sidecar"]

    return anchor


def import_interior(
    interior_data: dict[str, Any],
    package_root: bpy.types.Object,
    root_collection: bpy.types.Collection,
) -> bpy.types.Object:
    """Import an interior container."""
    interior_name = interior_data.get("name", "Interior")

    # Create interior collection
    interior_col = ensure_collection(root_collection, f"interiors_{interior_name}")

    # Create anchor for the interior
    anchor_name = (
        interior_name
        if interior_name.startswith("interior_")
        else f"interior_{interior_name}"
    )
    anchor = bpy.data.objects.new(anchor_name, None)
    anchor.empty_display_type = "CUBE"
    anchor.parent = package_root
    interior_col.objects.link(anchor)

    # Apply container transform
    if "container_transform" in interior_data:
        anchor.matrix_local = scene_matrix_to_blender(interior_data["container_transform"])

    # Store metadata
    anchor["starbreaker_interior_name"] = interior_name

    # Import lights
    if "lights" in interior_data:
        for light_data in interior_data["lights"]:
            create_light_object(light_data, anchor, interior_col)

    # Import placements (interior instances)
    if "placements" in interior_data:
        for placement_data in interior_data["placements"]:
            placement_name = placement_data.get("entity_name", "Placement")
            placement_anchor = bpy.data.objects.new(placement_name, None)
            placement_anchor.parent = anchor
            interior_col.objects.link(placement_anchor)

            # Apply placement transform
            if "transform" in placement_data:
                placement_anchor.matrix_local = scene_matrix_to_blender(
                    placement_data["transform"]
                )

            placement_anchor["starbreaker_placement_entity"] = placement_name

    return anchor


def generate_scene_from_json(scene_json_path: str, output_blend_path: str) -> None:
    """
    Main function to generate Blender scene from scene.json.

    Args:
        scene_json_path: Path to the scene.json file
        output_blend_path: Path where to save the generated scene.blend
    """
    scene_json_path = Path(scene_json_path).resolve()
    output_blend_path = Path(output_blend_path).resolve()

    if not scene_json_path.exists():
        raise FileNotFoundError(f"scene.json not found: {scene_json_path}")

    # Load scene.json
    with open(scene_json_path, "r") as f:
        scene_data = json.load(f)

    # Get or create default scene and world
    scene = bpy.context.scene
    scene.name = "Scene"

    # Create root collection
    root_col = create_root_collection(scene)

    # Create children collection
    children_col = ensure_collection(root_col, "children")

    # Create root entity object (package root anchor)
    root_entity_data = scene_data.get("root_entity", {})
    package_name = root_entity_data.get("name", "Package")
    
    package_root = bpy.data.objects.new(f"StarBreaker {package_name}", None)
    package_root.empty_display_type = "CUBE"
    root_col.objects.link(package_root)
    package_root["starbreaker_package_root"] = True
    package_root["starbreaker_entity_name"] = package_name

    # Import child attachments
    for child_data in scene_data.get("children", []):
        import_child_attachment(child_data, package_root, children_col)

    # Import interiors
    for interior_data in scene_data.get("interiors", []):
        import_interior(interior_data, package_root, root_col)

    # Ensure scene is set to starbreaker collection as active
    if root_col:
        scene.view_layers[0].active_layer_collection = scene.view_layers[0].layer_collection.children[
            root_col.name
        ]

    # Save the blend file
    output_blend_path.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(output_blend_path))

    print(f"✓ Scene saved to: {output_blend_path}")


def main():
    """Entry point for the Blender script."""
    try:
        scene_json_path, output_blend_path = parse_arguments()
        print(f"Generating scene from: {scene_json_path}")
        print(f"Output: {output_blend_path}")

        generate_scene_from_json(scene_json_path, output_blend_path)
        print("✓ Scene generation complete")

    except Exception as e:
        print(f"✗ Error: {e}", file=sys.stderr)
        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    main()
