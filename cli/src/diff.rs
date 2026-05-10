use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, ValueEnum};
use starbreaker_diff::compare::{DiffItem, DiffStatus};
use starbreaker_diff::report::{
    read_inventory_report, write_diff_report, write_inventory_report, InventoryReport, Tier,
    INVENTORY_EXTENSION,
};
use starbreaker_diff::{compare_reports, generate_inventory_from_p4k_with_progress, DiffFilter};

use crate::error::Result;

#[derive(Subcommand)]
pub enum DiffCommand {
    /// Generate a reusable P4k/DataCore inventory report.
    Inventory(InventoryArgs),
    /// Compare two P4k files and/or inventory reports.
    Compare(CompareArgs),
}

impl DiffCommand {
    pub fn run(self) -> Result<()> {
        match self {
            Self::Inventory(args) => inventory(args),
            Self::Compare(args) => compare(args),
        }
    }
}

#[derive(Parser)]
pub struct InventoryArgs {
    /// Data.p4k path.
    source: PathBuf,
    /// Output *.starbreaker-inventory.json path.
    #[arg(short, long)]
    output: PathBuf,
    /// Skip DataCore record inventory and compare only P4k file entries.
    #[arg(long)]
    skip_datacore: bool,
    /// Optional display label stored in the report.
    #[arg(long)]
    label: Option<String>,
}

#[derive(Parser)]
pub struct CompareArgs {
    /// Old source: Data.p4k or *.starbreaker-inventory.json.
    old: PathBuf,
    /// New source: Data.p4k or *.starbreaker-inventory.json.
    new: PathBuf,
    /// Optional output *.starbreaker-diff.json path.
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// Output format when not writing a diff file.
    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    format: OutputFormat,
    /// Filter by tier.
    #[arg(long, value_enum, default_value_t = TierArg::All)]
    tier: TierArg,
    /// Filter by statuses. Comma-separated values are accepted.
    #[arg(long, value_delimiter = ',')]
    status: Vec<StatusArg>,
    /// Plain-text search across paths, names, types, extensions, and GUIDs.
    #[arg(long)]
    search: Option<String>,
    /// Filter archive files by extension, such as .dds or xml.
    #[arg(long, value_delimiter = ',')]
    extension: Vec<String>,
    /// Filter DataCore records by record type.
    #[arg(long = "record-type", value_delimiter = ',')]
    record_type: Vec<String>,
    /// Filter archive/DataCore paths by prefix.
    #[arg(long = "path-prefix", value_delimiter = ',')]
    path_prefix: Vec<String>,
    /// Include unchanged rows in the diff output.
    #[arg(long)]
    include_unchanged: bool,
    /// Skip DataCore if a compare input is a raw P4k.
    #[arg(long)]
    skip_datacore: bool,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Table,
    Json,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum TierArg {
    All,
    P4k,
    Datacore,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum StatusArg {
    Added,
    Removed,
    Modified,
    Metadata,
    Unchanged,
}

fn inventory(args: InventoryArgs) -> Result<()> {
    let options = starbreaker_diff::InventoryOptions {
        skip_datacore: args.skip_datacore,
        label: args.label,
        ..Default::default()
    };
    let mut progress_output = CliProgress::new();
    let mut progress = |event: starbreaker_diff::ProgressEvent| progress_output.report(event);
    let report = generate_inventory_from_p4k_with_progress(
        &args.source,
        &options,
        Some(&mut progress),
        None,
    )?;
    drop(progress);
    progress_output.finish();
    write_inventory_report(&args.output, &report)?;
    eprintln!(
        "Wrote inventory: {} ({}, {} P4k entries, {} DataCore records)",
        args.output.display(),
        report.source.label,
        report.archive.len(),
        report.datacore.status.records().len()
    );
    Ok(())
}

fn compare(args: CompareArgs) -> Result<()> {
    let options = starbreaker_diff::InventoryOptions {
        skip_datacore: args.skip_datacore,
        ..Default::default()
    };
    let old = load_source(&args.old, &options)?;
    let new = load_source(&args.new, &options)?;
    let mut diff = compare_reports(&old, &new, args.include_unchanged);
    let filter = build_filter(&args);
    if filter_has_constraints(&filter) {
        let filtered = starbreaker_diff::filter_diff_items(&diff.items, &filter);
        diff.items = filtered.into_iter().cloned().collect();
        diff.summary = summary_for_items(&diff.items);
    }

    if let Some(output) = args.output {
        write_diff_report(&output, &diff)?;
        eprintln!("Wrote diff: {}", output.display());
        return Ok(());
    }

    match args.format {
        OutputFormat::Table => print_table(&diff.items),
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&diff)?),
    }
    Ok(())
}

fn load_source(path: &Path, options: &starbreaker_diff::InventoryOptions) -> Result<InventoryReport> {
    if is_inventory_report(path) {
        return Ok(read_inventory_report(path)?);
    }
    let mut progress_output = CliProgress::new();
    let mut progress = |event: starbreaker_diff::ProgressEvent| progress_output.report(event);
    let report = generate_inventory_from_p4k_with_progress(
        path,
        options,
        Some(&mut progress),
        None,
    )?;
    drop(progress);
    progress_output.finish();
    Ok(report)
}

fn is_inventory_report(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(INVENTORY_EXTENSION))
}

fn build_filter(args: &CompareArgs) -> DiffFilter {
    let tiers = match args.tier {
        TierArg::All => Vec::new(),
        TierArg::P4k => vec![Tier::P4k],
        TierArg::Datacore => vec![Tier::DataCore],
    };
    let statuses = args.status.iter().map(|status| match status {
        StatusArg::Added => DiffStatus::Added,
        StatusArg::Removed => DiffStatus::Removed,
        StatusArg::Modified => DiffStatus::Modified,
        StatusArg::Metadata => DiffStatus::MetadataChanged,
        StatusArg::Unchanged => DiffStatus::Unchanged,
    }).collect();
    DiffFilter {
        search: args.search.clone(),
        tiers,
        statuses,
        extensions: args.extension.clone(),
        record_types: args.record_type.clone(),
        path_prefixes: args.path_prefix.clone(),
        include_unchanged: args.include_unchanged,
    }
}

fn filter_has_constraints(filter: &DiffFilter) -> bool {
    filter.search.as_ref().is_some_and(|s| !s.trim().is_empty())
        || !filter.tiers.is_empty()
        || !filter.statuses.is_empty()
        || !filter.extensions.is_empty()
        || !filter.record_types.is_empty()
        || !filter.path_prefixes.is_empty()
}

fn summary_for_items(items: &[DiffItem]) -> starbreaker_diff::DiffSummary {
    let mut summary = starbreaker_diff::DiffSummary::default();
    for item in items {
        match item.status {
            DiffStatus::Added => summary.added += 1,
            DiffStatus::Removed => summary.removed += 1,
            DiffStatus::Modified => summary.modified += 1,
            DiffStatus::MetadataChanged => summary.metadata_changed += 1,
            DiffStatus::Unchanged => summary.unchanged += 1,
        }
        match item.tier {
            Tier::P4k => summary.p4k_items += 1,
            Tier::DataCore => summary.datacore_items += 1,
        }
    }
    summary
}

fn print_table(items: &[DiffItem]) {
    println!("{:<16} {:<10} {:<32} Reasons", "Status", "Tier", "Item");
    println!("{:-<16} {:-<10} {:-<32} {:-<24}", "", "", "", "");
    for item in items {
        println!(
            "{:<16} {:<10} {:<32} {}",
            status_label(item.status),
            tier_label(item.tier),
            truncate(&item.display, 32),
            item.reasons.join(",")
        );
    }
}

fn status_label(status: DiffStatus) -> &'static str {
    match status {
        DiffStatus::Added => "added",
        DiffStatus::Removed => "removed",
        DiffStatus::Modified => "modified",
        DiffStatus::MetadataChanged => "metadata",
        DiffStatus::Unchanged => "unchanged",
    }
}

fn tier_label(tier: Tier) -> &'static str {
    match tier {
        Tier::P4k => "p4k",
        Tier::DataCore => "datacore",
    }
}

fn truncate(value: &str, width: usize) -> String {
    if value.chars().count() <= width {
        return value.to_string();
    }
    let mut out: String = value.chars().take(width.saturating_sub(3)).collect();
    out.push_str("...");
    out
}

struct CliProgress {
    dynamic: bool,
    line_active: bool,
}

impl CliProgress {
    fn new() -> Self {
        Self {
            dynamic: io::stderr().is_terminal(),
            line_active: false,
        }
    }

    fn report(&mut self, event: starbreaker_diff::ProgressEvent) {
        if self.dynamic && event.phase == starbreaker_diff::ProgressPhase::HashingDataCoreRecords {
            self.write_dynamic(&format_progress(&event));
            return;
        }

        self.finish();
        eprintln!("{}", format_progress(&event));
    }

    fn write_dynamic(&mut self, text: &str) {
        let mut stderr = io::stderr().lock();
        let _ = write!(stderr, "\r\x1b[2K{text}");
        let _ = stderr.flush();
        self.line_active = true;
    }

    fn finish(&mut self) {
        if self.line_active {
            eprintln!();
            self.line_active = false;
        }
    }
}

fn format_progress(event: &starbreaker_diff::ProgressEvent) -> String {
    match (event.current, event.total) {
        (Some(current), Some(total)) => format!("{}: {current}/{total}", event.message),
        _ => event.message.clone(),
    }
}
