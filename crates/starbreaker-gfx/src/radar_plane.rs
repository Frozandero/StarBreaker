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
}

impl Default for RadarPlaneParams {
    fn default() -> Self {
        Self {
            tilt_deg: 37.0,
            fit: 0.92,
            centre_y_frac: 0.5,
            tint_rgb: [1.0, 0.62, 0.22],
            alpha: 1.0,
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
            // Map disc [-1,1] → texture [0,1] → texel.
            let u = (nx * 0.5 + 0.5) * (tw - 1.0);
            let v = (ny * 0.5 + 0.5) * (th - 1.0);
            let texel = sample_bilinear(texture, u, v);
            // Emissive: luminance → alpha; colour = tint.
            let lum = (0.299 * texel[0] as f32 + 0.587 * texel[1] as f32 + 0.114 * texel[2] as f32)
                / 255.0;
            let src_a = texel[3] as f32 / 255.0;
            let a = lum * src_a * params.alpha;
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
    out
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
        let p = RadarPlaneParams { tilt_deg: 30.0, ..Default::default() };
        let img = project_radar_disc(400, 400, &tex, &p);
        let (x0, x1, y0, y1) = bounds(&img);
        let w = (x1 - x0) as f32;
        let h = (y1 - y0) as f32;
        assert!(w > h * 1.4, "tilted disc must be wider than tall (w={w}, h={h})");
        // Centre pixel carries the orange tint, not white.
        let centre = img.get_pixel(200, 200).0;
        assert!(centre[3] > 0, "disc centre must be opaque");
        assert!(centre[0] > centre[2], "tint must be warmer (R>B)");
    }

    #[test]
    fn black_texture_background_is_transparent() {
        // A texture that is all black (zero luminance) yields a fully
        // transparent projection — the disc reads emissive, not a black fill.
        let tex = RgbaImage::from_pixel(64, 64, Rgba([0, 0, 0, 255]));
        let img = project_radar_disc(200, 200, &tex, &RadarPlaneParams::default());
        assert!(img.pixels().all(|p| p.0[3] == 0), "black texture must composite transparent");
    }

    #[test]
    fn empty_dims_do_not_panic() {
        let tex = white_disc_texture(16);
        let _ = project_radar_disc(0, 0, &tex, &RadarPlaneParams::default());
        let _ = project_radar_disc(100, 100, &RgbaImage::new(0, 0), &RadarPlaneParams::default());
    }
}
