mod canonical;

pub mod compare;
pub mod error;
pub mod filter;
pub mod inventory;
pub mod page;
pub mod report;

pub use compare::{compare_reports, DiffItem, DiffSide, DiffSummary};
pub use error::{DiffError, Result};
pub use filter::{filter_diff_items, DiffFilter};
pub use inventory::{
    generate_inventory_from_p4k, generate_inventory_from_p4k_with_progress, InventoryOptions,
    ProgressEvent, ProgressPhase,
};
pub use page::{compare_report_page, DiffPage};
pub use report::{
    ArchiveEntryInventory, BuildManifestInfo, DataCoreInventory, DataCoreRecordInventory,
    DataCoreStatus, DiffReport, HashAlgorithms, InventoryMode, InventoryReport, SourceFileInfo,
    SourceInfo, Tier,
};
