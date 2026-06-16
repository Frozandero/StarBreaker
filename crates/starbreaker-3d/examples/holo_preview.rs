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
    let mesh = match p4k.read_file(&cga) {
        Ok(primary) => starbreaker_3d::parse_skin_positioned(&data, &primary).expect("parse skin + nmc"),
        Err(_) => parse_skin(&data).expect("parse skin"),
    };
    eprintln!(
        "verts={} indices={} tris={} submeshes={} bbox_min={:?} bbox_max={:?}",
        mesh.positions.len(),
        mesh.indices.len(),
        mesh.indices.len() / 3,
        mesh.submeshes.len(),
        mesh.model_min,
        mesh.model_max
    );
    if std::env::var("HOLO_SUBMESH_DUMP").is_ok() {
        for (i, s) in mesh.submeshes.iter().enumerate() {
            let st = s.first_vertex as usize;
            let en = (st + s.num_vertices as usize).min(mesh.positions.len());
            if en <= st { continue; }
            let mut lo = [f32::MAX; 3];
            let mut hi = [f32::MIN; 3];
            let mut c = [0.0f64; 3];
            for p in &mesh.positions[st..en] {
                for k in 0..3 { lo[k] = lo[k].min(p[k]); hi[k] = hi[k].max(p[k]); c[k] += p[k] as f64; }
            }
            let n = (en - st) as f64;
            eprintln!(
                "  sub[{i:3}] mat={} verts={} centroid=({:.1},{:.1},{:.1}) bbox=({:.1},{:.1},{:.1})..({:.1},{:.1},{:.1})",
                s.material_id, en - st,
                c[0]/n, c[1]/n, c[2]/n, lo[0],lo[1],lo[2], hi[0],hi[1],hi[2],
            );
        }
    }

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
