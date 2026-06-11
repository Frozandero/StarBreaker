//! Scratch diagnostic: parse gen_mc_s_emissions standalone vs full resolve,
//! lay out at the live slot size, and print the emission subtree rects.
fn main() {
    let root = "/home/tom/projects/scorg_tools/ships/dcb_canvas/libs/foundry/records";
    let path = format!("{root}/ui/buildingblocks/ships/displays/mfdscreens/mc_mfdcomponents/screens/general/emissions_types/gen_mc_s_emissions.json");
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();

    let fetch = |url: &str| -> Result<serde_json::Value, String> {
        let rel = url.trim_start_matches("file://./").replace("../", "");
        let candidate = format!(
            "{root}/{}",
            rel.split("records/").nth(1).unwrap_or(&rel)
        );
        std::fs::read_to_string(&candidate)
            .map_err(|e| format!("{candidate}: {e}"))
            .and_then(|s| serde_json::from_str(&s).map_err(|e| e.to_string()))
    };

    let defaults = starbreaker_ui::defaults::DefaultValueRegistry::default();
    let scene = starbreaker_ui::bb_resolve::resolve_canvas_graph_with_defaults(
        &json, Some("drak"), &fetch, None, None, &defaults,
    )
    .expect("resolve");
    let result = starbreaker_ui::bb_layout::layout(&scene, 1458, 141);
    for (id, node) in &scene.nodes {
        if node.name.contains("Emission") && !node.name.contains("Container") {
            if let Some(r) = result.rects.get(id) {
                println!(
                    "{id} {} ({:.0},{:.0},{:.0},{:.0}) sizing=({:?},{:?}) active={}",
                    node.name, r.x, r.y, r.w, r.h, node.sizing.width, node.sizing.height, node.is_active
                );
            }
        }
    }
}
