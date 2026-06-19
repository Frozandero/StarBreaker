//! Animation binding diagnostics for StarBreaker.
//!
//! These commands are intentionally report-only.  They do not try to guess
//! missing bindings or mutate exported sidecars.  The goal is to compare the
//! hash-only CAF JSON tracks against the names that StarBreaker can recover
//! from the original CryEngine rig files and from the converted GLB output.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use clap::Subcommand;
use serde_json::{json, Value};

use crate::error::{CliError, Result};

#[derive(Subcommand)]
pub enum AnimationCommand {
    /// Build a report comparing animation JSON track hashes against rig, GLB,
    /// meshsetup, and chrparams names.
    BindingReport {
        /// Folder containing exported animation JSON files.  The folder is
        /// scanned recursively for *.json.
        #[arg(long)]
        animation_folder: Option<PathBuf>,
        /// One or more individual animation JSON files to include.
        #[arg(long)]
        animation_json: Vec<PathBuf>,
        /// Source .cga file to inspect for NMC/rig node names.
        #[arg(long)]
        cga: Option<PathBuf>,
        /// Source .cgam file to inspect for NMC/rig node names.
        #[arg(long)]
        cgam: Option<PathBuf>,
        /// Source .meshsetup file to inspect for Joint names.
        #[arg(long)]
        meshsetup: Option<PathBuf>,
        /// Source .chrparams file to inspect for animation references.
        #[arg(long)]
        chrparams: Option<PathBuf>,
        /// Converted .glb file to inspect for exported node names.
        #[arg(long)]
        glb: Option<PathBuf>,
        /// Write machine-readable JSON report to this path.  If omitted, JSON
        /// is printed to stdout.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Optional Markdown summary report path.
        #[arg(long)]
        markdown: Option<PathBuf>,
    },
}

impl AnimationCommand {
    pub fn run(self) -> Result<()> {
        match self {
            Self::BindingReport {
                animation_folder,
                animation_json,
                cga,
                cgam,
                meshsetup,
                chrparams,
                glb,
                out,
                markdown,
            } => binding_report(BindingReportArgs {
                animation_folder,
                animation_json,
                cga,
                cgam,
                meshsetup,
                chrparams,
                glb,
                out,
                markdown,
            }),
        }
    }
}

struct BindingReportArgs {
    animation_folder: Option<PathBuf>,
    animation_json: Vec<PathBuf>,
    cga: Option<PathBuf>,
    cgam: Option<PathBuf>,
    meshsetup: Option<PathBuf>,
    chrparams: Option<PathBuf>,
    glb: Option<PathBuf>,
    out: Option<PathBuf>,
    markdown: Option<PathBuf>,
}

#[derive(Default)]
struct NameSources {
    by_source: BTreeMap<String, BTreeSet<String>>,
}

impl NameSources {
    fn add(&mut self, source: impl Into<String>, name: impl AsRef<str>) {
        let name = name.as_ref().trim();
        if name.is_empty() {
            return;
        }
        self.by_source
            .entry(source.into())
            .or_default()
            .insert(name.to_string());
    }

    fn source_counts_json(&self) -> Value {
        let mut map = serde_json::Map::new();
        for (source, names) in &self.by_source {
            map.insert(source.clone(), json!(names.len()));
        }
        Value::Object(map)
    }

    fn all_names(&self) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        for names in self.by_source.values() {
            out.extend(names.iter().cloned());
        }
        out
    }
}

#[derive(Default)]
struct ClipStats {
    file: String,
    name: String,
    source_skeleton_paths: BTreeSet<String>,
    bone_entries: usize,
    channel_variants: usize,
    animated_track_entries: usize,
    hash_entries: BTreeSet<u32>,
    source_node_names: BTreeSet<String>,
}

fn binding_report(args: BindingReportArgs) -> Result<()> {
    let mut names = NameSources::default();
    let mut source_reports = Vec::<Value>::new();

    if let Some(path) = args.cga.as_deref() {
        let report = add_cry_rig_names(path, "cga", &mut names)?;
        source_reports.push(report);
    }
    if let Some(path) = args.cgam.as_deref() {
        let report = add_cry_rig_names(path, "cgam", &mut names)?;
        source_reports.push(report);
    }
    if let Some(path) = args.meshsetup.as_deref() {
        let report = add_meshsetup_names(path, &mut names)?;
        source_reports.push(report);
    }
    if let Some(path) = args.chrparams.as_deref() {
        let report = add_chrparams_names(path, &mut names)?;
        source_reports.push(report);
    }
    if let Some(path) = args.glb.as_deref() {
        let report = add_glb_names(path, &mut names)?;
        source_reports.push(report);
    }

    let animation_files = collect_animation_json_files(
        args.animation_folder.as_deref(),
        &args.animation_json,
    )?;

    if animation_files.is_empty() {
        return Err(CliError::InvalidInput(
            "no animation JSON files were supplied/found".into(),
        ));
    }

    let mut clip_stats = Vec::<ClipStats>::new();
    for file in &animation_files {
        clip_stats.extend(read_animation_file_stats(file)?);
    }

    let name_hash_index = build_name_hash_index(&names);
    let report = build_report_json(&names, source_reports, &animation_files, &clip_stats, &name_hash_index);
    let pretty = serde_json::to_string_pretty(&report)?;

    if let Some(out) = args.out.as_deref() {
        write_text(out, &pretty)?;
        eprintln!("Animation binding report written to {}", out.display());
    } else {
        println!("{pretty}");
    }

    if let Some(md_path) = args.markdown.as_deref() {
        let md = build_markdown_report(&report);
        write_text(md_path, &md)?;
        eprintln!("Markdown summary written to {}", md_path.display());
    }

    Ok(())
}

fn write_text(path: &Path, text: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| CliError::IoPath {
                source: e,
                path: parent.display().to_string(),
            })?;
        }
    }
    std::fs::write(path, text).map_err(|e| CliError::IoPath {
        source: e,
        path: path.display().to_string(),
    })
}

fn collect_animation_json_files(folder: Option<&Path>, explicit: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for path in explicit {
        if path.is_file() {
            files.push(path.clone());
        } else {
            return Err(CliError::NotFound(format!(
                "animation JSON file not found: {}",
                path.display()
            )));
        }
    }
    if let Some(folder) = folder {
        if !folder.is_dir() {
            return Err(CliError::NotFound(format!(
                "animation folder not found: {}",
                folder.display()
            )));
        }
        collect_json_recursive(folder, &mut files)?;
    }
    files.sort();
    files.dedup();
    Ok(files)
}

fn collect_json_recursive(folder: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(folder).map_err(|e| CliError::IoPath {
        source: e,
        path: folder.display().to_string(),
    })? {
        let entry = entry.map_err(|e| CliError::IoPath {
            source: e,
            path: folder.display().to_string(),
        })?;
        let path = entry.path();
        let ty = entry.file_type().map_err(|e| CliError::IoPath {
            source: e,
            path: path.display().to_string(),
        })?;
        if ty.is_dir() {
            collect_json_recursive(&path, files)?;
        } else if ty.is_file()
            && path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
        {
            files.push(path);
        }
    }
    Ok(())
}

fn add_cry_rig_names(path: &Path, label: &str, names: &mut NameSources) -> Result<Value> {
    let data = read_file(path)?;
    let rig_names = starbreaker_3d::skeleton::parse_rig_node_names(&data).unwrap_or_default();
    let nmc = starbreaker_3d::nmc::parse_nmc_full(&data);
    let (nmc_node_count, nmc_geom_node_count, nmc_non_geom_node_count) = match &nmc {
        Some((nodes, _)) => {
            let geom = nodes.iter().filter(|node| node.geometry_type == 0).count();
            (nodes.len(), geom, nodes.len().saturating_sub(geom))
        }
        None => (0, 0, 0),
    };
    for name in &rig_names {
        names.add(label, name);
    }
    Ok(json!({
        "source": label,
        "path": path.display().to_string(),
        "file_size": data.len(),
        "rig_node_name_count": rig_names.len(),
        "nmc_node_count": nmc_node_count,
        "nmc_geometry_node_count": nmc_geom_node_count,
        "nmc_non_geometry_node_count": nmc_non_geom_node_count,
        "sample_names": sample_strings(rig_names.iter().map(|s| s.as_str()), 40),
    }))
}

fn add_meshsetup_names(path: &Path, names: &mut NameSources) -> Result<Value> {
    let text = read_text_lossy(path)?;
    let joints = extract_attr_values(&text, "Joint");
    for name in &joints {
        names.add("meshsetup", name);
    }
    Ok(json!({
        "source": "meshsetup",
        "path": path.display().to_string(),
        "joint_count": joints.len(),
        "sample_names": sample_strings(joints.iter().map(|s| s.as_str()), 40),
    }))
}

fn add_chrparams_names(path: &Path, names: &mut NameSources) -> Result<Value> {
    let text = read_text_lossy(path)?;
    let animation_names = extract_attr_values(&text, "name");
    let animation_paths = extract_attr_values(&text, "path");
    for name in &animation_names {
        names.add("chrparams_animation_name", name);
    }
    for path_value in &animation_paths {
        if let Some(stem) = path_value.rsplit(['/', '\\']).next() {
            names.add("chrparams_animation_path_stem", stem.trim_end_matches(".caf"));
        }
    }
    Ok(json!({
        "source": "chrparams",
        "path": path.display().to_string(),
        "animation_name_count": animation_names.len(),
        "animation_path_count": animation_paths.len(),
        "sample_animation_names": sample_strings(animation_names.iter().map(|s| s.as_str()), 40),
        "sample_animation_paths": sample_strings(animation_paths.iter().map(|s| s.as_str()), 40),
    }))
}

fn add_glb_names(path: &Path, names: &mut NameSources) -> Result<Value> {
    let data = read_file(path)?;
    let json = read_glb_json(&data).ok_or_else(|| {
        CliError::InvalidInput(format!("failed to parse GLB JSON chunk: {}", path.display()))
    })?;
    let mut node_names = Vec::new();
    if let Some(nodes) = json.get("nodes").and_then(|v| v.as_array()) {
        for node in nodes {
            if let Some(name) = node.get("name").and_then(|v| v.as_str()) {
                node_names.push(name.to_string());
                names.add("glb_node", name);
            }
        }
    }
    let mesh_count = json.get("meshes").and_then(|v| v.as_array()).map_or(0, Vec::len);
    let skin_count = json.get("skins").and_then(|v| v.as_array()).map_or(0, Vec::len);
    let animation_count = json.get("animations").and_then(|v| v.as_array()).map_or(0, Vec::len);
    Ok(json!({
        "source": "glb",
        "path": path.display().to_string(),
        "node_count": node_names.len(),
        "mesh_count": mesh_count,
        "skin_count": skin_count,
        "animation_count": animation_count,
        "sample_names": sample_strings(node_names.iter().map(|s| s.as_str()), 40),
    }))
}

fn read_file(path: &Path) -> Result<Vec<u8>> {
    std::fs::read(path).map_err(|e| CliError::IoPath {
        source: e,
        path: path.display().to_string(),
    })
}

fn read_text_lossy(path: &Path) -> Result<String> {
    let data = read_file(path)?;
    Ok(String::from_utf8_lossy(&data).into_owned())
}

fn read_glb_json(data: &[u8]) -> Option<Value> {
    if data.len() < 20 || &data[0..4] != b"glTF" {
        return None;
    }
    let version = u32::from_le_bytes(data[4..8].try_into().ok()?);
    if version != 2 {
        return None;
    }
    let mut offset = 12usize;
    while offset.checked_add(8)? <= data.len() {
        let len = u32::from_le_bytes(data[offset..offset + 4].try_into().ok()?) as usize;
        let chunk_type = u32::from_le_bytes(data[offset + 4..offset + 8].try_into().ok()?);
        offset += 8;
        let end = offset.checked_add(len)?;
        if end > data.len() {
            return None;
        }
        if chunk_type == 0x4E4F534A {
            return serde_json::from_slice(&data[offset..end]).ok();
        }
        offset = end;
    }
    None
}

fn extract_attr_values(text: &str, attr: &str) -> Vec<String> {
    let needle = format!("{attr}=\"");
    let mut values = Vec::new();
    let mut rest = text;
    while let Some(pos) = rest.find(&needle) {
        let start = pos + needle.len();
        let after = &rest[start..];
        if let Some(end) = after.find('"') {
            values.push(after[..end].to_string());
            rest = &after[end + 1..];
        } else {
            break;
        }
    }
    values
}

fn read_animation_file_stats(path: &Path) -> Result<Vec<ClipStats>> {
    let data = read_file(path)?;
    let value: Value = serde_json::from_slice(&data)?;
    let mut clips = Vec::new();
    if let Some(array) = value.as_array() {
        for (i, clip) in array.iter().enumerate() {
            clips.push(clip_stats_from_value(path, Some(i), clip));
        }
    } else {
        clips.push(clip_stats_from_value(path, None, &value));
    }
    Ok(clips)
}

fn clip_stats_from_value(path: &Path, array_index: Option<usize>, clip: &Value) -> ClipStats {
    let fallback_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("animation")
        .to_string();
    let mut stats = ClipStats {
        file: path.display().to_string(),
        name: clip
            .get("name")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| match array_index {
                Some(i) => format!("{fallback_name}[{i}]"),
                None => fallback_name,
            }),
        ..Default::default()
    };

    let Some(bones) = clip.get("bones").and_then(|v| v.as_object()) else {
        return stats;
    };

    stats.bone_entries = bones.len();
    for (bone_key, channel_value) in bones {
        if let Some(hash) = parse_hash_key(bone_key) {
            stats.hash_entries.insert(hash);
        }
        if let Some(obj) = channel_value.as_object() {
            stats.channel_variants += 1;
            collect_channel_metadata(obj, &mut stats);
        } else if let Some(array) = channel_value.as_array() {
            for variant in array {
                if let Some(obj) = variant.as_object() {
                    stats.channel_variants += 1;
                    collect_channel_metadata(obj, &mut stats);
                }
            }
        }
    }

    stats
}

fn collect_channel_metadata(obj: &serde_json::Map<String, Value>, stats: &mut ClipStats) {
    let has_position = obj
        .get("has_position")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let has_rotation = obj
        .get("has_rotation")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if has_position || has_rotation {
        stats.animated_track_entries += 1;
    }
    if let Some(path) = obj.get("source_skeleton_path").and_then(|v| v.as_str()) {
        stats.source_skeleton_paths.insert(path.to_string());
    }
    if let Some(name) = obj.get("source_node_name").and_then(|v| v.as_str()) {
        stats.source_node_names.insert(name.to_string());
    }
}

fn parse_hash_key(key: &str) -> Option<u32> {
    let trimmed = key.trim();
    if let Some(hex) = trimmed.strip_prefix("0x").or_else(|| trimmed.strip_prefix("0X")) {
        return u32::from_str_radix(hex, 16).ok();
    }
    trimmed.parse::<u32>().ok()
}

#[derive(Clone)]
struct HashCandidate {
    source: String,
    name: String,
    variant: &'static str,
}

type HashIndex = BTreeMap<u32, Vec<HashCandidate>>;

fn build_name_hash_index(names: &NameSources) -> HashIndex {
    let mut index: HashIndex = BTreeMap::new();
    for (source, source_names) in &names.by_source {
        for name in source_names {
            add_hash_candidate(&mut index, source, name, name, "exact");
            let lower = name.to_ascii_lowercase();
            if lower != *name {
                add_hash_candidate(&mut index, source, name, &lower, "lowercase");
            }
            let upper = name.to_ascii_uppercase();
            if upper != *name {
                add_hash_candidate(&mut index, source, name, &upper, "uppercase");
            }
        }
    }
    index
}

fn add_hash_candidate(
    index: &mut HashIndex,
    source: &str,
    original_name: &str,
    hashed_name: &str,
    variant: &'static str,
) {
    let hash = starbreaker_3d::animation::bone_name_hash(hashed_name);
    index.entry(hash).or_default().push(HashCandidate {
        source: source.to_string(),
        name: original_name.to_string(),
        variant,
    });
}

fn build_report_json(
    names: &NameSources,
    source_reports: Vec<Value>,
    animation_files: &[PathBuf],
    clip_stats: &[ClipStats],
    name_hash_index: &HashIndex,
) -> Value {
    let mut all_track_hashes = BTreeSet::<u32>::new();
    let mut clips_with_any_source_names = 0usize;
    let mut clips_with_unresolved_hashes = 0usize;
    let mut exact_hits = 0usize;
    let mut case_variant_hits = 0usize;
    let mut unresolved_hashes = BTreeSet::<u32>::new();

    let clip_values: Vec<Value> = clip_stats
        .iter()
        .map(|clip| {
            if !clip.source_node_names.is_empty() {
                clips_with_any_source_names += 1;
            }
            let mut hash_reports = Vec::new();
            let mut clip_unresolved = false;
            for hash in &clip.hash_entries {
                all_track_hashes.insert(*hash);
                let candidates = name_hash_index.get(hash).cloned().unwrap_or_default();
                let exact: Vec<Value> = candidates
                    .iter()
                    .filter(|c| c.variant == "exact")
                    .map(candidate_json)
                    .collect();
                let variants: Vec<Value> = candidates
                    .iter()
                    .filter(|c| c.variant != "exact")
                    .map(candidate_json)
                    .collect();
                if !exact.is_empty() {
                    exact_hits += 1;
                } else if !variants.is_empty() {
                    case_variant_hits += 1;
                } else {
                    unresolved_hashes.insert(*hash);
                    clip_unresolved = true;
                }
                hash_reports.push(json!({
                    "hash_hex": format!("0x{hash:08X}"),
                    "hash_decimal": *hash,
                    "exact_name_hash_matches": exact,
                    "case_variant_name_hash_matches": variants,
                }));
            }
            if clip_unresolved {
                clips_with_unresolved_hashes += 1;
            }
            json!({
                "file": clip.file,
                "name": clip.name,
                "source_skeleton_paths": clip.source_skeleton_paths.iter().cloned().collect::<Vec<_>>(),
                "bone_entries": clip.bone_entries,
                "channel_variants": clip.channel_variants,
                "animated_track_entries": clip.animated_track_entries,
                "hash_entry_count": clip.hash_entries.len(),
                "source_node_name_count": clip.source_node_names.len(),
                "source_node_names": clip.source_node_names.iter().cloned().collect::<Vec<_>>(),
                "hashes": hash_reports,
            })
        })
        .collect();

    let all_candidate_names = names.all_names();
    json!({
        "report_type": "StarBreaker animation binding report",
        "summary": {
            "animation_json_files": animation_files.len(),
            "clips": clip_stats.len(),
            "clips_with_any_source_node_name": clips_with_any_source_names,
            "clips_with_unresolved_hashes": clips_with_unresolved_hashes,
            "unique_track_hashes": all_track_hashes.len(),
            "unique_candidate_names": all_candidate_names.len(),
            "unique_unresolved_hashes": unresolved_hashes.len(),
            "track_hashes_with_exact_candidate_name_hits": exact_hits,
            "track_hashes_with_case_variant_candidate_name_hits": case_variant_hits,
            "candidate_name_counts_by_source": names.source_counts_json(),
        },
        "source_reports": source_reports,
        "unresolved_hashes": unresolved_hashes.iter().map(|h| format!("0x{h:08X}")).collect::<Vec<_>>(),
        "clips": clip_values,
    })
}

fn candidate_json(candidate: &HashCandidate) -> Value {
    json!({
        "source": candidate.source,
        "name": candidate.name,
        "hash_variant": candidate.variant,
    })
}

fn build_markdown_report(report: &Value) -> String {
    let summary = &report["summary"];
    let mut out = String::new();
    out.push_str("# StarBreaker Animation Binding Report\n\n");
    out.push_str("## Summary\n\n");
    out.push_str(&format!("- Animation JSON files: {}\n", summary["animation_json_files"]));
    out.push_str(&format!("- Clips: {}\n", summary["clips"]));
    out.push_str(&format!("- Clips with source_node_name: {}\n", summary["clips_with_any_source_node_name"]));
    out.push_str(&format!("- Clips with unresolved hashes: {}\n", summary["clips_with_unresolved_hashes"]));
    out.push_str(&format!("- Unique track hashes: {}\n", summary["unique_track_hashes"]));
    out.push_str(&format!("- Unique unresolved hashes: {}\n", summary["unique_unresolved_hashes"]));
    out.push_str(&format!("- Exact candidate-name hash hits: {}\n", summary["track_hashes_with_exact_candidate_name_hits"]));
    out.push_str(&format!("- Case-variant candidate-name hash hits: {}\n\n", summary["track_hashes_with_case_variant_candidate_name_hits"]));

    out.push_str("## Candidate Name Counts\n\n");
    if let Some(map) = summary["candidate_name_counts_by_source"].as_object() {
        for (source, count) in map {
            out.push_str(&format!("- {source}: {count}\n"));
        }
    }

    out.push_str("\n## Unresolved Hashes\n\n");
    if let Some(values) = report["unresolved_hashes"].as_array() {
        if values.is_empty() {
            out.push_str("None.\n");
        } else {
            for value in values.iter().take(200) {
                out.push_str(&format!("- {}\n", value.as_str().unwrap_or("?")));
            }
            if values.len() > 200 {
                out.push_str(&format!("- ... {} more\n", values.len() - 200));
            }
        }
    }

    out.push_str("\n## Clips With Unresolved Hashes\n\n");
    if let Some(clips) = report["clips"].as_array() {
        for clip in clips {
            let unresolved_count = clip["hashes"]
                .as_array()
                .map(|hashes| {
                    hashes
                        .iter()
                        .filter(|h| {
                            h["exact_name_hash_matches"].as_array().map_or(true, Vec::is_empty)
                                && h["case_variant_name_hash_matches"].as_array().map_or(true, Vec::is_empty)
                        })
                        .count()
                })
                .unwrap_or(0);
            if unresolved_count == 0 {
                continue;
            }
            out.push_str(&format!(
                "- `{}` — unresolved hashes: {}, source names: {}\n",
                clip["name"].as_str().unwrap_or("?"),
                unresolved_count,
                clip["source_node_name_count"],
            ));
        }
    }

    out
}

fn sample_strings<'a>(iter: impl Iterator<Item = &'a str>, limit: usize) -> Vec<String> {
    iter.take(limit).map(str::to_string).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hash_key_accepts_hex_and_decimal() {
        assert_eq!(parse_hash_key("0x6B65934C"), Some(0x6B65934C));
        assert_eq!(parse_hash_key("0X10"), Some(16));
        assert_eq!(parse_hash_key(" 255 "), Some(255));
        assert_eq!(parse_hash_key("not_a_hash"), None);
        assert_eq!(parse_hash_key(""), None);
    }

    #[test]
    fn extract_attr_values_reads_quoted_attributes() {
        let xml = r#"<Joint Joint="hip" /><Joint Joint="spine_01" />"#;
        assert_eq!(extract_attr_values(xml, "Joint"), vec!["hip", "spine_01"]);
        assert!(extract_attr_values(xml, "missing").is_empty());
    }

    #[test]
    fn build_name_hash_index_indexes_exact_and_case_variants() {
        let mut names = NameSources::default();
        names.add("cga", "Shutter_15");
        let index = build_name_hash_index(&names);
        // The exact-case name resolves its own CRC32 hash.
        let exact = bone_name_hash_for_test("Shutter_15");
        assert!(index.get(&exact).is_some_and(|c| c.iter().any(|h| h.variant == "exact")));
        // The lowercase variant is indexed under the lowercase hash.
        let lower = bone_name_hash_for_test("shutter_15");
        assert!(index.get(&lower).is_some_and(|c| c.iter().any(|h| h.variant == "lowercase")));
    }

    fn bone_name_hash_for_test(name: &str) -> u32 {
        starbreaker_3d::animation::bone_name_hash(name)
    }

    #[test]
    fn read_glb_json_rejects_non_glb() {
        assert!(read_glb_json(b"not a glb file at all!!").is_none());
        assert!(read_glb_json(b"glTF").is_none());
    }
}
