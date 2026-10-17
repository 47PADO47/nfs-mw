//! A handle to an interface table the host offered.

use core::ptr::NonNull;

use crate::abi::{BbIface, has_field};

/// A handle to an interface table the host offered through `Ctx::query_interface`. The host keeps the table
/// valid while the mod is loaded. Typed wrappers for each interface are built on top of this type; this module
/// does not implement any interface itself.
#[derive(Debug, Clone, Copy)]
pub struct Interface {
    header: BbIface,
    table: NonNull<BbIface>,
}

impl Interface {
    /// Wrap a table whose header has already been read.
    pub(crate) const fn new(table: NonNull<BbIface>, header: BbIface) -> Self {
        Interface { header, table }
    }

    /// The interface's major version.
    pub const fn major(&self) -> u16 {
        self.header.major
    }

    /// The interface's minor version.
    pub const fn minor(&self) -> u16 {
        self.header.minor
    }

    /// Bytes the host filled in for this table.
    pub const fn size(&self) -> u32 {
        self.header.size
    }

    /// Whether the table holds a field that ends at byte `field_end`. Check it before reading a field a newer
    /// minor added, because an older host has a shorter table.
    pub const fn has_field(&self, field_end: usize) -> bool {
        has_field(self.header.size, field_end)
    }

    /// The table itself, for the typed wrappers of each interface.
    pub const fn as_ptr(&self) -> *const BbIface {
        self.table.as_ptr()
    }
}
