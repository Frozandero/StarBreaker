//! Preview the MFD-radar scope rasteriser to a PNG for visual calibration.
//!
//! `cargo run -p starbreaker-gfx --example radar_preview -- <out.png> [tilt] [rings] [spokes]`
use starbreaker_gfx::radar_plane::{render_radar_plane, RadarPlaneParams};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = args.get(1).map(String::as_str).unwrap_or("/tmp/radar_preview.png");
    let mut p = RadarPlaneParams::default();
    if let Some(v) = args.get(2).and_then(|s| s.parse().ok()) {
        p.tilt_deg = v;
    }
    if let Some(v) = args.get(3).and_then(|s| s.parse().ok()) {
        p.ring_count = v;
    }
    if let Some(v) = args.get(4).and_then(|s| s.parse().ok()) {
        p.spoke_count = v;
    }
    // Render over a dark DRAK vignette-ish background so it reads like the screen.
    let (w, h) = (1024u32, 834u32);
    let plane = render_radar_plane(w, h, &p);
    let mut img = image::RgbaImage::from_pixel(w, h, image::Rgba([34, 22, 10, 255]));
    for (x, y, px) in plane.enumerate_pixels() {
        let a = px.0[3] as f32 / 255.0;
        if a > 0.0 {
            let bg = img.get_pixel_mut(x, y);
            for i in 0..3 {
                bg.0[i] = (px.0[i] as f32 * a + bg.0[i] as f32 * (1.0 - a)).round() as u8;
            }
        }
    }
    img.save(out).expect("save");
    println!("wrote {out} (tilt={} rings={} spokes={})", p.tilt_deg, p.ring_count, p.spoke_count);
}
