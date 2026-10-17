//! The size-prefix rule: a reader may read a field only if the table's `size` covers it.

/// True when a table whose header says it holds `table_size` bytes also holds the field that ends at byte
/// `field_end` (the field's offset plus its size). Check this before reading any field a newer minor added.
pub const fn has_field(table_size: u32, field_end: usize) -> bool {
    table_size as usize >= field_end
}
