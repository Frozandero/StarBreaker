//! Socpak interior animation export: promotes animated interior entities to
//! decomposed `children` so their skeletal `.caf` clips are extracted.
//!
//! Hangar doors/elevators are `.cga` (animated geometry) with a sibling
//! `.chrparams` referencing `.caf` clips — the same mechanism ships use. The
//! socpak interior path emits them as static placements, so their animations are
//! never extracted. [`extract_animated_interior_children`] detects them, loads
//! them as [`EntityPayload`] children (carrying `skeleton_source_path`), and
//! removes the static duplicates so the existing decomposed animation pipeline
//! produces their clips.

use starbreaker_datacore::database::Database;
use starbreaker_datacore::types::{Record, StringId, StringId2};
use starbreaker_p4k::MappedP4k;

use crate::types::{EntityPayload, ResolvedNode};

use super::{
    datacore_path_to_p4k, load_child_payload_asset, ExportOptions, LoadedInteriors,
};

/// The sibling `.chrparams` path for an animated `.cga` geometry, or `None` for
/// static `.cgf` (or non-geometry) paths. Case of the stem is preserved.
pub(crate) fn chrparams_path_for_cga(cgf_path: &str) -> Option<String> {
    if cgf_path.to_ascii_lowercase().ends_with(".cga") {
        let stem = &cgf_path[..cgf_path.len() - ".cga".len()];
        Some(format!("{stem}.chrparams"))
    } else {
        None
    }
}

/// Synthetic placeholder DataCore record for an interior geometry that has no
/// entity record (socpak `IncludedObjects` are raw geometry placements).
/// `load_child_payload_asset` only falls back to the record when `geometry_path`
/// fails to load, so an empty record is sufficient.
fn placeholder_record() -> Record {
    Record {
        name_offset: StringId2(-1),
        file_name_offset: StringId(0),
        tag_offset: StringId2(-1),
        struct_index: 0,
        id: starbreaker_common::CigGuid::EMPTY,
        instance_index: 0,
        struct_size: 0,
    }
}

fn cga_entity_name(cgf_path: &str) -> String {
    let base = cgf_path.rsplit(['/', '\\']).next().unwrap_or(cgf_path);
    base.get(..base.len().saturating_sub(".cga".len())).unwrap_or(base).to_string()
}

/// Detect animated `.cga` interior entities (those with a sibling `.chrparams`
/// in the P4k), load each as an `EntityPayload` child with its
/// `skeleton_source_path` set, and remove the static placements that became
/// children (so geometry isn't duplicated). Returns the animated children to add
/// to the decomposed `DecomposedInput.children`.
pub(crate) fn extract_animated_interior_children(
    db: &Database,
    p4k: &MappedP4k,
    interiors: &mut LoadedInteriors,
    opts: &ExportOptions,
) -> Vec<EntityPayload> {
    // Snapshot per-CGF info so the container loop can mutate placements while
    // still reading the shared unique-CGF table.
    let cgf_info: Vec<(String, Option<String>)> = interiors
        .unique_cgfs
        .iter()
        .map(|e| (e.cgf_path.clone(), e.material_path.clone()))
        .collect();

    let mut children = Vec::new();
    for container in &mut interiors.containers {
        let container_mat = glam::Mat4::from_cols_array_2d(&container.container_transform);
        let mut kept = Vec::with_capacity(container.placements.len());
        for placement in std::mem::take(&mut container.placements) {
            let Some((cgf_path, material_path)) = cgf_info.get(placement.mesh_index).cloned() else {
                kept.push(placement);
                continue;
            };
            let Some(chrparams) = chrparams_path_for_cga(&cgf_path) else {
                kept.push(placement);
                continue;
            };
            if p4k
                .entry_case_insensitive(&datacore_path_to_p4k(&chrparams))
                .is_none()
            {
                kept.push(placement);
                continue;
            }

            // Animated: place at the placement's world transform decomposed into
            // CryEngine-style offset position + Euler-degree rotation.
            let world = container_mat * glam::Mat4::from_cols_array_2d(&placement.transform);
            let (_scale, rot, trans) = world.to_scale_rotation_translation();
            let (rx, ry, rz) = rot.to_euler(glam::EulerRot::XYZ);
            let offset_position = [trans.x, trans.y, trans.z];
            let offset_rotation = [rx.to_degrees(), ry.to_degrees(), rz.to_degrees()];
            let entity_name = cga_entity_name(&cgf_path);

            let node = ResolvedNode {
                entity_name: entity_name.clone(),
                attachment_name: String::new(),
                no_rotation: false,
                offset_position,
                offset_rotation,
                detach_direction: [0.0; 3],
                port_flags: String::new(),
                nmc: None,
                bones: Vec::new(),
                has_geometry: true,
                record: placeholder_record(),
                geometry_path: Some(cgf_path.clone()),
                material_path: material_path.clone(),
                allows_child_object_containers: false,
                children: Vec::new(),
            };

            match load_child_payload_asset(&node, db, p4k, opts, opts.material_mode, None) {
                Some(loaded) => {
                    children.push(EntityPayload {
                        mesh: loaded.mesh,
                        materials: loaded.materials,
                        textures: loaded.textures,
                        nmc: loaded.nmc,
                        // Keep the placement's per-object tint palette if it had one.
                        palette: placement.palette.clone().or(loaded.palette),
                        geometry_path: loaded.geometry_path,
                        material_path: loaded.material_path,
                        bones: loaded.bones,
                        skeleton_source_path: loaded.skeleton_source_path,
                        entity_name,
                        entity_category: None,
                        attach_def_type: None,
                        parent_node_name: String::new(),
                        parent_entity_name: String::new(),
                        no_rotation: false,
                        offset_position,
                        offset_rotation,
                        detach_direction: [0.0; 3],
                        port_flags: String::new(),
                        ui_bindings: Vec::new(),
                    });
                }
                None => kept.push(placement),
            }
        }
        container.placements = kept;
    }
    children
}

#[cfg(test)]
mod tests {
    use super::{cga_entity_name, chrparams_path_for_cga};

    #[test]
    fn chrparams_path_for_cga_maps_only_cga_geometry() {
        assert_eq!(
            chrparams_path_for_cga(
                "objects/buildingsets/human/hightech/delta/int/hangar/elev/ht_hangar_ship_elev_xl_a.cga"
            )
            .as_deref(),
            Some("objects/buildingsets/human/hightech/delta/int/hangar/elev/ht_hangar_ship_elev_xl_a.chrparams")
        );
        // static .cgf geometry has no animation chrparams
        assert_eq!(chrparams_path_for_cga("objects/walls/hangar_wall_a.cgf"), None);
        // case-insensitive extension match, stem case preserved
        assert_eq!(
            chrparams_path_for_cga("Objects/Elev/Door_A.CGA").as_deref(),
            Some("Objects/Elev/Door_A.chrparams")
        );
        // not geometry
        assert_eq!(chrparams_path_for_cga("foo/bar.dds"), None);
    }

    #[test]
    fn cga_entity_name_strips_dir_and_extension() {
        assert_eq!(cga_entity_name("a/b/ht_hangar_ship_elev_xl_a.cga"), "ht_hangar_ship_elev_xl_a");
        assert_eq!(cga_entity_name("Door_A.CGA"), "Door_A");
    }
}
