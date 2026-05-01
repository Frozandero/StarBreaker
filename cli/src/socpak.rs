use std::path::PathBuf;

use clap::Subcommand;
use starbreaker_datacore::database::Database;

use crate::common::{
    ExportOpts, collect_existing_decomposed_assets, load_dcb_bytes, prepare_decomposed_output_root,
    sanitize_export_name, write_decomposed_file,
};
use crate::error::{CliError, Result};

#[derive(Subcommand)]
pub enum SocpakCommand {
    /// Export socpak interior containers to GLB
    Export {
        /// P4k path substring for socpak files (case-insensitive)
        pattern: String,
        /// Output .glb path, or export root for --kind decomposed
        output: Option<PathBuf>,
        /// Path to Data.p4k
        #[arg(long, env = "SC_DATA_P4K")]
        p4k: Option<PathBuf>,
        /// Disable connected-socpak traversal for decomposed exports
        #[arg(long)]
        no_connected: bool,
        #[command(flatten)]
        opts: ExportOpts,
    },
}

impl SocpakCommand {
    pub fn run(self) -> Result<()> {
        match self {
            Self::Export {
                pattern,
                output,
                p4k,
                no_connected,
                opts,
            } => export(pattern, output, p4k, no_connected, opts),
        }
    }
}

fn export(
    pattern: String,
    output: Option<PathBuf>,
    p4k_path: Option<PathBuf>,
    no_connected: bool,
    opts: ExportOpts,
) -> Result<()> {
    let (p4k, dcb_bytes) = load_dcb_bytes(p4k_path.as_deref(), None)?;
    let p4k =
        p4k.ok_or_else(|| CliError::MissingRequirement("P4k required for socpak export".into()))?;
    let db = Database::from_bytes(&dcb_bytes)?;

    let socpak_paths = resolve_socpak_paths(&p4k, &pattern);

    if socpak_paths.is_empty() {
        return Err(CliError::NotFound(format!(
            "no .socpak files matching '{pattern}'"
        )));
    }

    eprintln!("Found {} socpak files", socpak_paths.len());
    let export_opts = starbreaker_3d::ExportOptions::from(&opts);

    match export_opts.kind {
        starbreaker_3d::ExportKind::Bundled => {
            let glb = starbreaker_3d::socpaks_to_glb(&db, &p4k, &socpak_paths, &export_opts)?;

            let output = output.unwrap_or_else(|| PathBuf::from(format!("{pattern}.glb")));
            std::fs::write(&output, &glb).map_err(|e| CliError::IoPath {
                source: e,
                path: output.display().to_string(),
            })?;
            eprintln!("Written {} bytes to {}", glb.len(), output.display());
        }
        starbreaker_3d::ExportKind::Decomposed => {
            let output_root =
                output.unwrap_or_else(|| PathBuf::from(sanitize_export_name(&pattern)));
            if output_root.exists() && output_root.is_file() {
                return Err(CliError::InvalidInput(format!(
                    "decomposed output root '{}' already exists as a file",
                    output_root.display(),
                )));
            }
            let existing_asset_paths = if opts.skip_existing_assets {
                Some(collect_existing_decomposed_assets(&output_root)?)
            } else {
                None
            };
            let result = starbreaker_3d::socpaks_to_decomposed(
                &db,
                &p4k,
                &socpak_paths,
                &export_opts,
                starbreaker_3d::SocpakGraphOptions {
                    connected: !no_connected,
                },
                existing_asset_paths.as_ref(),
            )?;
            let package_name = format!(
                "{}_LOD{}_TEX{}",
                sanitize_export_name(&format!(
                    "socpak {}",
                    socpak_paths
                        .first()
                        .and_then(|path| path.rsplit(&['/', '\\']).next())
                        .and_then(|name| name.strip_suffix(".socpak"))
                        .unwrap_or(&pattern)
                )),
                export_opts.lod_level,
                export_opts.texture_mip
            );
            prepare_decomposed_output_root(&output_root, &package_name)?;
            for file in &result.export.files {
                let output_path = output_root.join(&file.relative_path);
                if let Some(parent) = output_path.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| CliError::IoPath {
                        source: e,
                        path: parent.display().to_string(),
                    })?;
                }
                write_decomposed_file(file, &output_path, opts.skip_existing_assets)?;
            }
            eprintln!(
                "Socpak graph: {} root(s), {} connected socpak(s), {} warning(s)",
                result.root_count,
                result.socpak_count,
                result.warnings.len()
            );
            for warning in &result.warnings {
                eprintln!("warning: {warning}");
            }
            eprintln!(
                "Decomposed export file count: {}",
                result.export.files.len()
            );
            eprintln!("Written to {}", output_root.display());
        }
    }
    Ok(())
}

fn resolve_socpak_paths(p4k: &starbreaker_p4k::MappedP4k, pattern: &str) -> Vec<String> {
    let normalized_pattern = pattern.replace('/', "\\");
    let explicit = if normalized_pattern
        .to_ascii_lowercase()
        .starts_with("data\\")
    {
        normalized_pattern.clone()
    } else {
        format!("Data\\{normalized_pattern}")
    };
    if explicit.to_ascii_lowercase().ends_with(".socpak") {
        if let Some(entry) = p4k.entry_case_insensitive(&explicit) {
            return vec![entry.name.clone()];
        }
    }

    let search_lower = normalized_pattern.to_lowercase();
    p4k.entries()
        .iter()
        .filter(|e| {
            let name = e.name.to_lowercase();
            name.contains(&search_lower) && name.ends_with(".socpak")
        })
        .map(|e| e.name.clone())
        .collect()
}
