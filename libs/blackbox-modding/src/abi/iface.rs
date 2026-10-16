//! The header every interface table starts with.

/// The header of an interface table. An interface is a `repr(C)` struct that starts with this header and then
/// holds its function pointers. Its `size` says how many bytes the host filled in, and `major` and `minor` are
/// the interface's own version (for example `bb.map 1.2`), separate from the ABI version.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BbIface {
    /// Bytes the table holds.
    pub size: u32,
    /// Interface major version.
    pub major: u16,
    /// Interface minor version.
    pub minor: u16,
}
