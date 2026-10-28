//! The errors of the crate.

use thiserror::Error;

/// Everything that can go wrong building a skeleton, a graph or reading a pack.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    /// A bone names a parent that does not come before it.
    #[error("bone {bone} has parent {parent}, which does not precede it")]
    BadParent { bone: usize, parent: usize },
    /// Two bones share a name.
    #[error("two bones are named {0}")]
    DuplicateName(String),
    /// A pose or a table does not have one entry per bone.
    #[error("expected {expected} entries, got {got}")]
    CountMismatch { expected: usize, got: usize },
    /// A track, socket or mesh names a bone that does not exist.
    #[error("bone index {0} is out of range")]
    BadBone(usize),
    /// The pack ended before a field was complete.
    #[error("the pack is cut short")]
    UnexpectedEnd,
    /// The pack does not start with the magic or has an unknown version.
    #[error("not a skeleton pack: {0}")]
    BadHeader(&'static str),
    /// A string in the pack is not UTF-8.
    #[error("a string in the pack is not UTF-8")]
    BadString,
    /// A field holds a value the format does not allow.
    #[error("bad value in the pack: {0}")]
    BadValue(&'static str),
}

/// Result alias of the crate.
pub type Result<T> = std::result::Result<T, Error>;
