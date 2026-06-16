//! Throwaway diagnostic: decode a ship hull `.cga` and render the SELF-STATUS
//! vehicle hologram to a PNG so the projection/style can be eyeballed before
//! wiring it into the UI pipeline. Usage:
//!   cargo run -p starbreaker-3d --example holo_preview -- [cga_path] [out.png] [tilt_deg] [yaw_deg]

use std::env;

use starbreaker_3d::parse_skin;
use starbreaker_gfx::{render_vehicle_hologram, HologramParams};
use starbreaker_p4k::MappedP4k;

fn main() {
    let p4k_path = env::var("SC_DATA_P4K")
        .expect("set SC_DATA_P4K to the Data.p4k path (e.g. \"$HOME/.../StarCitizen/LIVE/Data.p4k\")");
    let cga = env::args()
        .nth(1)
        .unwrap_or_else(|| r"Data\Objects\Spaceships\Ships\DRAK\clipper\exterior\drak_clipper.cga".to_string());
    let out = env::args().nth(2).unwrap_or_else(|| "/tmp/holo.png".to_string());
    let tilt: f32 = env::args().nth(3).and_then(|s| s.parse().ok()).unwrap_or(15.0);
    let yaw: f32 = env::args().nth(4).and_then(|s| s.parse().ok()).unwrap_or(0.0);

    let p4k = MappedP4k::open(&p4k_path).expect("open p4k");
    let companion = format!("{cga}m");
    let data = p4k
        .read_file(&companion)
        .or_else(|_| p4k.read_file(&cga))
        .expect("read geometry");
    let mesh = parse_skin(&data).expect("parse skin");
    eprintln!(
        "verts={} indices={} tris={} bbox_min={:?} bbox_max={:?}",
        mesh.positions.len(),
        mesh.indices.len(),
        mesh.indices.len() / 3,
        mesh.model_min,
        mesh.model_max
    );

    let params = HologramParams {
        tilt_back_deg: tilt,
        yaw_deg: yaw,
        tint: [0.45, 0.75, 1.0, 1.0], // preview-only light blue
        ..HologramParams::default()
    };
    let t = std::time::Instant::now();
    let img = render_vehicle_hologram(&mesh.positions, &mesh.indices, 512, 512, &params);
    eprintln!("rastered in {:.2}s", t.elapsed().as_secs_f32());
    img.save(&out).expect("save png");
    eprintln!("wrote {out}");
}
