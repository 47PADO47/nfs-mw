//! The error of the fixed-layout parsers.

/// A buffer did not have the size the layout needs.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{what} needs {need} bytes but the buffer has {got}")]
pub struct LayoutError {
    /// What was being read.
    pub what: &'static str,
    /// The size the layout needs.
    pub need: usize,
    /// The size that was given.
    pub got: usize,
}

impl LayoutError {
    /// Checks that `buf` holds at least `need` bytes.
    pub(crate) fn check(what: &'static str, buf: &[u8], need: usize) -> Result<(), LayoutError> {
        if buf.len() < need {
            return Err(LayoutError { what, need, got: buf.len() });
        }
        Ok(())
    }
}
