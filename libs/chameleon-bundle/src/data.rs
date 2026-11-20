use std::borrow::Cow;
use std::sync::Arc;

/// Where a bundle's bytes come from when they are not all in memory: anything that can read a range of the file. This crate never
/// opens files, so the caller supplies the reader (a file it opened, a view of one) and the bundle asks it only for the table of
/// resources and for the blocks of the resources that are read. A bundle of 100 MB of which a model needs a few hundred kilobytes
/// costs those few hundred kilobytes.
pub trait Ranges: std::fmt::Debug + Send + Sync {
    /// The size of the whole file in bytes.
    fn size(&self) -> u64;
    /// `count` bytes from `start`. Fewer than `count` bytes (the file ends first) or an error is a truncated bundle.
    fn read(&self, start: u64, count: usize) -> std::io::Result<Vec<u8>>;
}

/// The bytes of a bundle: all in memory, or read on demand.
#[derive(Debug, Clone)]
pub(crate) enum Data<'a> {
    Memory(&'a [u8]),
    Ranged(Arc<dyn Ranges>),
}

impl Data<'_> {
    /// The size of the file.
    pub(crate) fn len(&self) -> usize {
        match self {
            Data::Memory(bytes) => bytes.len(),
            Data::Ranged(source) => usize::try_from(source.size()).unwrap_or(usize::MAX),
        }
    }

    /// The bytes `start..end`, `None` when they are not all in the file or cannot be read.
    pub(crate) fn slice(&self, start: usize, end: usize) -> Option<Cow<'_, [u8]>> {
        match self {
            Data::Memory(bytes) => bytes.get(start..end).map(Cow::Borrowed),
            Data::Ranged(source) => {
                if start > end || end > self.len() {
                    return None;
                }
                let bytes = source.read(start as u64, end - start).ok()?;
                (bytes.len() == end - start).then_some(Cow::Owned(bytes))
            }
        }
    }

    /// The bytes from `start` to the end of the file.
    pub(crate) fn tail(&self, start: usize) -> Option<Cow<'_, [u8]>> {
        self.slice(start, self.len())
    }
}
