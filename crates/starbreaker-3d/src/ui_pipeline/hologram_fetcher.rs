//! P4K/DataCore-backed [`HologramFetcher`] for the SELF-STATUS vehicle
//! hologram.
//!
//! The engine draws `WidgetRuntimeImage`/`Primitive` "Own Vehicle Hologram"
//! nodes as a live 3D render of the player's ship; the 2D UI compositor cannot.
//! This fetcher resolves the render scene's ROOT vehicle hull geometry from
//! DataCore (`SGeometryResourceParams`), decodes it ([`crate::parse_skin`]) and
//! rasterises a neutral greyscale shaded-filled-faces hologram (from behind, in
//! perspective) via [`starbreaker_gfx::render_vehicle_hologram`], tinted by the
//! node's authored background fill (the per-manufacturer holo colour). It is
//! generic over the loaded ship (root entity), with no ship/screen name
//! branches.

use starbreaker_datacore::loadout::EntityIndex;
use starbreaker_datacore::Database;
use starbreaker_gfx::{render_vehicle_hologram, HologramParams};
use starbreaker_p4k::MappedP4k;
use starbreaker_ui::pipeline::{HologramFetcher, HologramImage};

use crate::pipeline::datacore_path_to_p4k;

/// Resolves and rasterises the loaded ship's hologram on demand.
pub(super) struct P4kHologramFetcher<'a> {
    pub(super) p4k: &'a MappedP4k,
    pub(super) db: &'a Database<'a>,
    /// Render scene's root entity name (the player's vehicle). `None` for
    /// non-vehicle exports — the fetcher then yields nothing.
    pub(super) root_entity_name: Option<&'a str>,
}

impl P4kHologramFetcher<'_> {
    /// Resolve the root vehicle's hull geometry P4K path
    /// (`SGeometryResourceParams.Geometry.Geometry.Geometry.path`).
    fn hull_geometry_p4k_path(&self) -> Option<String> {
        let name = self.root_entity_name?;
        let idx = EntityIndex::new(self.db);
        let stem = name.rsplit('.').next().unwrap_or(name);
        let record = idx.find_record(stem)?;
        let compiled = self
            .db
            .compile_path::<String>(
                record.struct_id(),
                "Components[SGeometryResourceParams].Geometry.Geometry.Geometry.path",
            )
            .ok()?;
        let geom_path = self.db.query_single::<String>(&compiled, record).ok()??;
        Some(datacore_path_to_p4k(&geom_path))
    }

    /// Resolve the generic shield-face proxy mesh + its placement distance from
    /// the global `UIHoloVehicle_Config` record: `shieldProxyModel` (a curved
    /// unit pane shared by every ship's SELF-STATUS hologram) and
    /// `shieldDistance` (the fraction of the shield box the hull fills). Returns
    /// `None` if the config record or its asset is unavailable. Fully
    /// data-driven — both the asset path and the distance come from DataCore,
    /// not hard-coded values.
    fn shield_proxy(&self) -> Option<(crate::Mesh, f32)> {
        let record = self.db.records_by_type_name("UIHoloVehicle_Config").next()?;
        let model_path = self
            .db
            .compile_path::<String>(record.struct_id(), "shieldProxyModel")
            .ok()?;
        let proxy = self.db.query_single::<String>(&model_path, record).ok()??;
        let dist_path = self
            .db
            .compile_path::<f32>(record.struct_id(), "shieldDistance")
            .ok()?;
        let shield_distance = self.db.query_single::<f32>(&dist_path, record).ok()??;
        let p4k_path = datacore_path_to_p4k(&proxy);
        let companion = format!("{p4k_path}m");
        let mesh = match (self.p4k.read_file(&companion), self.p4k.read_file(&p4k_path)) {
            (Ok(verts), Ok(primary)) => crate::parse_skin_positioned(&verts, &primary).ok()?,
            (Ok(verts), Err(_)) => crate::parse_skin(&verts).ok()?,
            (Err(_), Ok(primary)) => crate::parse_skin(&primary).ok()?,
            (Err(_), Err(_)) => return None,
        };
        Some((mesh, shield_distance))
    }
}

impl HologramFetcher for P4kHologramFetcher<'_> {
    fn fetch_vehicle_hologram(&self, width: u32, height: u32, tint: [f32; 4]) -> Option<HologramImage> {
        if width == 0 || height == 0 {
            return None;
        }
        let p4k_path = self.hull_geometry_p4k_path()?;
        // Vertex data lives in the `m` companion (`.cgam`/`.cgfm`/`.skinm`);
        // the bare primary file holds the scene-graph/NMC. The full (non-LOD)
        // hull is used deliberately: the LOD variants don't decode cleanly yet.
        // `parse_skin_positioned` bakes the NMC node transforms so multi-part
        // geometry (wings, both engine pods, sub-objects) is assembled in world
        // space — raw `parse_skin` leaves parts detached / at the origin.
        let companion = format!("{p4k_path}m");
        let mut mesh = match (self.p4k.read_file(&companion), self.p4k.read_file(&p4k_path)) {
            (Ok(verts), Ok(primary)) => crate::parse_skin_positioned(&verts, &primary).ok()?,
            (Ok(verts), Err(_)) => crate::parse_skin(&verts).ok()?,
            (Err(_), Ok(primary)) => crate::parse_skin(&primary).ok()?,
            (Err(_), Err(_)) => return None,
        };
        if mesh.positions.is_empty() || mesh.indices.len() < 3 {
            return None;
        }
        // Wrap the hull in 4 shield faces forming a box PROPORTIONED to the
        // hull, placed via the data-driven `shieldDistance` (the hull fills that
        // fraction of the box → natural corner gaps). The proxy mesh + distance
        // both come from `UIHoloVehicle_Config`; only the fainter alpha is a
        // SELF-STATUS framing choice. `shield_index_start` marks where the
        // shield triangles begin so the renderer can fade them vs the hull.
        const SELF_STATUS_SHIELD_ALPHA_SCALE: f32 = 0.5;
        let shield_index_start = match self.shield_proxy() {
            Some((pane, shield_distance)) if !pane.positions.is_empty() => {
                let (merged, boundary) = crate::with_shield_panes(mesh, &pane, shield_distance);
                mesh = merged;
                Some(boundary)
            }
            _ => None,
        };
        // SELF-STATUS framing, calibrated to the in-game reference: yaw 0°
        // (nose pointing AWAY / up the frame), pitched -30° so the nose dips
        // DOWN, with perspective, filling roughly half the diagram area.
        // Semi-transparent faces (low alpha) so overlapping panels read as a
        // see-through hologram. The engine camera FOV/distance for the runtime
        // primitive isn't decoded, so these are a view choice matched to the
        // reference, not a layout fudge.
        const SELF_STATUS_YAW_DEG: f32 = 0.0;
        const SELF_STATUS_TILT_BACK_DEG: f32 = -30.0;
        // The whole hologram (hull + shield box) fills the WidgetRuntimeImage
        // rect (down to just above the footer), leaving a thin margin.
        const SELF_STATUS_FIT: f32 = 0.95;
        // Filled, shaded faces only (no wireframe) with a strong perspective so
        // the flat hull reads as an angled 3D hologram rather than a top-down
        // silhouette.
        const SELF_STATUS_FACE_ALPHA: f32 = 0.25;
        const SELF_STATUS_WIRE_ALPHA: f32 = 0.0;
        const SELF_STATUS_PERSPECTIVE: f32 = 1.5;
        let params = HologramParams {
            tint,
            yaw_deg: SELF_STATUS_YAW_DEG,
            tilt_back_deg: SELF_STATUS_TILT_BACK_DEG,
            fit: SELF_STATUS_FIT,
            face_alpha: SELF_STATUS_FACE_ALPHA,
            wire_alpha: SELF_STATUS_WIRE_ALPHA,
            perspective: SELF_STATUS_PERSPECTIVE,
            shield_index_start,
            shield_alpha_scale: SELF_STATUS_SHIELD_ALPHA_SCALE,
        };
        let img = render_vehicle_hologram(&mesh.positions, &mesh.indices, width, height, &params);
        Some(HologramImage {
            width,
            height,
            rgba: img.into_raw(),
        })
    }
}
