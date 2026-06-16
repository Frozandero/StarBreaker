pub mod dequant;
pub(crate) mod decomposed;
pub mod error;
pub(crate) mod gltf;
pub(crate) mod included_objects;
pub mod ivo;
pub mod mtl;
pub mod nmc;
pub(crate) mod pipeline;
pub mod skeleton;
pub(crate) mod socpak;
pub mod types;
pub mod animation;
pub mod chrparams;
pub mod validation;
pub mod ui_pipeline;

pub use error::Error;
pub use included_objects::{IncludedObject, IncludedObjects};
pub use pipeline::{
    DecomposedExport, ExportFormat, ExportKind, ExportOptions, ExportResult, ExportedFile,
    ExportedFileKind, MaterialMode,
    assemble_glb_with_loadout, assemble_glb_with_loadout_with_progress,
    dump_hierarchy, load_invisible_ports, query_animation_controller_source,
    resolve_loadout_meshes, socpaks_to_glb,
};
pub use types::Mesh;
pub use validation::{ValidationReport, validate_decomposed_export};

use starbreaker_chunks::ChunkFile;

/// Decode a multi-part `.cga`/`.cgf` and BAKE its NMC scene-graph node
/// transforms into a single world-space [`Mesh`], the same assembly the glTF
/// builder applies (so the result matches what the app's `skin_to_glb` viewer
/// shows: wings, engine pods and sub-objects positioned, not detached at the
/// origin). `mesh_data` is the vertex companion (`.cgam`/`.cgfm`/`.skinm`);
/// `nmc_data` is the primary file (`.cga`/…) carrying the NMC chunk. Falls back
/// to the raw [`parse_skin`] mesh when the file has no usable NMC hierarchy.
/// Result is in CryEngine (Z-up) model space.
pub fn parse_skin_positioned(mesh_data: &[u8], nmc_data: &[u8]) -> Result<Mesh, Error> {
    let mesh = parse_skin(mesh_data)?;
    match nmc::parse_nmc_full(nmc_data) {
        Some((nodes, _mat)) if nodes.len() > 1 => Ok(flatten_nmc_to_world(&mesh, &nodes)),
        _ => Ok(mesh),
    }
}

/// Multiply two 3×4 row-major affine matrices (each implicitly `[0,0,0,1]`
/// bottom row): `a * b`.
fn mat3x4_mul(a: &[[f32; 4]; 3], b: &[[f32; 4]; 3]) -> [[f32; 4]; 3] {
    let mut r = [[0.0f32; 4]; 3];
    for i in 0..3 {
        for j in 0..3 {
            r[i][j] = a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j];
        }
        r[i][3] = a[i][0] * b[0][3] + a[i][1] * b[1][3] + a[i][2] * b[2][3] + a[i][3];
    }
    r
}

const NMC_IDENTITY: [[f32; 4]; 3] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
];

fn nmc_matrix_is_identity_or_zero(m: &[[f32; 4]; 3]) -> bool {
    const EPS: f32 = 1e-5;
    if m.iter().all(|row| row.iter().all(|&v| v.abs() < EPS)) {
        return true;
    }
    m.iter()
        .zip(NMC_IDENTITY.iter())
        .all(|(r, ir)| r.iter().zip(ir.iter()).all(|(&a, &b)| (a - b).abs() < EPS))
}

/// Bake NMC node transforms into world-space vertices. Mirrors the glTF
/// builder: each node's `bone_to_world` is its LOCAL matrix composed down the
/// parent hierarchy; a submesh's triangles (its INDEX range) reference shared
/// per-node-local positions, transformed by that node's composed world matrix.
fn flatten_nmc_to_world(mesh: &Mesh, nodes: &[nmc::NmcNode]) -> Mesh {
    // Composed world matrix per node (parents resolved first via a fixpoint
    // pass — NMC parent indices are not guaranteed to precede children).
    let local: Vec<[[f32; 4]; 3]> = nodes
        .iter()
        .map(|n| {
            if nmc_matrix_is_identity_or_zero(&n.bone_to_world) {
                NMC_IDENTITY
            } else {
                n.bone_to_world
            }
        })
        .collect();
    let mut world: Vec<Option<[[f32; 4]; 3]>> = vec![None; nodes.len()];
    for _ in 0..nodes.len() {
        let mut progressed = false;
        for (i, node) in nodes.iter().enumerate() {
            if world[i].is_some() {
                continue;
            }
            match node.parent_index {
                Some(p) if (p as usize) < nodes.len() => {
                    if let Some(pw) = world[p as usize] {
                        world[i] = Some(mat3x4_mul(&pw, &local[i]));
                        progressed = true;
                    }
                }
                _ => {
                    world[i] = Some(local[i]);
                    progressed = true;
                }
            }
        }
        if !progressed {
            break;
        }
    }

    let apply = |m: &[[f32; 4]; 3], p: [f32; 3]| -> [f32; 3] {
        [
            m[0][0] * p[0] + m[0][1] * p[1] + m[0][2] * p[2] + m[0][3],
            m[1][0] * p[0] + m[1][1] * p[1] + m[1][2] * p[2] + m[1][3],
            m[2][0] * p[0] + m[2][1] * p[1] + m[2][2] * p[2] + m[2][3],
        ]
    };

    let mut out_pos: Vec<[f32; 3]> = Vec::with_capacity(mesh.indices.len());
    let mut out_idx: Vec<u32> = Vec::with_capacity(mesh.indices.len());
    for sub in &mesh.submeshes {
        let w = world
            .get(sub.node_parent_index as usize)
            .and_then(|w| *w)
            .unwrap_or(NMC_IDENTITY);
        let start = sub.first_index as usize;
        let end = (start + sub.num_indices as usize).min(mesh.indices.len());
        for &vi in &mesh.indices[start..end] {
            let Some(&p) = mesh.positions.get(vi as usize) else {
                continue;
            };
            out_idx.push(out_pos.len() as u32);
            out_pos.push(apply(&w, p));
        }
    }
    if out_pos.is_empty() {
        return mesh.clone();
    }
    let mut flat = mesh.clone();
    flat.positions = out_pos;
    flat.indices = out_idx;
    flat.uvs = None;
    flat.normals = None;
    flat.tangents = None;
    flat.colors = None;
    flat.secondary_uvs = None;
    flat.submeshes = Vec::new();
    flat
}

/// Parse a `.skin`/`.cgf` IVO file into a Mesh domain type.
/// Returns an error if the file uses CrCh format (not supported).
pub fn parse_skin(data: &[u8]) -> Result<Mesh, Error> {
    parse_skin_with_options(data, false)
}

/// Parse a `.skin`/`.cgf` IVO file, optionally dequantizing with model bbox.
/// Interior CGFs use `use_model_bbox = true` because IncludedObjects placements
/// are authored for model-bbox space.
pub(crate) fn parse_skin_with_options(data: &[u8], use_model_bbox: bool) -> Result<Mesh, Error> {
    let chunk_file = ChunkFile::from_bytes(data)?;
    let ivo = match &chunk_file {
        ChunkFile::Ivo(ivo) => ivo,
        ChunkFile::CrCh(_) => return Err(Error::UnsupportedFormat),
    };

    // Find and parse IvoSkin2 chunk (0xB8757777)
    let skin_entry = ivo
        .chunks()
        .iter()
        .find(|c| c.chunk_type == starbreaker_chunks::known_types::ivo::IVO_SKIN2)
        .ok_or(Error::MissingChunk {
            chunk_type: starbreaker_chunks::known_types::ivo::IVO_SKIN2,
        })?;
    let skin_mesh = ivo::skin::SkinMesh::read(ivo.chunk_data(skin_entry))?;

    // Find and parse MtlName chunks (0x83353333)
    let materials: Vec<ivo::material::MaterialName> = ivo
        .chunks()
        .iter()
        .filter(|c| c.chunk_type == starbreaker_chunks::known_types::ivo::MTL_NAME_IVO320)
        .map(|entry| ivo::material::MaterialName::read(ivo.chunk_data(entry)))
        .collect::<Result<_, _>>()?;

    Ok(types::build_mesh_with_bbox(&skin_mesh, &materials, use_model_bbox))
}

/// Parse a `.skin`/`.cgf` IVO file and convert to GLB in one step.
/// If `metadata` is provided (the primary `.cgf`/`.skin` bytes), NMC transforms
/// are parsed from it and applied to the scene graph.
pub fn skin_to_glb(data: &[u8], metadata: Option<&[u8]>) -> Result<Vec<u8>, Error> {
    let mesh = parse_skin(data)?;
    let root_nmc = metadata
        .and_then(nmc::parse_nmc_full)
        .map(|(nodes, mat_indices)| nmc::NodeMeshCombo {
            nodes,
            material_indices: mat_indices,
        });
    gltf::write_glb(
        gltf::GlbInput {
            root_mesh: Some(mesh),
            root_materials: None,
            root_textures: None,
            root_nmc,
            root_palette: None,
            skeleton_bones: Vec::new(),
            children: Vec::new(),
            interiors: pipeline::LoadedInteriors::default(),
        },
        &mut gltf::GlbLoaders {
            load_textures: &mut |_, _| None,
            load_interior_mesh: &mut |_| None,
        },
        &gltf::GlbOptions {
            material_mode: pipeline::MaterialMode::None,
            preserve_textureless_decal_primitives: false,
            metadata: gltf::GlbMetadata {
                entity_name: None,
                geometry_path: None,
                material_path: None,
                export_options: gltf::ExportOptionsMetadata {
                    kind: "Bundled".to_string(),
                    material_mode: "None".to_string(),
                    format: "Glb".to_string(),
                    lod_level: 0,
                    texture_mip: 0,
                    include_attachments: false,
                    include_interior: false,
                },
            },
            fallback_palette: None,
        },
    )
}

#[cfg(test)]
mod nmc_flatten_tests {
    use super::*;
    use crate::nmc::NmcNode;
    use crate::types::{Mesh, SubMesh};

    fn node(name: &str, parent: Option<u16>, btw: [[f32; 4]; 3]) -> NmcNode {
        NmcNode {
            name: name.into(),
            parent_index: parent,
            world_to_bone: NMC_IDENTITY,
            bone_to_world: btw,
            scale: [1.0, 1.0, 1.0],
            geometry_type: 0,
            properties: std::collections::HashMap::new(),
        }
    }

    fn submesh(node_idx: u16, first_index: u32, num_indices: u32) -> SubMesh {
        SubMesh {
            material_name: None,
            material_id: 0,
            source_material_id: None,
            first_index,
            num_indices,
            first_vertex: 0,
            num_vertices: 0,
            node_parent_index: node_idx,
        }
    }

    /// A submesh attached to a node with a translation is baked into world
    /// space; a child node composes its parent's transform. Mirrors the wing /
    /// engine-pod placement that raw `parse_skin` leaves at the origin.
    #[test]
    fn flatten_bakes_node_and_parent_transforms() {
        // Node 0: root identity. Node 1: root, +10 in x. Node 2: child of 1, +1 in y.
        let nodes = vec![
            node("root", None, NMC_IDENTITY),
            node("wing", None, [[1.0, 0.0, 0.0, 10.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0]]),
            node("tip", Some(1), [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 1.0], [0.0, 0.0, 1.0, 0.0]]),
        ];
        // One vertex at origin per node, indexed once each.
        let mut mesh = Mesh {
            positions: vec![[0.0, 0.0, 0.0]],
            indices: vec![0, 0, 0],
            uvs: None,
            secondary_uvs: None,
            normals: None,
            tangents: None,
            colors: None,
            submeshes: vec![submesh(0, 0, 1), submesh(1, 1, 1), submesh(2, 2, 1)],
            model_min: [0.0; 3],
            model_max: [0.0; 3],
            scaling_min: [0.0; 3],
            scaling_max: [0.0; 3],
        };
        // Make each index reference the single shared origin vertex.
        mesh.indices = vec![0, 0, 0];
        let flat = flatten_nmc_to_world(&mesh, &nodes);
        assert_eq!(flat.positions.len(), 3, "one emitted vertex per index");
        // sub0 (root): origin unchanged.
        assert_eq!(flat.positions[0], [0.0, 0.0, 0.0]);
        // sub1 (wing): +10 x.
        assert_eq!(flat.positions[1], [10.0, 0.0, 0.0]);
        // sub2 (tip, child of wing): parent +10x composed with local +1y.
        assert_eq!(flat.positions[2], [10.0, 1.0, 0.0]);
    }
}
