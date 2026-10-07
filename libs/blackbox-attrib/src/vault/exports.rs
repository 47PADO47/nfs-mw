//! `ExpN`: the export table. `u32 count`, then one entry per exported object (a class, a
//! collection or the database header); the entry layout comes from [`crate::layout`].

use crate::bytes::u32_at;
use crate::layout::ExportLayout;

/// One exported object of a vault.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Export {
    /// Export id: for a collection, the hash of `"<class>/<collection>"`; for a class, its key.
    pub id: u32,
    /// Type key, such as `vlt_hash("Attrib::CollectionLoadData")`.
    pub kind: u32,
    /// Offset of the record in the `.vlt` blob.
    pub offset: usize,
    pub size: usize,
}

/// Reads the entries. [`crate::layout::detect`] has already checked that they fit.
pub fn read_exports(table: &[u8], layout: &ExportLayout) -> Vec<Export> {
    let count = u32_at(table, 0).unwrap_or(0) as usize;
    (0..count)
        .map(|i| {
            let at = |field: usize| u32_at(table, 4 + i * layout.len + field).unwrap_or(0);
            Export {
                id: at(layout.id),
                kind: at(layout.kind),
                offset: at(layout.offset) as usize,
                size: at(layout.size) as usize,
            }
        })
        .collect()
}
