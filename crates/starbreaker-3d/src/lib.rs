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

/// Append 4 shield-face panes (front/rear/port/starboard) around `ship`,
/// returning the merged mesh **and the index-array offset at which the shield
/// triangles begin** (so the renderer can draw them at a reduced alpha).
///
/// Mirrors the engine's SELF-STATUS shield render (`UIHoloVehicle_Config`:
/// `shieldProxyModel` = the `shield_pane.cgf` curved unit pane, placed around
/// the hull). The decoded `shield_pane` is FLAT in its local X-Y plane (width
/// X, height Y) with a shallow dome bulging along +Z (its normal). Each of the
/// 4 faces stands the pane up (width → horizontal tangent, height → world up,
/// dome → outward), then rotates it about the vertical axis by 0/90/180/270°
/// so the faces are identical and the rear is the front rotated 180°.
///
/// Layout (X = hull length, Y = width, Z = height):
/// * The 4 faces form a SQUARE box of half-side `off = half_larger + margin`,
///   where `half_larger` is the larger horizontal half-extent and
///   `margin = shield_distance × (full larger dimension)` (e.g. `0.15` = a 15%
///   gap between the hull's outer bound and the shield — no intersection).
/// * Each pane spans only `SPAN_FRAC` of the square side, centred, so adjacent
///   faces leave a gap at the corners (4 distinct faces, not a closed box).
/// * Wall height = `HEIGHT_MULT × hull height`, centred on the hull centre line
///   (50% above / 50% below).
/// The engine derives exact faces from the ship's shield component; this is the
/// generic UI-proxy approximation.
pub fn with_shield_panes(mut ship: Mesh, pane: &Mesh, shield_distance: f32) -> (Mesh, usize) {
    let ship_index_count = ship.indices.len();
    if pane.positions.is_empty() || ship.positions.is_empty() {
        return (ship, ship_index_count);
    }
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for p in &ship.positions {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    let c = [(lo[0] + hi[0]) * 0.5, (lo[1] + hi[1]) * 0.5, (lo[2] + hi[2]) * 0.5];
    let half = [(hi[0] - lo[0]) * 0.5, (hi[1] - lo[1]) * 0.5, (hi[2] - lo[2]) * 0.5];

    // Pane local extents: X = width, Y = height (both centred on 0), Z = the
    // shallow dome depth (its normal; NOT centred — the rim sits at z_min).
    let (mut plo, mut phi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for p in &pane.positions {
        for k in 0..3 {
            plo[k] = plo[k].min(p[k]);
            phi[k] = phi[k].max(p[k]);
        }
    }
    let pane_w = (phi[0] - plo[0]).max(1e-4); // local x extent (width)
    let pane_h = (phi[1] - plo[1]).max(1e-4); // local y extent (height)

    // Square box half-side and 15%-style margin off the larger horizontal dim.
    let half_larger = half[0].max(half[1]);
    let margin = shield_distance.max(0.0) * 2.0 * half_larger;
    let off = half_larger + margin; // square half-side (pane rim offset)
    const SPAN_FRAC: f32 = 0.8; // face spans 80% of the side → corner gaps
    const HEIGHT_MULT: f32 = 2.0; // wall = 2× hull height, centred

    let full_span = SPAN_FRAC * 2.0 * off; // pane width in world units
    let wall_height = HEIGHT_MULT * 2.0 * half[2];
    let scale_w = full_span / pane_w; // local x (width) → world tangent span
    let scale_h = wall_height / pane_h; // local y (height) → world up
    // Keep the dome's native curvature proportion relative to width (so the
    // spherical cap isn't flattened or exaggerated — the warp came from scaling
    // the tiny depth axis to fill the height).
    let scale_d = scale_w;

    // Combined rotation R(θ) = Rz(θ)·R0, where R0 stands the pane up (local
    // width→world +Y, height→world +Z, dome/normal→world +X) and Rz(θ) spins it
    // about the vertical axis. Outward normal n(θ) = (cosθ, sinθ, 0).
    for quarter in 0..4 {
        let theta = std::f32::consts::FRAC_PI_2 * quarter as f32;
        let (s, co) = theta.sin_cos();
        // R rows (see doc): [[-s,0,co],[co,0,s],[0,1,0]].
        let r = [[-s, 0.0, co], [co, 0.0, s], [0.0, 1.0, 0.0]];
        let centre = [c[0] + off * co, c[1] + off * s, c[2]];
        let base = ship.positions.len() as u32;
        for p in &pane.positions {
            // Scale in pane-local space; shift the dome so its rim (z_min) sits
            // at the box face and the cap bulges outward (+local z).
            let sx = (p[0] - 0.5 * (plo[0] + phi[0])) * scale_w; // centre width
            let sy = (p[1] - 0.5 * (plo[1] + phi[1])) * scale_h; // centre height
            let sz = (p[2] - plo[2]) * scale_d; // rim at 0, dome outward
            ship.positions.push([
                r[0][0] * sx + r[0][1] * sy + r[0][2] * sz + centre[0],
                r[1][0] * sx + r[1][1] * sy + r[1][2] * sz + centre[1],
                r[2][0] * sx + r[2][1] * sy + r[2][2] * sz + centre[2],
            ]);
        }
        for &i in &pane.indices {
            ship.indices.push(base + i);
        }
    }
    ship.submeshes = Vec::new();
    (ship, ship_index_count)
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

    fn bare_mesh(positions: Vec<[f32; 3]>, indices: Vec<u32>) -> Mesh {
        Mesh {
            positions,
            indices,
            uvs: None,
            secondary_uvs: None,
            normals: None,
            tangents: None,
            colors: None,
            submeshes: vec![],
            model_min: [0.0; 3],
            model_max: [0.0; 3],
            scaling_min: [0.0; 3],
            scaling_max: [0.0; 3],
        }
    }

    /// The 4 shield panes form a SQUARE box clear of the hull (15% margin off
    /// the larger horizontal dimension), 2× the hull height centred on it, with
    /// a corner gap, and the returned boundary marks the first shield triangle.
    /// Guards the pane-axis fix (the pane is flat in X-Y, dome along +Z — a
    /// previous version mis-took it as flat in Y-Z and warped the faces).
    #[test]
    fn shield_panes_form_square_box_clear_of_hull() {
        // Hull bbox ±(10, 5, 2): half = (10, 5, 2), centred at the origin.
        let ship = bare_mesh(vec![[-10.0, -5.0, -2.0], [10.0, 5.0, 2.0], [0.0, 0.0, 0.0]], vec![0, 1, 2]);
        // Flat unit pane in X-Y (width 1, height 1) with a +Z dome (rim at z=0).
        let pane = bare_mesh(
            vec![[-0.5, -0.5, 0.0], [0.5, -0.5, 0.0], [0.5, 0.5, 0.1], [-0.5, 0.5, 0.1]],
            vec![0, 1, 2, 0, 2, 3],
        );
        let (merged, boundary) = with_shield_panes(ship, &pane, 0.15);

        // Shield triangles begin right after the hull indices; 4 faces appended.
        assert_eq!(boundary, 3, "boundary = original hull index count");
        assert_eq!(merged.positions.len(), 3 + 4 * 4, "4 faces × 4 pane verts");
        assert_eq!(merged.indices.len(), 3 + 4 * 6, "4 faces × 6 pane indices");

        // half_larger = 10, margin = 0.15 × 2 × 10 = 3 → square half-side off = 13.
        let off = 13.0_f32;
        let shield = &merged.positions[3..];
        // No intersection: every shield vert sits at/beyond the box rim
        // (max(|x|,|y|) ≥ off), clearing the hull (horizontal extent ±10).
        for p in shield {
            let reach = p[0].abs().max(p[1].abs());
            assert!(reach >= off - 1e-3, "shield vert {p:?} intrudes inside the box (reach {reach})");
        }
        // Corner gap: the first face's (+X) tangential (Y) span stays inside the
        // square half-side, so adjacent faces don't meet at the corners.
        let max_tangent = merged.positions[3..7].iter().map(|p| p[1].abs()).fold(0.0, f32::max);
        assert!(max_tangent < off, "corner gap: +X face Y-span {max_tangent} < off {off}");
        // Wall height = 2× hull height (=8) centred on z=0 → half-height ~4.
        let max_z = shield.iter().map(|p| p[2].abs()).fold(0.0, f32::max);
        assert!((max_z - 4.0).abs() < 1e-2, "wall half-height ~4, got {max_z}");
    }
}
