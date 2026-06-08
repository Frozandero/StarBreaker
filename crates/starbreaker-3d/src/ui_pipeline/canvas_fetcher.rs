//! DataCore canvas fetcher with an O(1) name→GUID index (B2b).
//!
//! [`DatacoreCanvasFetcher`] precomputes a `HashMap<lowercase_key, CigGuid>` over
//! all UI-support record families once at construction time, turning every
//! `fetch_canvas_by_name` call from O(records) to O(1).

use std::collections::HashMap;

use log::warn;
use starbreaker_datacore::Database;
use starbreaker_datacore::starbreaker_common::CigGuid;
use starbreaker_ui::{UiError, pipeline::CanvasFetcher};

use super::{datacore_ui_lookup_type_names, parse_guid};

// ── Name index ────────────────────────────────────────────────────────────────

/// Precomputed, case-insensitive name→GUID index over all DataCore UI record families.
struct CanvasNameIndex {
    index: HashMap<String, CigGuid>,
}

impl CanvasNameIndex {
    fn build(db: &Database<'_>) -> Self {
        let mut index: HashMap<String, CigGuid> = HashMap::new();
        for type_name in datacore_ui_lookup_type_names() {
            for record in db.records_by_type_name(type_name) {
                let full_name = db.resolve_string2(record.name_offset);
                let stem = full_name.rsplit('.').next().unwrap_or(full_name);
                let stem_lower = stem.to_ascii_lowercase();
                let full_lower = full_name.to_ascii_lowercase();
                // Iterate types in declared priority order; first type+record wins.
                for key in std::iter::once(&stem_lower).chain(
                    (full_lower != stem_lower).then_some(&full_lower),
                ) {
                    match index.entry(key.clone()) {
                        std::collections::hash_map::Entry::Vacant(e) => {
                            e.insert(record.id);
                        }
                        std::collections::hash_map::Entry::Occupied(e) if *e.get() != record.id => {
                            warn!(
                                "ui_pipeline: duplicate UI record name '{}' in {}; first wins",
                                key, type_name
                            );
                        }
                        _ => {}
                    }
                }
            }
        }
        Self { index }
    }

    fn lookup(&self, name: &str) -> Option<CigGuid> {
        self.index.get(&name.to_ascii_lowercase()).copied()
    }
}

// ── Fetcher ───────────────────────────────────────────────────────────────────

/// Canvas fetcher backed by a DataCore [`Database`] with an O(1) name index.
pub(super) struct DatacoreCanvasFetcher<'a> {
    db: &'a Database<'a>,
    name_index: CanvasNameIndex,
}

impl<'a> DatacoreCanvasFetcher<'a> {
    pub(super) fn new(db: &'a Database<'a>) -> Self {
        Self { db, name_index: CanvasNameIndex::build(db) }
    }

    fn fetch_by_guid(&self, cig_guid: CigGuid, lookup_key: &str) -> Result<serde_json::Value, UiError> {
        let record = self.db.record_by_id(&cig_guid).ok_or_else(|| UiError::FetchFailed {
            guid: lookup_key.to_string(),
            source: format!("record not found in DataCore for GUID {lookup_key}").into(),
        })?;
        let bytes = starbreaker_datacore::export::to_json_compact(self.db, record)
            .map_err(|e| UiError::FetchFailed {
                guid: lookup_key.to_string(),
                source: Box::new(e),
            })?;
        serde_json::from_slice(&bytes).map_err(|e| UiError::FetchFailed {
            guid: lookup_key.to_string(),
            source: Box::new(e),
        })
    }
}

impl<'a> CanvasFetcher for DatacoreCanvasFetcher<'a> {
    fn fetch_canvas_json(&self, guid: &str) -> Result<serde_json::Value, UiError> {
        let cig_guid = parse_guid(guid).ok_or_else(|| UiError::FetchFailed {
            guid: guid.to_string(),
            source: "invalid GUID format".into(),
        })?;
        self.fetch_by_guid(cig_guid, guid)
    }

    fn fetch_canvas_by_name(&self, record_name: &str) -> Result<serde_json::Value, UiError> {
        let cig_guid = self.name_index.lookup(record_name).ok_or_else(|| UiError::FetchFailed {
            guid: record_name.to_string(),
            source: format!("no UI-support record found by name: {record_name}").into(),
        })?;
        self.fetch_by_guid(cig_guid, record_name)
    }
}
