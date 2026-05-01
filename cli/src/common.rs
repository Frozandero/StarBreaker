use std::collections::HashSet;
use std::path::{Path, PathBuf};

use starbreaker_p4k::MappedP4k;

use crate::error::Result;

pub fn sanitize_export_name(name: &str) -> String {
    let mut cleaned = String::new();
    let mut last_was_space = false;

    for ch in name.chars() {
        if ch.is_alphanumeric() {
            cleaned.push(ch);
            last_was_space = false;
        } else if ch.is_whitespace() || matches!(ch, '_' | '-' | ':' | '/' | '\\') {
            if !cleaned.is_empty() && !last_was_space {
                cleaned.push(' ');
                last_was_space = true;
            }
        }
    }

    let cleaned = cleaned.trim();
    if cleaned.is_empty() {
        "Export".to_string()
    } else {
        cleaned.to_string()
    }
}

pub fn prepare_decomposed_output_root(output_root: &PathBuf, package_name: &str) -> Result<()> {
    if output_root.exists() {
        if output_root.is_file() {
            return Err(crate::error::CliError::InvalidInput(format!(
                "decomposed output root '{}' already exists as a file",
                output_root.display(),
            )));
        }
    }

    let packages_root = output_root.join("Packages");
    let package_root = packages_root.join(package_name);
    if package_root.exists() {
        std::fs::remove_dir_all(&package_root).map_err(|e| crate::error::CliError::IoPath {
            source: e,
            path: package_root.display().to_string(),
        })?;
    }

    std::fs::create_dir_all(&package_root).map_err(|e| crate::error::CliError::IoPath {
        source: e,
        path: package_root.display().to_string(),
    })?;
    Ok(())
}

fn should_skip_existing_decomposed_asset(
    file: &starbreaker_3d::ExportedFile,
    skip_existing_assets: bool,
) -> bool {
    skip_existing_assets && file.kind.is_mesh_or_texture_asset()
}

pub fn write_decomposed_file(
    file: &starbreaker_3d::ExportedFile,
    output_path: &PathBuf,
    skip_existing_assets: bool,
) -> Result<()> {
    if output_path.exists() {
        if !output_path.is_file() {
            return Err(crate::error::CliError::InvalidInput(format!(
                "decomposed output path '{}' already exists as a directory",
                output_path.display(),
            )));
        }
        if should_skip_existing_decomposed_asset(file, skip_existing_assets) {
            return Ok(());
        }
    }

    std::fs::write(output_path, &file.bytes).map_err(|e| crate::error::CliError::IoPath {
        source: e,
        path: output_path.display().to_string(),
    })?;
    Ok(())
}

pub fn collect_existing_decomposed_assets(output_root: &Path) -> Result<HashSet<String>> {
    let data_root = output_root.join("Data");
    let mut existing = HashSet::new();
    if !data_root.exists() {
        return Ok(existing);
    }

    let mut pending = vec![data_root];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).map_err(|e| crate::error::CliError::IoPath {
            source: e,
            path: dir.display().to_string(),
        })? {
            let entry = entry.map_err(|e| crate::error::CliError::IoPath {
                source: e,
                path: dir.display().to_string(),
            })?;
            let path = entry.path();
            let file_type = entry
                .file_type()
                .map_err(|e| crate::error::CliError::IoPath {
                    source: e,
                    path: path.display().to_string(),
                })?;
            if file_type.is_dir() {
                pending.push(path);
                continue;
            }
            if !file_type.is_file() {
                continue;
            }

            let Some(extension) = path.extension().and_then(|ext| ext.to_str()) else {
                continue;
            };
            if !matches!(extension, "glb" | "png" | "dds") {
                continue;
            }

            let relative = path
                .strip_prefix(output_root)
                .map_err(|_| {
                    crate::error::CliError::InvalidInput(format!(
                        "failed to compute relative decomposed asset path for '{}'",
                        path.display(),
                    ))
                })?
                .to_string_lossy()
                .replace('\\', "/")
                .to_ascii_lowercase();
            existing.insert(relative);
        }
    }

    Ok(existing)
}

/// Open P4k from explicit path or auto-discover.
pub fn load_p4k(p4k_path: Option<&Path>) -> Result<MappedP4k> {
    match p4k_path {
        Some(path) => Ok(MappedP4k::open(path)?),
        None => Ok(starbreaker_p4k::open_p4k()?),
    }
}

/// Load DCB bytes from explicit file or extract from P4k.
/// When dcb_path is provided, P4k is optional.
pub fn load_dcb_bytes(
    p4k_path: Option<&Path>,
    dcb_path: Option<&Path>,
) -> Result<(Option<MappedP4k>, Vec<u8>)> {
    if let Some(dcb) = dcb_path {
        let bytes = std::fs::read(dcb).map_err(|e| crate::error::CliError::IoPath {
            source: e,
            path: dcb.display().to_string(),
        })?;
        let p4k = load_p4k(p4k_path).ok();
        return Ok((p4k, bytes));
    }
    let p4k = load_p4k(p4k_path)?;
    let bytes = p4k
        .read_file("Data\\Game2.dcb")
        .or_else(|_| p4k.read_file("Data\\Game.dcb"))?;
    Ok((Some(p4k), bytes))
}

/// Shared glTF export options.
#[derive(clap::Args, Debug)]
pub struct ExportOpts {
    /// Export kind: bundled or decomposed
    #[arg(long, default_value = "bundled")]
    pub kind: String,
    /// Material detail: none, colors, textures, all
    #[arg(long, default_value = "textures")]
    pub materials: String,
    /// Output format: glb or stl
    #[arg(long, default_value = "glb")]
    pub format: String,
    /// Texture mip level (0=full, 2=1/4 res, 4=1/16 res)
    #[arg(long, default_value = "2")]
    pub mip: u32,
    /// LOD level (0=highest detail, 1+=lower)
    #[arg(long, default_value = "1")]
    pub lod: u32,
    /// Skip attached items (weapons, thrusters, landing gear)
    #[arg(long)]
    pub no_attachments: bool,
    /// Skip interior geometry from socpak containers
    #[arg(long)]
    pub no_interior: bool,
    /// Skip lights from interior containers
    #[arg(long)]
    pub no_lights: bool,
    /// Skip writing existing decomposed mesh and texture assets under Data/
    #[arg(long)]
    pub skip_existing_assets: bool,
    /// Include NoDraw faces and sidecar entries in decomposed exports
    #[arg(long)]
    pub include_nodraw: bool,
    /// Include shield helper meshes and shield attachments in exports
    #[arg(long)]
    pub include_shields: bool,
}

impl From<&ExportOpts> for starbreaker_3d::ExportOptions {
    fn from(opts: &ExportOpts) -> Self {
        let kind = match opts.kind.to_lowercase().as_str() {
            "decomposed" => starbreaker_3d::ExportKind::Decomposed,
            _ => starbreaker_3d::ExportKind::Bundled,
        };
        let material_mode = match opts.materials.to_lowercase().as_str() {
            "none" => starbreaker_3d::MaterialMode::None,
            "colors" => starbreaker_3d::MaterialMode::Colors,
            "textures" => starbreaker_3d::MaterialMode::Textures,
            "all" => starbreaker_3d::MaterialMode::All,
            other => {
                eprintln!("Unknown material mode '{other}', using 'textures'");
                starbreaker_3d::MaterialMode::Textures
            }
        };
        let format = match opts.format.to_lowercase().as_str() {
            "stl" => starbreaker_3d::ExportFormat::Stl,
            _ => starbreaker_3d::ExportFormat::Glb,
        };
        starbreaker_3d::ExportOptions {
            kind,
            format,
            material_mode,
            include_attachments: !opts.no_attachments,
            include_interior: !opts.no_interior,
            include_lights: !opts.no_lights,
            include_nodraw: opts.include_nodraw,
            include_shields: opts.include_shields,
            texture_mip: opts.mip,
            lod_level: opts.lod,
            include_animations: matches!(kind, starbreaker_3d::ExportKind::Decomposed),
            apply_default_animation_pose: !matches!(kind, starbreaker_3d::ExportKind::Decomposed),
            default_animation_tags: vec!["landing_gear_extend".to_string()],
        }
    }
}

/// Filter entries by glob or regex.
///
/// For glob patterns, both the pattern and name are normalized to forward
/// slashes before matching — P4k entries use backslashes internally but
/// users shouldn't have to care.
pub fn matches_filter(name: &str, filter: Option<&str>, regex: Option<&regex::Regex>) -> bool {
    if let Some(pattern) = filter {
        let norm_name = name.replace('\\', "/");
        let norm_pattern = pattern.replace('\\', "/");
        return glob_match::glob_match(&norm_pattern, &norm_name);
    }
    if let Some(re) = regex {
        return re.is_match(name);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    // Typical P4k entry paths (backslash-separated, rooted at Data\).
    const XML_DEEP: &str = r"Data\Libs\Subsumption\Missions\mission.xml";
    const XML_SHALLOW: &str = r"Data\game.xml";
    const DDS_DEEP: &str = r"Data\Objects\ships\aurora\texture.dds";
    const DDS_SIBLING: &str = r"Data\Objects\ships\aurora\texture.dds.1";
    const CGF_DEEP: &str = r"Data\Objects\ships\aurora\model.cgf";

    // -----------------------------------------------------------------------
    // matches_filter — glob: extension wildcards
    // -----------------------------------------------------------------------

    #[test]
    fn glob_star_xml_only_matches_root_level() {
        // `*` does NOT cross path separators, so `*.xml` only matches names
        // with no directory component. This is correct glob semantics.
        assert!(!matches_filter(XML_DEEP, Some("*.xml"), None));
        assert!(!matches_filter(XML_SHALLOW, Some("*.xml"), None));
    }

    #[test]
    fn glob_star_dds_only_matches_root_level() {
        assert!(!matches_filter(DDS_DEEP, Some("*.dds"), None));
    }

    #[test]
    fn glob_doublestar_xml_matches_all_depths() {
        // `**/*.xml` matches .xml files at any depth.
        assert!(matches_filter(XML_DEEP, Some("**/*.xml"), None));
        assert!(matches_filter(XML_SHALLOW, Some("**/*.xml"), None));
    }

    #[test]
    fn glob_doublestar_dds_matches_all_depths() {
        assert!(matches_filter(DDS_DEEP, Some("**/*.dds"), None));
    }

    #[test]
    fn glob_doublestar_dds_excludes_siblings() {
        // `.dds.1` is NOT a `.dds` file.
        assert!(!matches_filter(DDS_SIBLING, Some("**/*.dds"), None));
    }

    // -----------------------------------------------------------------------
    // matches_filter — glob: backslash patterns work (normalized)
    // -----------------------------------------------------------------------

    #[test]
    fn glob_backslash_doublestar_works() {
        // Users on Windows may type backslashes — should work identically.
        assert!(matches_filter(XML_DEEP, Some(r"**\*.xml"), None));
        assert!(matches_filter(XML_SHALLOW, Some(r"**\*.xml"), None));
        assert!(matches_filter(DDS_DEEP, Some(r"**\*.dds"), None));
    }

    // -----------------------------------------------------------------------
    // matches_filter — glob: exact paths & prefixes
    // -----------------------------------------------------------------------

    #[test]
    fn glob_exact_path_with_backslashes() {
        assert!(matches_filter(
            XML_DEEP,
            Some(r"Data\Libs\Subsumption\Missions\mission.xml"),
            None,
        ));
    }

    #[test]
    fn glob_exact_path_with_forward_slashes() {
        assert!(matches_filter(
            XML_DEEP,
            Some("Data/Libs/Subsumption/Missions/mission.xml"),
            None,
        ));
    }

    #[test]
    fn glob_prefix_doublestar_backslash() {
        assert!(matches_filter(XML_DEEP, Some(r"Data\Libs\**"), None));
        assert!(!matches_filter(DDS_DEEP, Some(r"Data\Libs\**"), None));
    }

    #[test]
    fn glob_prefix_doublestar_forward_slash() {
        assert!(matches_filter(XML_DEEP, Some("Data/Libs/**"), None));
        assert!(!matches_filter(DDS_DEEP, Some("Data/Libs/**"), None));
    }

    #[test]
    fn glob_partial_directory_wildcard() {
        // Match all files under any ships subdirectory.
        assert!(matches_filter(DDS_DEEP, Some("**/ships/**"), None));
        assert!(matches_filter(CGF_DEEP, Some("**/ships/**"), None));
        assert!(!matches_filter(XML_DEEP, Some("**/ships/**"), None));
    }

    // -----------------------------------------------------------------------
    // matches_filter — regex mode
    // -----------------------------------------------------------------------

    #[test]
    fn regex_xml_suffix() {
        let re = regex::Regex::new(r"\.xml$").unwrap();
        assert!(matches_filter(XML_DEEP, None, Some(&re)));
        assert!(matches_filter(XML_SHALLOW, None, Some(&re)));
        assert!(!matches_filter(DDS_DEEP, None, Some(&re)));
    }

    #[test]
    fn regex_dds_suffix_excludes_siblings() {
        let re = regex::Regex::new(r"\.dds$").unwrap();
        assert!(matches_filter(DDS_DEEP, None, Some(&re)));
        assert!(!matches_filter(DDS_SIBLING, None, Some(&re)));
    }

    // -----------------------------------------------------------------------
    // matches_filter — no filter
    // -----------------------------------------------------------------------

    #[test]
    fn no_filter_matches_everything() {
        assert!(matches_filter(XML_DEEP, None, None));
        assert!(matches_filter(DDS_DEEP, None, None));
        assert!(matches_filter(CGF_DEEP, None, None));
    }
}
