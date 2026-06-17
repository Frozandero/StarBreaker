//! Tilt-projection compositor for the MFD-radar scope.
//!
//! The cockpit radar (`Screen_Radar_RTT` → `MapDisplayMaster` → radar mode →
//! `StarMapDisplayRTT` → `mapdisplaystarmap_window`) draws its scope through a
//! `BuildingBlocks_WidgetWindow` (`rendererType: "Primitive"`, material
//! `Materials/UI/Starmap/map_window.mtl`, `camera { fieldOfView: 20 }`): a
//! render-to-texture WINDOW that projects a 3D radar plane through a 3D camera.
//! The disc's artwork is NOT invented here — it is a real engine texture
//! (`UI/Textures/R_RadarMapScreen/3D_Object_Textures/r_radarmapscreen_radial_gradients.dds`,
//! bound by `ui_r_radarmapscreen_radial_grid.mtl` on the `Circle_Radial_Grid`
//! node): concentric rings, a perimeter degree-tick scale, the axis spoke and a
//! central pattern. The 2D-UI pipeline can't run the engine's RTT window, so —
//! as the SELF-STATUS hologram is composited by [`crate::mesh_holo`] — this
//! module takes that REAL decoded texture and projects it as the engine camera
//! would: the flat disc viewed from above and tilted back, so its circle
//! projects to an ELLIPSE.
//!
//! Only the camera TILT is an owner-tuned view constant: the engine's runtime
//! camera transform is pushed via `/MapNamespace/GeneralMapData/DisplayPosition`
//! + `DisplayOrientation`, which are absent at static rest (the same legitimate
//! boundary as the hologram's owner-tuned camera). Everything visible — the ring
//! pattern, tick scale, axis — comes from the real texture; the tint comes from
//! the brand palette (passed by the caller). The texture is treated as additive/
//! emissive: its luminance becomes the tinted disc's alpha, so the texture's
//! black background composites as transparent over the screen vignette.

use image::{Rgba, RgbaImage};

/// Radial gamma (< 1) that stretches the sweep wedge's outer-edge bright band
/// inward toward the disc centre (the in-game beam reaches the centre; the raw
/// texture hugs the edge). Lower = reaches further in. Owner-tuned (the sweep is
/// a live animation, so its radial extent isn't in the static data).
const SWEEP_RADIAL_GAMMA: f32 = 0.30;

/// Owner-tuned bloom: how far the `line_a` material's `Glow` spreads each spoke
/// bar, as a fraction of the disc major radius per unit Glow. The bars are only
/// ~1px wide geometrically (`sizing.width` 0.002), but the engine draws them as
/// emissive/bloomed glows, so a hairline bar reads as a soft radial. This is the
/// same render-pipeline-bloom boundary as [`RadarPlaneParams::intensity`] (not in
/// the static data); the actual `Glow` value it scales IS data
/// ([`RadarPlaneParams::spoke_glow`], from the material).
const SPOKE_GLOW_BLOOM_FRAC: f32 = 0.02;

/// Owner-tuned placement of the heading-tape ring on the tilted disc: the runtime
/// `radialTransform` curl that wraps the tape into a ring is absent at static rest
/// (`transformMultiplier=0`), so the ring radius/band are view constants matched to
/// the reference (same boundary as [`RadarPlaneParams::tilt_deg`]). The atlas
/// window + content are DATA ([`HeadingRingParams`]).
const HEADING_RING_RADIUS: f32 = 0.94;
/// Radial thickness of the heading-tape band, as a fraction of the disc radius.
const HEADING_RING_BAND: f32 = 0.08;

/// The outer heading-tape ring: the `coordinates_novalue` tick-marker atlas window
/// (DATA — the `HeadingTape` node's `UVStart`/`UVSize`) wrapped around the disc
/// perimeter. `UVSize.x` (>1) tiles the row around the full ring.
#[derive(Debug, Clone, Copy)]
pub struct HeadingRingParams {
    /// `primitiveSettings.UVStart` (atlas-space window origin).
    pub uv_start: [f32; 2],
    /// `primitiveSettings.UVSize` (atlas-space window extent; `x` = tiles around
    /// the ring, `y` = the tick-row band height).
    pub uv_size: [f32; 2],
    /// Overall alpha of the ring (the tape's authored fill × emissive).
    pub alpha: f32,
}

/// One authored radar spoke (`Circle_Line_*`) in the radar plane, projected onto
/// the tilted disc. Every field is AUTHORED DATA read from the spoke's IR node —
/// geometry from `anchor`/`sizing`/`orientation`, colour from its resolved
/// `background` fill (cardinals `Accent1`, diagonals `Base`; alpha `0.1`/`0.2`).
/// Nothing here is invented: the count (8), 45° spacing, per-spoke lengths and
/// colours all come from `rc_radarmapscreen_hostplane_visuals_large.json`.
#[derive(Debug, Clone, Copy)]
pub struct RadarSpoke {
    /// Bar centre in the parent plane's normalized `[0,1]×[0,1]` space (the node
    /// `anchor`); `(0.5,0.5)` is the disc centre.
    pub anchor: [f32; 2],
    /// Bar length as a fraction of the parent plane (`sizing.height` Percent:
    /// cardinals `0.4`, diagonals `0.3`).
    pub length_frac: f32,
    /// Bar width as a fraction of the parent plane (`sizing.width` Percent,
    /// `0.002`) — a hairline; the visible softness is the material glow.
    pub width_frac: f32,
    /// In-plane rotation in degrees (`orientation.z`: 0/45/90/135).
    pub rotation_deg: f32,
    /// Resolved RGB of the spoke's brand colour token (`Accent1`/`Base`).
    pub colour: [f32; 3],
    /// Authored fill alpha (`background.color.alpha`).
    pub alpha: f32,
}

/// Projection + style parameters for [`project_radar_disc`].
#[derive(Debug, Clone)]
pub struct RadarPlaneParams {
    /// Tilt of the disc back from top-down, in degrees: the disc circle's
    /// vertical axis is scaled by `sin(tilt)` (`90°` = top-down, circle stays a
    /// circle; smaller = more squashed). The reference reads ~`37°`
    /// (minor/major ≈ 0.6). Owner-tuned view constant — the engine's runtime
    /// camera transform is not in the static UI data (see module docs).
    pub tilt_deg: f32,
    /// Fraction of the smaller output dimension the disc's MAJOR (horizontal)
    /// axis fills, `0.0..=1.0`.
    pub fit: f32,
    /// Disc-centre vertical position as a fraction of output height
    /// (`0.5` = centre).
    pub centre_y_frac: f32,
    /// RGB multiply tint for the (greyscale/emissive) disc texture, channels
    /// `0.0..=1.0` — the caller passes the brand palette colour (DRAK = orange).
    pub tint_rgb: [f32; 3],
    /// Overall alpha multiplier applied to the projected disc, `0.0..=1.0`.
    pub alpha: f32,
    /// Emissive intensity multiplier on the texture luminance: the engine draws
    /// the disc as an emissive/bloomed surface, so the subtle source texture
    /// reads much brighter in-game than its raw values. Owner-tuned (the bloom
    /// intensity is a render-pipeline effect not in the static data).
    pub intensity: f32,
    /// Alpha of the bright outer boundary ring (the `Circle_Ripple_Textured`
    /// WidgetCircle that fills the disc plane: stroke `Accent1`). `0.0` = none.
    pub outer_ring_alpha: f32,
    /// Draw the own-ship centre marker (the white triangle the radar shows for
    /// the player vehicle).
    pub ship_marker: bool,
    /// Rotation (degrees) applied to the disc TEXTURE content — the disc
    /// material's `ViewingAngle` PublicParam (the radial shader's view angle:
    /// the DRAK radar authors `ViewingAngle=180`, so the texture's tick scale /
    /// "0" marker read rotated 180° vs an un-rotated sample). Data-backed +
    /// per-manufacturer (the material is brand-resolved). The disc OUTLINE and
    /// the own-ship marker are unaffected (only the texture lookup rotates).
    pub texture_rotation_deg: f32,
    /// Alpha of the rotating radar SWEEP wedge (`r_radarmapscreen_idle_animation`,
    /// the ping/scan beam), `0.0` = none. The sweep is a live animation; at static
    /// rest it's drawn once at `sweep_angle_deg`.
    pub sweep_alpha: f32,
    /// Rest-frame rotation (degrees) of the sweep wedge. The sweep rotates live
    /// in-game; this is the captured-frame position (owner-tuned, same basis as
    /// the camera tilt — the animation phase isn't in static data).
    pub sweep_angle_deg: f32,
    /// The authored radar spokes (`Circle_Line_*`), each projected in the tilted
    /// disc plane as a soft glowing bar. Empty = none. Geometry + colour are all
    /// data (see [`RadarSpoke`]).
    pub spokes: Vec<RadarSpoke>,
    /// The spoke material's gradient rim-end alpha (`line_a.mtl` `OuterAlpha`):
    /// the bar's alpha at its outer (rim) end. Data-backed (from the material).
    pub spoke_outer_alpha: f32,
    /// The spoke material's gradient centre-end alpha (`line_a.mtl` `InnerAlpha`,
    /// with `Gradient=1`): each bar fades from `spoke_outer_alpha` at its rim end
    /// to this at its disc-centre (inner) end. Data-backed (from the material).
    pub spoke_inner_alpha: f32,
    /// The spoke material's `Glow` (`line_a.mtl` `Glow`), scaled by
    /// [`SPOKE_GLOW_BLOOM_FRAC`] into a soft bloom half-width so a hairline bar
    /// reads as a glowing radial. The `Glow` value is data; the bloom scale is the
    /// owner-tuned render-bloom boundary (like [`Self::intensity`]).
    pub spoke_glow: f32,
    /// How far each spoke's INNER end reaches toward the disc centre, as a scale
    /// on its authored inner radius (`1.0` = authored; `<1` = closer to centre).
    /// The authored `Circle_Line` lengths stop short of centre at static rest
    /// (cardinals ~0.2, diagonals ~0.4 of the radius); the live radar plane
    /// extends them inward — owner-tuned to the reference (same runtime-camera
    /// boundary as [`Self::tilt_deg`] / the sweep gamma).
    pub spoke_inner_reach: f32,
    /// The outer heading-tape ring (atlas window), if loaded. The atlas texture is
    /// passed separately to [`project_radar_disc`]; this carries its UV window +
    /// alpha. `None` = no ring.
    pub heading_ring: Option<HeadingRingParams>,
    /// Where each spoke's OUTER end lands, as a scale on its authored outer radius
    /// (`1.0` = the disc/plane edge). The authored outer ends sit at the plane
    /// edge (radius ~1.0), OUTSIDE the bright ring (~0.8) that reads as the scope
    /// boundary — so they poke past the ring (and, at 45°, jut to the corners).
    /// Pull them to the ring (~0.8) to match the reference. Owner-tuned (runtime
    /// plane scale, same boundary as the inner reach).
    pub spoke_outer_reach: f32,
}

impl Default for RadarPlaneParams {
    fn default() -> Self {
        Self {
            tilt_deg: 37.0,
            fit: 0.92,
            centre_y_frac: 0.5,
            tint_rgb: [1.0, 0.62, 0.22],
            alpha: 1.0,
            intensity: 3.0,
            outer_ring_alpha: 0.7,
            ship_marker: true,
            texture_rotation_deg: 0.0,
            sweep_alpha: 0.0,
            sweep_angle_deg: 0.0,
            spokes: Vec::new(),
            spoke_outer_alpha: 1.0,
            spoke_inner_alpha: 0.5,
            spoke_glow: 0.0,
            spoke_inner_reach: 1.0,
            spoke_outer_reach: 1.0,
            heading_ring: None,
        }
    }
}

/// Project the real radar-disc `texture` (a flat top-down disc, e.g. the decoded
/// `r_radarmapscreen_radial_gradients.dds`) into a fresh `width × height` RGBA
/// image: scaled to an ellipse (major = `fit` × min-dim, minor = major ×
/// `sin(tilt)`), centred at `(width/2, height·centre_y_frac)`, tinted by
/// `tint_rgb`, with the texture's luminance as the emissive alpha. Transparent
/// background — the caller composites it over the screen vignette.
pub fn project_radar_disc(
    width: u32,
    height: u32,
    texture: &RgbaImage,
    sweep: Option<&RgbaImage>,
    heading: Option<&RgbaImage>,
    params: &RadarPlaneParams,
) -> RgbaImage {
    let mut out = RgbaImage::new(width, height);
    if width == 0 || height == 0 || texture.width() == 0 || texture.height() == 0 {
        return out;
    }

    let major = params.fit * (width.min(height) as f32) / 2.0;
    let squash = params.tilt_deg.to_radians().sin().clamp(0.02, 1.0);
    let minor = (major * squash).max(1.0);
    let cx = width as f32 / 2.0;
    let cy = height as f32 * params.centre_y_frac;

    let x0 = (cx - major).floor().max(0.0) as u32;
    let x1 = (cx + major).ceil().min(width as f32) as u32;
    let y0 = (cy - minor).floor().max(0.0) as u32;
    let y1 = (cy + minor).ceil().min(height as f32) as u32;

    let tw = texture.width() as f32;
    let th = texture.height() as f32;

    for y in y0..y1 {
        for x in x0..x1 {
            // Output pixel centre relative to the disc centre, in [-1, 1] disc
            // coords (undo the ellipse squash to sample the round source).
            let nx = (x as f32 + 0.5 - cx) / major;
            let ny = (y as f32 + 0.5 - cy) / minor;
            if nx * nx + ny * ny > 1.0 {
                continue; // outside the disc
            }
            // Rotate the texture lookup by the material's ViewingAngle (the disc
            // OUTLINE stays put; only the sampled content rotates).
            let (rx, ry) = if params.texture_rotation_deg != 0.0 {
                let a = params.texture_rotation_deg.to_radians();
                (nx * a.cos() - ny * a.sin(), nx * a.sin() + ny * a.cos())
            } else {
                (nx, ny)
            };
            // Map disc [-1,1] → texture [0,1] → texel.
            let u = (rx * 0.5 + 0.5) * (tw - 1.0);
            let v = (ry * 0.5 + 0.5) * (th - 1.0);
            let texel = sample_bilinear(texture, u, v);
            // Emissive: luminance → alpha; colour = tint.
            let lum = (0.299 * texel[0] as f32 + 0.587 * texel[1] as f32 + 0.114 * texel[2] as f32)
                / 255.0;
            let src_a = texel[3] as f32 / 255.0;
            let a = (lum * params.intensity).min(1.0) * src_a * params.alpha;
            if a <= 0.003 {
                continue;
            }
            out.put_pixel(
                x,
                y,
                Rgba([
                    (params.tint_rgb[0] * 255.0).round() as u8,
                    (params.tint_rgb[1] * 255.0).round() as u8,
                    (params.tint_rgb[2] * 255.0).round() as u8,
                    (a.clamp(0.0, 1.0) * 255.0).round() as u8,
                ]),
            );
        }
    }

    // Rotating SWEEP wedge (`r_radarmapscreen_idle_animation`): the ping/scan
    // beam, projected like the disc but rotated to its rest-frame angle and
    // blended faintly over the disc. The wedge texture is bright only in the
    // beam, transparent elsewhere, so only the beam shows.
    if let Some(sweep) = sweep {
        if params.sweep_alpha > 0.0 && sweep.width() > 0 && sweep.height() > 0 {
            let sw = sweep.width() as f32;
            let sh = sweep.height() as f32;
            let a = (params.texture_rotation_deg + params.sweep_angle_deg).to_radians();
            let (ca, sa) = (a.cos(), a.sin());
            for y in y0..y1 {
                for x in x0..x1 {
                    let nx = (x as f32 + 0.5 - cx) / major;
                    let ny = (y as f32 + 0.5 - cy) / minor;
                    if nx * nx + ny * ny > 1.0 {
                        continue;
                    }
                    let (rx, ry) = (nx * ca - ny * sa, nx * sa + ny * ca);
                    // Stretch the wedge radially INWARD: the texture's bright arc
                    // hugs the outer edge, but the in-game sweep beam reaches from
                    // the centre out, so map the outer texture band across more of
                    // the disc radius (radial gamma < 1).
                    let d = (rx * rx + ry * ry).sqrt();
                    let scale = if d > 1e-4 { d.powf(SWEEP_RADIAL_GAMMA) / d } else { 1.0 };
                    let (sx, sy) = (rx * scale, ry * scale);
                    // Flip horizontally so the sweep reads CLOCKWISE (the
                    // idle-animation texture is authored anti-clockwise).
                    let texel = sample_bilinear(sweep, (-sx * 0.5 + 0.5) * (sw - 1.0), (sy * 0.5 + 0.5) * (sh - 1.0));
                    let lum = (0.299 * texel[0] as f32 + 0.587 * texel[1] as f32 + 0.114 * texel[2] as f32) / 255.0;
                    let alpha = lum * (texel[3] as f32 / 255.0) * params.sweep_alpha;
                    if alpha > 0.003 {
                        blend(&mut out, x as i32, y as i32, [params.tint_rgb[0], params.tint_rgb[1], params.tint_rgb[2], alpha]);
                    }
                }
            }
        }
    }

    // Radial spokes (`Circle_Line_*`): the authored thin `line_a` bars. Each is
    // placed by its node geometry — centre at `anchor`, length `length_frac` of
    // the plane, rotated by `rotation_deg` about its own centre — projected into
    // the tilted disc plane (so they foreshorten with the ellipse), and drawn as
    // a SOFT glowing bar reproducing the material (a `Gradient` fade from full at
    // the rim end to `spoke_inner_alpha` at the disc-centre end, with a `Glow`
    // bloom half-width). Colour + alpha are per-spoke (Accent1/Base).
    if !params.spokes.is_empty() {
        // Map a plane point `[0,1]²` → screen px through the disc tilt: the plane
        // square inscribes the disc (`[-1,1]` ↦ `±major`/`±minor` about centre).
        let to_screen = |p: [f32; 2]| -> (f32, f32) {
            ((cx + (2.0 * p[0] - 1.0) * major), (cy + (2.0 * p[1] - 1.0) * minor))
        };
        // Glow bloom in px: the geometric half-width (data) plus the material
        // `Glow` spread (data × owner-tuned bloom scale). Floor so a hairline bar
        // still anti-aliases to ~1px.
        let glow_half = (params.spoke_glow * SPOKE_GLOW_BLOOM_FRAC * major).max(0.0);
        for spoke in &params.spokes {
            let theta = spoke.rotation_deg.to_radians();
            let (st, ct) = (theta.sin(), theta.cos());
            let h = spoke.length_frac * 0.5;
            // Endpoints of the bar centreline in plane space (long axis = the
            // node's vertical, rotated by orientation.z about the anchor).
            let e1 = [spoke.anchor[0] + h * st, spoke.anchor[1] - h * ct];
            let e2 = [spoke.anchor[0] - h * st, spoke.anchor[1] + h * ct];
            // The inner (disc-centre) end is whichever is nearer plane centre
            // (0.5,0.5); the gradient fades toward it.
            let d1 = (e1[0] - 0.5).hypot(e1[1] - 0.5);
            let d2 = (e2[0] - 0.5).hypot(e2[1] - 0.5);
            let (outer, inner) = if d1 >= d2 { (e1, e2) } else { (e2, e1) };
            // Rescale the spoke's radial span to the reference (the live radar
            // plane's runtime scale is absent at rest): pull the inner end toward
            // the centre and the outer end in to the bright ring. Both are scales
            // on the authored endpoint's offset from the plane centre.
            let inner = [
                0.5 + (inner[0] - 0.5) * params.spoke_inner_reach,
                0.5 + (inner[1] - 0.5) * params.spoke_inner_reach,
            ];
            let outer = [
                0.5 + (outer[0] - 0.5) * params.spoke_outer_reach,
                0.5 + (outer[1] - 0.5) * params.spoke_outer_reach,
            ];
            // Clamp endpoints to the disc: a 45° outer end lands at radius ~1.007
            // (the ellipse corner), poking past the rim — keep it on the disc so
            // spokes stay within the ring as in the reference.
            let clamp_disc = |p: [f32; 2]| -> [f32; 2] {
                let r = (2.0 * p[0] - 1.0).hypot(2.0 * p[1] - 1.0);
                if r > 1.0 {
                    [0.5 + (p[0] - 0.5) / r, 0.5 + (p[1] - 0.5) / r]
                } else {
                    p
                }
            };
            let (ox, oy) = to_screen(clamp_disc(outer)); // full alpha here (rim)
            let (ix, iy) = to_screen(clamp_disc(inner)); // spoke_inner_alpha (centre)
            let half_w = (spoke.width_frac * major).max(0.5) + glow_half;
            draw_soft_spoke(
                &mut out,
                (ox, oy),
                (ix, iy),
                half_w,
                spoke.colour,
                spoke.alpha * params.intensity,
                params.spoke_outer_alpha,
                params.spoke_inner_alpha,
            );
        }
    }

    // Outer heading-tape ring (`HeadingTape`): the `coordinates_novalue` tick-row
    // (the atlas window `uv_start`/`uv_size` — DATA) wrapped around the disc
    // perimeter. `uv_size.x` (>1) tiles the row around the full ring; `uv_size.y`
    // is the band height. Tilt-projected like the disc; the curl radius/band are
    // owner-tuned view constants (the runtime `radialTransform` is absent at rest).
    if let (Some(ring), Some(atlas)) = (params.heading_ring, heading)
        && ring.alpha > 0.0
        && atlas.width() > 0
        && atlas.height() > 0
    {
        let (aw, ah) = (atlas.width() as f32, atlas.height() as f32);
        let steps = ((major * 8.0) as u32).clamp(360, 4096);
        let band_steps = 6u32;
        let rot = params.texture_rotation_deg.to_radians();
        for i in 0..steps {
            let frac = i as f32 / steps as f32; // 0..1 around the ring
            let ang = frac * std::f32::consts::TAU + rot;
            let (sa, ca) = (ang.sin(), ang.cos());
            let u = ring.uv_start[0] + frac * ring.uv_size[0];
            let uu = u.rem_euclid(1.0) * (aw - 1.0);
            for b in 0..band_steps {
                let bt = b as f32 / (band_steps - 1) as f32; // 0..1 across the band
                let v = (ring.uv_start[1] + bt * ring.uv_size[1]).clamp(0.0, 1.0);
                let r = HEADING_RING_RADIUS + (bt - 0.5) * HEADING_RING_BAND;
                let texel = sample_bilinear(atlas, uu, v * (ah - 1.0));
                let lum = (0.299 * texel[0] as f32 + 0.587 * texel[1] as f32 + 0.114 * texel[2] as f32) / 255.0;
                let a = lum * (texel[3] as f32 / 255.0) * ring.alpha;
                if a <= 0.003 {
                    continue;
                }
                // disc-plane: angle 0 = North (up), clockwise.
                let px = cx + r * sa * major;
                let py = cy - r * ca * minor;
                blend(
                    &mut out,
                    px.round() as i32,
                    py.round() as i32,
                    [params.tint_rgb[0], params.tint_rgb[1], params.tint_rgb[2], a.min(1.0)],
                );
            }
        }
    }

    // Bright outer boundary ring: the `Circle_Ripple_Textured` WidgetCircle fills
    // the disc plane with an `Accent1` stroke — a crisp tinted ellipse at the
    // disc edge (the reference's bright outer ring). Drawn over the disc.
    if params.outer_ring_alpha > 0.0 {
        let c = [params.tint_rgb[0], params.tint_rgb[1], params.tint_rgb[2], params.outer_ring_alpha];
        draw_ellipse_stroke(&mut out, cx, cy, major, minor, c);
    }
    // Own-ship centre marker: the white triangle the radar shows for the player
    // vehicle (pointing up = the ship's facing at the neutral heading).
    if params.ship_marker {
        let s = (major * 0.045).max(3.0);
        fill_triangle(
            &mut out,
            (cx, cy - s * 1.2),
            (cx - s, cy + s * 0.8),
            (cx + s, cy + s * 0.8),
            [1.0, 1.0, 1.0, 1.0],
        );
    }
    out
}

/// Alpha-blend `c` (channels `0.0..=1.0`) onto pixel `(x, y)` (source-over).
fn blend(img: &mut RgbaImage, x: i32, y: i32, c: [f32; 4]) {
    if x < 0 || y < 0 || x as u32 >= img.width() || y as u32 >= img.height() || c[3] <= 0.0 {
        return;
    }
    let px = img.get_pixel_mut(x as u32, y as u32);
    let a = c[3];
    for i in 0..3 {
        px.0[i] = ((c[i] * 255.0) * a + px.0[i] as f32 * (1.0 - a)).round() as u8;
    }
    px.0[3] = ((c[3] * 255.0) + px.0[3] as f32 * (1.0 - a)).round().min(255.0) as u8;
}

/// Stroke an axis-aligned ellipse outline (segmented, blended).
fn draw_ellipse_stroke(img: &mut RgbaImage, cx: f32, cy: f32, rx: f32, ry: f32, c: [f32; 4]) {
    let segments = (rx.max(ry) * 6.0).clamp(96.0, 4096.0) as u32;
    let mut prev: Option<(f32, f32)> = None;
    for i in 0..=segments {
        let t = std::f32::consts::TAU * (i as f32 / segments as f32);
        let p = (cx + rx * t.cos(), cy + ry * t.sin());
        if let Some((px, py)) = prev {
            let steps = ((p.0 - px).abs().max((p.1 - py).abs())).ceil().max(1.0) as i32;
            for s in 0..=steps {
                let f = s as f32 / steps as f32;
                blend(img, (px + (p.0 - px) * f).round() as i32, (py + (p.1 - py) * f).round() as i32, c);
            }
        }
        prev = Some(p);
    }
}

/// Draw one soft, faded radar spoke bar from `outer` (rim) to `inner` (disc
/// centre), reproducing the `line_a` material: the `Gradient` alpha fade along
/// the bar (`outer_alpha` at the rim → `inner_alpha` at the centre, both authored
/// in the material) and a Gaussian `Glow` falloff across it (half-width `half_w`
/// px), so a hairline bar reads as a soft glow rather than a crisp line.
/// `base_alpha` is the authored fill alpha already scaled by the emissive boost.
fn draw_soft_spoke(
    img: &mut RgbaImage,
    outer: (f32, f32),
    inner: (f32, f32),
    half_w: f32,
    colour: [f32; 3],
    base_alpha: f32,
    outer_alpha: f32,
    inner_alpha: f32,
) {
    let (dx, dy) = (inner.0 - outer.0, inner.1 - outer.1);
    let len2 = dx * dx + dy * dy;
    if len2 <= 1e-6 || base_alpha <= 0.0 || half_w <= 0.0 {
        return;
    }
    // Bounding box of the bar expanded by the glow half-width (+ a small AA pad).
    let pad = half_w + 1.5;
    let min_x = (outer.0.min(inner.0) - pad).floor().max(0.0) as u32;
    let max_x = (outer.0.max(inner.0) + pad).ceil().min(img.width() as f32) as u32;
    let min_y = (outer.1.min(inner.1) - pad).floor().max(0.0) as u32;
    let max_y = (outer.1.max(inner.1) + pad).ceil().min(img.height() as f32) as u32;
    for y in min_y..max_y {
        for x in min_x..max_x {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            // Projection parameter t along outer→inner (0 = rim, 1 = centre).
            let t = (((px - outer.0) * dx + (py - outer.1) * dy) / len2).clamp(0.0, 1.0);
            let projx = outer.0 + dx * t;
            let projy = outer.1 + dy * t;
            let perp = (px - projx).hypot(py - projy);
            // Across-bar Gaussian glow (the `Glow` bloom).
            let across = (-(perp / half_w) * (perp / half_w)).exp();
            // Along-bar gradient: material OuterAlpha at the rim → InnerAlpha at
            // the centre.
            let along = outer_alpha + (inner_alpha - outer_alpha) * t;
            let a = base_alpha * across * along;
            if a > 0.003 {
                blend(img, x as i32, y as i32, [colour[0], colour[1], colour[2], a.min(1.0)]);
            }
        }
    }
}

/// Fill a triangle (scanline, blended) — the own-ship marker.
fn fill_triangle(img: &mut RgbaImage, a: (f32, f32), b: (f32, f32), d: (f32, f32), col: [f32; 4]) {
    let edge = |p: (f32, f32), q: (f32, f32), r: (f32, f32)| {
        (r.0 - p.0) * (q.1 - p.1) - (r.1 - p.1) * (q.0 - p.0)
    };
    if edge(a, b, d).abs() < 1e-3 {
        return;
    }
    let min_x = a.0.min(b.0).min(d.0).floor() as i32;
    let max_x = a.0.max(b.0).max(d.0).ceil() as i32;
    let min_y = a.1.min(b.1).min(d.1).floor() as i32;
    let max_y = a.1.max(b.1).max(d.1).ceil() as i32;
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let p = (x as f32 + 0.5, y as f32 + 0.5);
            let (w0, w1, w2) = (edge(b, d, p), edge(d, a, p), edge(a, b, p));
            if (w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0) || (w0 <= 0.0 && w1 <= 0.0 && w2 <= 0.0) {
                blend(img, x, y, col);
            }
        }
    }
}

/// Bilinear texture sample at floating `(u, v)` in pixel coords (clamped).
fn sample_bilinear(tex: &RgbaImage, u: f32, v: f32) -> [u8; 4] {
    let u = u.clamp(0.0, tex.width() as f32 - 1.001);
    let v = v.clamp(0.0, tex.height() as f32 - 1.001);
    let (u0, v0) = (u.floor() as u32, v.floor() as u32);
    let (fu, fv) = (u - u0 as f32, v - v0 as f32);
    let p = |dx: u32, dy: u32| tex.get_pixel(u0 + dx, v0 + dy).0;
    let (a, b, c, d) = (p(0, 0), p(1, 0), p(0, 1), p(1, 1));
    let mut out = [0u8; 4];
    for i in 0..4 {
        let top = a[i] as f32 * (1.0 - fu) + b[i] as f32 * fu;
        let bot = c[i] as f32 * (1.0 - fu) + d[i] as f32 * fu;
        out[i] = (top * (1.0 - fv) + bot * fv).round() as u8;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A solid white circular source projects to a tinted, vertically-squashed
    /// ellipse: wider than tall, coloured by the tint, transparent outside.
    fn white_disc_texture(n: u32) -> RgbaImage {
        let mut t = RgbaImage::new(n, n);
        let c = n as f32 / 2.0;
        for (x, y, px) in t.enumerate_pixels_mut() {
            let dx = (x as f32 + 0.5 - c) / c;
            let dy = (y as f32 + 0.5 - c) / c;
            if dx * dx + dy * dy <= 1.0 {
                *px = Rgba([255, 255, 255, 255]);
            }
        }
        t
    }

    fn bounds(img: &RgbaImage) -> (i32, i32, i32, i32) {
        let (mut a, mut b, mut c, mut d) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
        for (x, y, px) in img.enumerate_pixels() {
            if px.0[3] > 0 {
                a = a.min(x as i32);
                b = b.max(x as i32);
                c = c.min(y as i32);
                d = d.max(y as i32);
            }
        }
        (a, b, c, d)
    }

    #[test]
    fn projects_tinted_squashed_ellipse_from_texture() {
        let tex = white_disc_texture(128);
        // Isolate the disc projection (no ring/ship overlays) for the tint/squash
        // assertions.
        let p = RadarPlaneParams {
            tilt_deg: 30.0,
            outer_ring_alpha: 0.0,
            ship_marker: false,
            ..Default::default()
        };
        let img = project_radar_disc(400, 400, &tex, None, None, &p);
        let (x0, x1, y0, y1) = bounds(&img);
        let w = (x1 - x0) as f32;
        let h = (y1 - y0) as f32;
        assert!(w > h * 1.4, "tilted disc must be wider than tall (w={w}, h={h})");
        // Disc pixel carries the orange tint, not white.
        let p = img.get_pixel(200, 200).0;
        assert!(p[3] > 0, "disc must be opaque");
        assert!(p[0] > p[2], "tint must be warmer (R>B)");
    }

    #[test]
    fn black_texture_background_is_transparent() {
        // A texture that is all black (zero luminance) yields a fully
        // transparent disc — it reads emissive, not a black fill. (Ring/ship
        // overlays disabled so only the texture mapping is under test.)
        let tex = RgbaImage::from_pixel(64, 64, Rgba([0, 0, 0, 255]));
        let p = RadarPlaneParams { outer_ring_alpha: 0.0, ship_marker: false, ..Default::default() };
        let img = project_radar_disc(200, 200, &tex, None, None, &p);
        assert!(img.pixels().all(|p| p.0[3] == 0), "black texture must composite transparent");
    }

    #[test]
    fn outer_ring_and_ship_marker_draw() {
        // With overlays on, a black texture still gets the tinted outer ring and
        // the white own-ship triangle (the data-driven WidgetCircle + own-ship
        // marker over the disc).
        let tex = RgbaImage::from_pixel(64, 64, Rgba([0, 0, 0, 255]));
        let img = project_radar_disc(400, 400, &tex, None, None, &RadarPlaneParams::default());
        let mut ring = 0u32;
        let mut white = 0u32;
        for px in img.pixels() {
            if px.0[3] > 0 {
                if px.0[0] > 220 && px.0[1] > 220 && px.0[2] > 220 {
                    white += 1;
                } else if px.0[0] > 120 && px.0[2] < px.0[0] {
                    ring += 1;
                }
            }
        }
        assert!(ring > 50, "expected tinted outer-ring pixels, got {ring}");
        assert!(white > 5, "expected white own-ship marker pixels, got {white}");
    }

    #[test]
    fn sweep_wedge_composites_over_disc() {
        // Black (transparent) disc so only the sweep shows; a sweep bright in the
        // left half. With sweep_alpha > 0 the projection gains tinted pixels.
        let disc = RgbaImage::from_pixel(64, 64, Rgba([0, 0, 0, 255]));
        let mut sweep = RgbaImage::new(64, 64);
        for (x, _y, px) in sweep.enumerate_pixels_mut() {
            if x < 32 {
                *px = Rgba([255, 255, 255, 255]);
            }
        }
        let p = RadarPlaneParams {
            outer_ring_alpha: 0.0,
            ship_marker: false,
            sweep_alpha: 0.6,
            ..Default::default()
        };
        let img = project_radar_disc(200, 200, &disc, Some(&sweep), None, &p);
        let lit = img.pixels().filter(|px| px.0[3] > 0).count();
        assert!(lit > 100, "sweep wedge must composite over the (black) disc, got {lit}");
        // No sweep texture → nothing (the black disc stays transparent).
        let none = project_radar_disc(200, 200, &disc, None, None, &p);
        assert!(none.pixels().all(|px| px.0[3] == 0), "no sweep + black disc → transparent");
    }

    #[test]
    fn spokes_are_data_driven_soft_and_per_colour() {
        // Black (transparent) disc, no ring/ship/sweep — only authored spokes.
        // One Accent-orange cardinal (North, anchor 0.5,0.2, len 0.4) and one
        // Base-grey diagonal (anchor 0.75,0.25, 45°, len 0.3) — like the real
        // Circle_Line nodes' per-spoke colours.
        let disc = RgbaImage::from_pixel(64, 64, Rgba([0, 0, 0, 255]));
        let spokes = vec![
            RadarSpoke {
                anchor: [0.5, 0.2],
                length_frac: 0.4,
                width_frac: 0.002,
                rotation_deg: 0.0,
                colour: [1.0, 0.6, 0.2],
                alpha: 0.5,
            },
            RadarSpoke {
                anchor: [0.75, 0.25],
                length_frac: 0.3,
                width_frac: 0.002,
                rotation_deg: 45.0,
                colour: [0.7, 0.7, 0.7],
                alpha: 0.5,
            },
        ];
        let p = RadarPlaneParams {
            outer_ring_alpha: 0.0,
            ship_marker: false,
            intensity: 1.0,
            spoke_glow: 0.23,
            spoke_inner_alpha: 0.5,
            spokes: spokes.clone(),
            ..Default::default()
        };
        let img = project_radar_disc(400, 400, &disc, None, None, &p);
        // Both colours present: warm (R>B) from the Accent spoke, neutral
        // (R≈G≈B) from the Base spoke.
        let warm = img.pixels().filter(|px| px.0[3] > 0 && px.0[0] > px.0[2] + 10).count();
        let neutral = img
            .pixels()
            .filter(|px| px.0[3] > 0 && (px.0[0] as i32 - px.0[2] as i32).abs() <= 6)
            .count();
        assert!(warm > 20, "Accent (warm) spoke must draw, got {warm}");
        assert!(neutral > 20, "Base (neutral) spoke must draw, got {neutral}");
        // SOFT, not a crisp 1px line: a meaningful share of lit pixels carry a
        // partial alpha (the Gaussian glow falloff), not just full opacity.
        let lit = img.pixels().filter(|px| px.0[3] > 0).count();
        let partial = img.pixels().filter(|px| px.0[3] > 0 && px.0[3] < 200).count();
        assert!(lit > 60, "spokes must cover a soft band, got {lit}");
        assert!(
            partial * 2 > lit,
            "spokes must be soft (mostly partial-alpha edges), got {partial}/{lit}"
        );
        // No spokes → fully transparent (black disc).
        let none = project_radar_disc(
            400,
            400,
            &disc,
            None,
            None,
            &RadarPlaneParams { outer_ring_alpha: 0.0, ship_marker: false, ..Default::default() },
        );
        assert!(none.pixels().all(|px| px.0[3] == 0), "no spokes + black disc → transparent");
    }

    #[test]
    fn empty_dims_do_not_panic() {
        let tex = white_disc_texture(16);
        let _ = project_radar_disc(0, 0, &tex, None, None, &RadarPlaneParams::default());
        let _ = project_radar_disc(100, 100, &RgbaImage::new(0, 0), None, None, &RadarPlaneParams::default());
    }
}
