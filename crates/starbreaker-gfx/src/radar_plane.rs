//! CPU rasteriser for the MFD-radar scope plane.
//!
//! The cockpit radar (`Screen_Radar_RTT` → `MapDisplayMaster` → radar mode →
//! `StarMapDisplayRTT` → `mapdisplaystarmap_window`) draws its scope through a
//! `BuildingBlocks_WidgetWindow` (`rendererType: "Primitive"`, material
//! `Materials/UI/Starmap/map_window.mtl`, `camera { fieldOfView: 20 }`): a
//! render-to-texture WINDOW that projects a 3D radar plane (`PlayerRadarPlane`:
//! concentric ripple rings, radial spokes, the own-ship marker, multi-altitude
//! navigation grids) through a 3D camera. The 2D-UI pipeline can't natively
//! produce that, so — exactly as the SELF-STATUS own-vehicle hologram is
//! reproduced by [`crate::mesh_holo`] — this module rasterises the scope on the
//! CPU and the headless UI pipeline composites it into the window node.
//!
//! The disc is a flat plane viewed from above and tilted back, so its
//! concentric circles project to concentric ELLIPSES and its radial spokes to
//! lines fanning out from the centre. The engine's at-rest camera transform is
//! NOT in the static UI data (it is pushed at runtime via
//! `/MapNamespace/GeneralMapData/DisplayPosition` + `DisplayOrientation`, absent
//! at static rest), so the viewing tilt / fit are owner-tuned view constants —
//! the same basis as the hologram's owner-tuned yaw/pitch/fit — captured in
//! [`RadarPlaneParams`]. The ring / spoke / tick counts mirror the reference
//! scope. The tint is data-driven by the caller (the node's palette colour).
//!
//! Input is intentionally primitive (counts + angles + an RGBA tint) so this
//! crate keeps depending only on `image`.

use image::{Rgba, RgbaImage};

/// Camera and style parameters for [`render_radar_plane`].
#[derive(Debug, Clone)]
pub struct RadarPlaneParams {
    /// Tilt of the disc back from edge-on, in degrees. The disc's circles
    /// project to ellipses whose vertical axis is scaled by `sin(tilt)`:
    /// `90°` is a pure top-down view (circles stay circles), smaller tilts
    /// squash them. The reference scope reads ~`37°` (minor/major ≈ 0.6).
    /// Owner-tuned view constant (the engine's runtime camera is not in the
    /// static data — see module docs).
    pub tilt_deg: f32,
    /// Number of concentric range rings (the reference shows a bright outer
    /// boundary plus fainter inner rings).
    pub ring_count: u32,
    /// Number of radial spokes fanning from the centre (the "compass rose").
    pub spoke_count: u32,
    /// Number of fine tick marks around the outer ring.
    pub tick_count: u32,
    /// Fraction of the smaller image dimension the disc's MAJOR (horizontal)
    /// axis fills, `0.0..=1.0` (the rest is margin).
    pub fit: f32,
    /// Disc-centre vertical position as a fraction of image height
    /// (`0.5` = centre). The reference centres the ship slightly above middle.
    pub centre_y_frac: f32,
    /// RGBA line/tint colour for the rings, spokes and ticks, channels
    /// `0.0..=1.0` (the DRAK scope is orange — the caller passes the node's
    /// palette colour).
    pub tint: [f32; 4],
    /// Alpha of the bright outer ring, `0.0..=1.0`.
    pub outer_alpha: f32,
    /// Alpha of the inner rings / spokes / ticks, `0.0..=1.0` (fainter).
    pub inner_alpha: f32,
    /// Half-width of the own-ship centre triangle as a fraction of the disc's
    /// major radius.
    pub ship_size_frac: f32,
}

impl Default for RadarPlaneParams {
    fn default() -> Self {
        Self {
            tilt_deg: 37.0,
            ring_count: 3,
            spoke_count: 6,
            tick_count: 72,
            fit: 0.82,
            centre_y_frac: 0.5,
            tint: [1.0, 0.62, 0.22, 1.0],
            outer_alpha: 0.85,
            inner_alpha: 0.45,
            ship_size_frac: 0.06,
        }
    }
}

/// Rasterise the radar scope into a fresh `width × height` RGBA image
/// (transparent background; the caller composites it over the screen vignette).
///
/// Geometry: a flat disc in its own plane, tilted back by `tilt_deg`, projected
/// orthographically. A disc point at polar `(r, θ)` (r in `0..=1` of the major
/// radius) maps to screen `centre + (r·cosθ·R, −r·sinθ·R·sin(tilt))`, so circles
/// become ellipses with vertical axis `R·sin(tilt)` and spokes become straight
/// lines from the centre to the ellipse.
pub fn render_radar_plane(width: u32, height: u32, params: &RadarPlaneParams) -> RgbaImage {
    let mut img = RgbaImage::new(width, height);
    if width == 0 || height == 0 {
        return img;
    }

    let cx = width as f32 / 2.0;
    let cy = height as f32 * params.centre_y_frac;
    // Major (horizontal) radius: half of `fit` × the smaller dimension.
    let major = params.fit * (width.min(height) as f32) / 2.0;
    let squash = params.tilt_deg.to_radians().sin().max(0.02);
    let minor = major * squash;

    let line = |t: f32| {
        Rgba([
            (params.tint[0] * 255.0).round() as u8,
            (params.tint[1] * 255.0).round() as u8,
            (params.tint[2] * 255.0).round() as u8,
            (params.tint[3] * t * 255.0).round() as u8,
        ])
    };

    // Map disc polar (r in 0..=1, theta) to screen pixel.
    let project = |r: f32, theta: f32| -> (f32, f32) {
        (cx + r * major * theta.cos(), cy - r * minor * theta.sin())
    };

    // Concentric range rings (ellipses). Outermost is brightest.
    for ring in 1..=params.ring_count {
        let frac = ring as f32 / params.ring_count as f32;
        let alpha = if ring == params.ring_count {
            params.outer_alpha
        } else {
            params.inner_alpha
        };
        draw_ellipse(&mut img, cx, cy, major * frac, minor * frac, line(alpha));
    }

    // Radial spokes from centre to the outer ellipse.
    for s in 0..params.spoke_count {
        let theta = std::f32::consts::TAU * (s as f32 / params.spoke_count as f32);
        let (ex, ey) = project(1.0, theta);
        draw_line(&mut img, cx, cy, ex, ey, line(params.inner_alpha));
    }

    // Fine tick marks just outside the outer ellipse (small radial stubs).
    for k in 0..params.tick_count {
        let theta = std::f32::consts::TAU * (k as f32 / params.tick_count as f32);
        let (x0, y0) = project(0.96, theta);
        let (x1, y1) = project(1.04, theta);
        draw_line(&mut img, x0, y0, x1, y1, line(params.inner_alpha));
    }

    // Own-ship centre triangle (always white, pointing up).
    let white = Rgba([255, 255, 255, 255]);
    let tw = params.ship_size_frac * major;
    let th = tw * 1.3;
    fill_triangle(
        &mut img,
        (cx, cy - th * 0.6),
        (cx - tw, cy + th * 0.4),
        (cx + tw, cy + th * 0.4),
        white,
    );

    img
}

/// Alpha-blend `c` onto pixel `(x, y)` (source-over).
fn blend(img: &mut RgbaImage, x: i32, y: i32, c: Rgba<u8>) {
    if x < 0 || y < 0 || x as u32 >= img.width() || y as u32 >= img.height() {
        return;
    }
    let a = c.0[3] as f32 / 255.0;
    if a <= 0.0 {
        return;
    }
    let px = img.get_pixel_mut(x as u32, y as u32);
    for i in 0..3 {
        px.0[i] = (c.0[i] as f32 * a + px.0[i] as f32 * (1.0 - a)).round() as u8;
    }
    px.0[3] = ((c.0[3] as f32) + (px.0[3] as f32) * (1.0 - a)).round().min(255.0) as u8;
}

/// Draw an axis-aligned ellipse outline (rx horizontal, ry vertical) centred at
/// `(cx, cy)`, sampling enough segments to stay smooth at any size.
fn draw_ellipse(img: &mut RgbaImage, cx: f32, cy: f32, rx: f32, ry: f32, c: Rgba<u8>) {
    let segments = (rx.max(ry) * 6.0).clamp(64.0, 2048.0) as u32;
    let mut prev: Option<(f32, f32)> = None;
    for i in 0..=segments {
        let t = std::f32::consts::TAU * (i as f32 / segments as f32);
        let p = (cx + rx * t.cos(), cy + ry * t.sin());
        if let Some((px, py)) = prev {
            draw_line(img, px, py, p.0, p.1, c);
        }
        prev = Some(p);
    }
}

/// Draw an anti-aliased-ish 1px line (Bresenham-style, blended endpoints).
fn draw_line(img: &mut RgbaImage, x0: f32, y0: f32, x1: f32, y1: f32, c: Rgba<u8>) {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let steps = dx.abs().max(dy.abs()).ceil().max(1.0) as i32;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let x = x0 + dx * t;
        let y = y0 + dy * t;
        blend(img, x.round() as i32, y.round() as i32, c);
    }
}

/// Fill a triangle (scanline, no AA) — used for the small own-ship marker.
fn fill_triangle(img: &mut RgbaImage, a: (f32, f32), b: (f32, f32), c: (f32, f32), col: Rgba<u8>) {
    let min_y = a.1.min(b.1).min(c.1).floor() as i32;
    let max_y = a.1.max(b.1).max(c.1).ceil() as i32;
    let edge = |p: (f32, f32), q: (f32, f32), r: (f32, f32)| {
        (r.0 - p.0) * (q.1 - p.1) - (r.1 - p.1) * (q.0 - p.0)
    };
    let area = edge(a, b, c);
    if area.abs() < 1e-3 {
        return;
    }
    let min_x = a.0.min(b.0).min(c.0).floor() as i32;
    let max_x = a.0.max(b.0).max(c.0).ceil() as i32;
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let p = (x as f32 + 0.5, y as f32 + 0.5);
            let w0 = edge(b, c, p);
            let w1 = edge(c, a, p);
            let w2 = edge(a, b, p);
            let inside = (w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0)
                || (w0 <= 0.0 && w1 <= 0.0 && w2 <= 0.0);
            if inside {
                blend(img, x, y, col);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_non_empty_disc_with_tint() {
        let p = RadarPlaneParams::default();
        let img = render_radar_plane(400, 320, &p);
        // Some pixels carry the orange tint (rings/spokes), some are the white ship.
        let mut orange = 0u32;
        let mut white = 0u32;
        for px in img.pixels() {
            if px.0[3] > 0 {
                if px.0[0] > 200 && px.0[1] > 200 && px.0[2] > 200 {
                    white += 1;
                } else if px.0[0] > 150 && px.0[2] < 120 {
                    orange += 1;
                }
            }
        }
        assert!(orange > 100, "expected orange ring/spoke pixels, got {orange}");
        assert!(white > 5, "expected white ship-marker pixels, got {white}");
    }

    #[test]
    fn tilt_squashes_disc_vertically() {
        // A shallow tilt must make the disc much wider than it is tall.
        let p = RadarPlaneParams { tilt_deg: 30.0, ..Default::default() };
        let img = render_radar_plane(400, 400, &p);
        let (mut min_x, mut max_x, mut min_y, mut max_y) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
        for (x, y, px) in img.enumerate_pixels() {
            if px.0[3] > 0 {
                min_x = min_x.min(x as i32);
                max_x = max_x.max(x as i32);
                min_y = min_y.min(y as i32);
                max_y = max_y.max(y as i32);
            }
        }
        let w = (max_x - min_x) as f32;
        let h = (max_y - min_y) as f32;
        assert!(w > h * 1.3, "tilted disc must be wider than tall (w={w}, h={h})");
    }

    #[test]
    fn empty_dims_do_not_panic() {
        let _ = render_radar_plane(0, 0, &RadarPlaneParams::default());
    }
}
