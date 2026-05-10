use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use clap::{Args, ValueEnum};
use rayon::prelude::*;
use starbreaker_datacore::database::Database;
use starbreaker_datacore::enums::{ConversionType, DataType};
use starbreaker_p4k::{MappedP4k, P4kEntry};

use crate::common::load_p4k;
use crate::error::{CliError, Result};

#[derive(Clone, ValueEnum)]
pub enum DiffFormat {
    Json,
    Xml,
}

impl DiffFormat {
    fn extension(&self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Xml => "xml",
        }
    }
}

/// Generate diffable reports from a Star Citizen install
#[derive(Args)]
pub struct DiffCommand {
    /// Path to the game channel folder containing Data.p4k
    #[arg(short, long, env = "GAME_FOLDER")]
    game: Option<PathBuf>,
    /// Path to Data.p4k. Overrides --game/Data.p4k
    #[arg(long, env = "SC_DATA_P4K")]
    p4k: Option<PathBuf>,
    /// Output directory for the report tree
    #[arg(short, long, env = "OUTPUT_FOLDER")]
    output: PathBuf,
    /// Keep existing generated files in the output directory
    #[arg(short, long, env = "KEEP_OLD")]
    keep: bool,
    /// Output format for text reports
    #[arg(short, long, value_enum, default_value = "xml", env = "TEXT_FORMAT")]
    format: DiffFormat,
}

pub fn report_root(p4k_path: Option<PathBuf>, game: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(path) = p4k_path {
        return Ok(path);
    }
    if let Some(game) = game {
        return Ok(game.join("Data.p4k"));
    }
    Ok(load_p4k(None)?.path().to_path_buf())
}

impl DiffCommand {
    pub fn run(self) -> Result<()> {
        let started = std::time::Instant::now();
        let p4k_path = report_root(self.p4k, self.game.clone())?;
        let game_dir = self
            .game
            .or_else(|| p4k_path.parent().map(Path::to_path_buf));

        if !self.keep {
            clean_output(&self.output, matches!(self.format, DiffFormat::Json))?;
        }

        let p4k = MappedP4k::open(&p4k_path)?;
        eprintln!("P4k loaded: {}", p4k.path().display());

        let mut sw = std::time::Instant::now();
        dump_p4k_listing(&p4k, &self.output.join("P4k"), &self.format)?;
        eprintln!("P4k listing dumped in {:?}", sw.elapsed());

        sw = std::time::Instant::now();
        extract_report_contents(&p4k, &self.output.join("P4kContents"))?;
        eprintln!("P4k contents extracted in {:?}", sw.elapsed());

        sw = std::time::Instant::now();
        let dcb_bytes = p4k
            .read_file("Data\\Game2.dcb")
            .or_else(|_| p4k.read_file("Data\\Game.dcb"))?;
        let db = Database::from_bytes(&dcb_bytes)?;
        export_datacore_records(&db, &self.output.join("DataCore"), &self.format)?;
        export_datacore_types(&db, &self.output.join("DataCoreTypes"), &self.format)?;
        export_datacore_enums(&db, &self.output.join("DataCoreEnums"), &self.format)?;
        eprintln!("DataCore reports exported in {:?}", sw.elapsed());

        if let Some(game_dir) = game_dir {
            copy_build_manifest(&game_dir, &self.output)?;
        }

        eprintln!("Done in {:?}", started.elapsed());
        Ok(())
    }
}

fn clean_output(output: &Path, json: bool) -> Result<()> {
    for dir in [
        "DataCore",
        "DataCoreTypes",
        "DataCoreEnums",
        "P4k",
        "P4kContents",
        "Protobuf",
    ] {
        let path = output.join(dir);
        if path.exists() {
            std::fs::remove_dir_all(path)?;
        }
    }

    let manifest = output.join("build_manifest.json");
    if manifest.exists() {
        std::fs::remove_file(manifest)?;
    }

    let root_listing = output.join("P4k").join(format!("Data.{}", if json { "json" } else { "xml" }));
    if root_listing.exists() {
        std::fs::remove_file(root_listing)?;
    }
    Ok(())
}

fn dump_p4k_listing(p4k: &MappedP4k, output: &Path, format: &DiffFormat) -> Result<()> {
    let mut dirs: BTreeMap<String, Vec<&P4kEntry>> = BTreeMap::new();

    for entry in p4k.entries().iter().filter(|e| e.uncompressed_size > 0) {
        let (dir, _) = split_entry_path(&entry.name);
        dirs.entry(dir).or_default().push(entry);
    }

    dirs.par_iter()
        .try_for_each(|(dir, files)| write_p4k_dir_report(output, dir, files, format))?;
    Ok(())
}

fn write_p4k_dir_report(
    output: &Path,
    dir: &str,
    files: &[&P4kEntry],
    format: &DiffFormat,
) -> Result<()> {
    let name = dir.rsplit('\\').next().unwrap_or("Data");
    let relative_dir = dir.replace('\\', "/");
    let out_dir = output.join(relative_dir);
    std::fs::create_dir_all(&out_dir)?;
    let out_path = out_dir.join(format!("{name}.{}", format.extension()));

    match format {
        DiffFormat::Json => write_p4k_dir_json(&out_path, name, files),
        DiffFormat::Xml => write_p4k_dir_xml(&out_path, name, files),
    }
}

fn write_p4k_dir_json(path: &Path, name: &str, files: &[&P4kEntry]) -> Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    let files = files.iter().map(|entry| {
        let (_, file_name) = split_entry_path(&entry.name);
        serde_json::json!({
            "Name": file_name,
            "CRC32": format!("0x{:08X}", entry.crc32),
            "Size": entry.uncompressed_size.to_string(),
            "CompressionType": entry.compression_method.to_string(),
            "Encrypted": entry.is_encrypted.to_string(),
        })
    });
    let report = serde_json::json!({
        "Name": name,
        "Files": files.collect::<Vec<_>>(),
    });
    serde_json::to_writer_pretty(&mut writer, &report)?;
    writer.write_all(b"\n")?;
    Ok(())
}

fn write_p4k_dir_xml(path: &Path, name: &str, files: &[&P4kEntry]) -> Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    writeln!(writer, "<?xml version=\"1.0\" encoding=\"utf-8\"?>")?;
    writeln!(writer, "<Directory Name=\"{}\">", xml_escape(name))?;
    for entry in files {
        let (_, file_name) = split_entry_path(&entry.name);
        writeln!(
            writer,
            "  <File Name=\"{}\" CRC32=\"0x{:08X}\" Size=\"{}\" CompressionType=\"{}\" Encrypted=\"{}\" />",
            xml_escape(file_name),
            entry.crc32,
            entry.uncompressed_size,
            entry.compression_method,
            entry.is_encrypted
        )?;
    }
    writeln!(writer, "</Directory>")?;
    Ok(())
}

fn extract_report_contents(p4k: &MappedP4k, output: &Path) -> Result<()> {
    let patterns = ["english\\global.ini", "TagDatabase.TagDatabase.xml"];
    let entries: Vec<&P4kEntry> = p4k
        .entries()
        .iter()
        .filter(|entry| {
            let lower = entry.name.to_ascii_lowercase();
            patterns.iter().any(|suffix| lower.ends_with(suffix))
        })
        .collect();

    entries.par_iter().try_for_each(|entry| {
        let data = p4k.read(entry)?;
        let out_path = output.join(entry.name.replace('\\', "/"));
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(out_path, data)?;
        Ok::<_, CliError>(())
    })?;
    Ok(())
}

fn export_datacore_records(db: &Database<'_>, output: &Path, format: &DiffFormat) -> Result<()> {
    let ext = format.extension();
    let records: Vec<_> = db
        .records()
        .iter()
        .filter(|record| db.is_main_record(record))
        .collect();

    records.par_iter().try_for_each(|record| {
        let file_name = db.resolve_string(record.file_name_offset);
        let out_path = output.join(change_extension(file_name, ext));
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = match format {
            DiffFormat::Json => starbreaker_datacore::export::to_json(db, record)?,
            DiffFormat::Xml => starbreaker_datacore::export::to_xml(db, record)?,
        };
        std::fs::write(out_path, data)?;
        Ok::<_, CliError>(())
    })?;
    Ok(())
}

fn export_datacore_types(db: &Database<'_>, output: &Path, format: &DiffFormat) -> Result<()> {
    let children = struct_children(db);
    let roots: Vec<usize> = db
        .struct_defs()
        .iter()
        .enumerate()
        .filter_map(|(index, def)| (def.parent_type_index == -1).then_some(index))
        .collect();

    for root in roots {
        export_type_node(db, output, &children, root, format)?;
    }
    Ok(())
}

fn export_type_node(
    db: &Database<'_>,
    current_dir: &Path,
    children: &[Vec<usize>],
    index: usize,
    format: &DiffFormat,
) -> Result<()> {
    let def = &db.struct_defs()[index];
    let name = db.resolve_string2(def.name_offset);
    let node_dir = if children[index].is_empty() {
        current_dir.to_path_buf()
    } else {
        current_dir.join(name)
    };
    std::fs::create_dir_all(&node_dir)?;
    let out_path = node_dir.join(format!("{name}.{}", format.extension()));
    write_type_report(db, index, &out_path, format)?;

    for &child in &children[index] {
        export_type_node(db, &node_dir, children, child, format)?;
    }
    Ok(())
}

fn write_type_report(db: &Database<'_>, index: usize, path: &Path, format: &DiffFormat) -> Result<()> {
    match format {
        DiffFormat::Json => {
            let def = &db.struct_defs()[index];
            let mut report = serde_json::Map::new();
            report.insert(
                "Name".to_string(),
                serde_json::Value::String(db.resolve_string2(def.name_offset).to_string()),
            );
            if def.parent_type_index != -1 {
                let parent = &db.struct_defs()[def.parent_type_index as usize];
                report.insert(
                    "Parent".to_string(),
                    serde_json::Value::String(db.resolve_string2(parent.name_offset).to_string()),
                );
            }
            let properties = properties_for_type(db, index)
                .into_iter()
                .map(|(name, type_name)| serde_json::json!({ "Name": name, "Type": type_name }))
                .collect::<Vec<_>>();
            report.insert("Properties".to_string(), serde_json::Value::Array(properties));
            let file = File::create(path)?;
            let mut writer = BufWriter::new(file);
            serde_json::to_writer_pretty(&mut writer, &serde_json::Value::Object(report))?;
            writer.write_all(b"\n")?;
        }
        DiffFormat::Xml => {
            let def = &db.struct_defs()[index];
            let file = File::create(path)?;
            let mut writer = BufWriter::new(file);
            writeln!(writer, "<?xml version=\"1.0\" encoding=\"utf-8\"?>")?;
            write!(writer, "<Struct Name=\"{}\"", xml_escape(db.resolve_string2(def.name_offset)))?;
            if def.parent_type_index != -1 {
                let parent = &db.struct_defs()[def.parent_type_index as usize];
                write!(writer, " Parent=\"{}\"", xml_escape(db.resolve_string2(parent.name_offset)))?;
            }
            writeln!(writer, ">")?;
            for (name, type_name) in properties_for_type(db, index) {
                writeln!(
                    writer,
                    "  <Property Name=\"{}\" Type=\"{}\" />",
                    xml_escape(&name),
                    xml_escape(&type_name)
                )?;
            }
            writeln!(writer, "</Struct>")?;
        }
    }
    Ok(())
}

fn export_datacore_enums(db: &Database<'_>, output: &Path, format: &DiffFormat) -> Result<()> {
    std::fs::create_dir_all(output)?;
    (0..db.enum_defs().len()).into_par_iter().try_for_each(|index| {
        let enum_def = &db.enum_defs()[index];
        let name = db.resolve_string2(enum_def.name_offset);
        let values = db
            .enum_options(index as i32)
            .iter()
            .map(|value| db.resolve_string2(*value))
            .collect::<Vec<_>>();
        let path = output.join(format!("{name}.{}", format.extension()));
        match format {
            DiffFormat::Json => {
                let file = File::create(path)?;
                let mut writer = BufWriter::new(file);
                let report = serde_json::json!({ "Name": name, "Values": values });
                serde_json::to_writer_pretty(&mut writer, &report)?;
                writer.write_all(b"\n")?;
            }
            DiffFormat::Xml => {
                let file = File::create(path)?;
                let mut writer = BufWriter::new(file);
                writeln!(writer, "<?xml version=\"1.0\" encoding=\"utf-8\"?>")?;
                writeln!(writer, "<Enum Name=\"{}\">", xml_escape(name))?;
                for value in values {
                    writeln!(writer, "  <Value>{}</Value>", xml_escape(value))?;
                }
                writeln!(writer, "</Enum>")?;
            }
        }
        Ok::<_, CliError>(())
    })?;
    Ok(())
}

fn struct_children(db: &Database<'_>) -> Vec<Vec<usize>> {
    let mut children = vec![Vec::new(); db.struct_defs().len()];
    for (index, def) in db.struct_defs().iter().enumerate() {
        if def.parent_type_index != -1 {
            children[def.parent_type_index as usize].push(index);
        }
    }
    for child_list in &mut children {
        child_list.sort_by_key(|&index| db.resolve_string2(db.struct_defs()[index].name_offset));
    }
    children
}

fn properties_for_type(db: &Database<'_>, index: usize) -> Vec<(String, String)> {
    db.all_properties(index as i32)
        .into_iter()
        .map(|prop| {
            let name = db.resolve_string2(prop.name_offset).to_string();
            let type_name = property_type_name(db, prop.data_type, prop.conversion_type, prop.struct_index);
            (name, type_name)
        })
        .collect()
}

fn property_type_name(
    db: &Database<'_>,
    data_type: u16,
    conversion_type: u16,
    struct_index: u16,
) -> String {
    let scalar = match DataType::try_from(data_type) {
        Ok(DataType::Boolean) => "bool".to_string(),
        Ok(DataType::Byte) => "byte".to_string(),
        Ok(DataType::SByte) => "sbyte".to_string(),
        Ok(DataType::Int16) => "short".to_string(),
        Ok(DataType::UInt16) => "ushort".to_string(),
        Ok(DataType::Int32) => "int".to_string(),
        Ok(DataType::UInt32) => "uint".to_string(),
        Ok(DataType::Int64) => "long".to_string(),
        Ok(DataType::UInt64) => "ulong".to_string(),
        Ok(DataType::Single) => "float".to_string(),
        Ok(DataType::Double) => "double".to_string(),
        Ok(DataType::Guid) => "CigGuid".to_string(),
        Ok(DataType::Locale | DataType::String) => "string".to_string(),
        Ok(DataType::EnumChoice) => db
            .enum_defs()
            .get(struct_index as usize)
            .map(|def| db.resolve_string2(def.name_offset).to_string())
            .unwrap_or_else(|| format!("Enum#{struct_index}")),
        Ok(DataType::Reference | DataType::StrongPointer | DataType::WeakPointer | DataType::Class) => db
            .struct_defs()
            .get(struct_index as usize)
            .map(|def| db.resolve_string2(def.name_offset).to_string())
            .unwrap_or_else(|| format!("Struct#{struct_index}")),
        Err(_) => format!("Unknown({data_type:#06x})"),
    };

    match ConversionType::try_from(conversion_type) {
        Ok(ConversionType::Attribute) => scalar,
        Ok(_) => format!("{scalar}[]"),
        Err(_) => format!("{scalar}[conversion:{conversion_type:#06x}]"),
    }
}

fn split_entry_path(name: &str) -> (String, &str) {
    match name.rsplit_once('\\') {
        Some((dir, file)) => (dir.to_string(), file),
        None => ("Data".to_string(), name),
    }
}

fn change_extension(path: &str, ext: &str) -> String {
    match path.rfind('.') {
        Some(dot) => format!("{}.{ext}", &path[..dot]),
        None => format!("{path}.{ext}"),
    }
}

fn copy_build_manifest(game_dir: &Path, output: &Path) -> Result<()> {
    let source = game_dir.join("build_manifest.id");
    if !source.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(output)?;
    std::fs::copy(source, output.join("build_manifest.json"))?;
    Ok(())
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
