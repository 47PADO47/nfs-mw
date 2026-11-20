//! A bundle of the 2012 install read by ranges: only its table of resources and the blocks of the resources the trees need are
//! read from the file, never the whole of it. The district and track unit bundles that hold the trees are 8 to 97 MB each (350 MB
//! for the nine the species come from); the trees take about 2.5 MB of them (`docs/formats/chameleon-vegetation.md`, "What the
//! conversion costs").

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use chameleon_bundle::{Bundle, Ranges};

/// A file that answers range reads and counts the bytes it was asked for.
#[derive(Debug)]
pub struct FileRanges {
    file: Mutex<File>,
    size: u64,
    taken: AtomicU64,
}

impl FileRanges {
    /// How many bytes have been read from the file so far.
    pub fn bytes_read(&self) -> u64 {
        self.taken.load(Ordering::Relaxed)
    }
}

impl Ranges for FileRanges {
    fn size(&self) -> u64 {
        self.size
    }

    fn read(&self, start: u64, count: usize) -> io::Result<Vec<u8>> {
        let mut file = self.file.lock().map_err(|_| io::Error::other("the file lock is poisoned"))?;
        file.seek(SeekFrom::Start(start))?;
        let mut bytes = vec![0; count];
        file.read_exact(&mut bytes)?;
        self.taken.fetch_add(count as u64, Ordering::Relaxed);
        Ok(bytes)
    }
}

/// The bundle at `path`, read on demand, and the reader that counts what is read from it.
pub fn open(path: &Path) -> Result<(Bundle<'static>, Arc<FileRanges>)> {
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let size = file.metadata().with_context(|| format!("measuring {}", path.display()))?.len();
    let ranges = Arc::new(FileRanges { file: Mutex::new(file), size, taken: AtomicU64::new(0) });
    let bundle =
        Bundle::parse_ranges(ranges.clone()).with_context(|| format!("reading the table of {}", path.display()))?;
    Ok((bundle, ranges))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_is_read_by_range_and_the_bytes_are_counted() {
        let path = std::env::temp_dir().join(format!("nfsmw-trees-ranges-{}", std::process::id()));
        std::fs::write(&path, (0..=255u8).collect::<Vec<_>>()).unwrap();
        let file = File::open(&path).unwrap();
        let ranges = FileRanges { file: Mutex::new(file), size: 256, taken: AtomicU64::new(0) };
        assert_eq!(ranges.size(), 256);
        assert_eq!(ranges.read(250, 6).unwrap(), [250, 251, 252, 253, 254, 255]);
        assert_eq!(ranges.read(0, 2).unwrap(), [0, 1]);
        assert_eq!(ranges.bytes_read(), 8);
        assert!(ranges.read(250, 7).is_err(), "past the end of the file");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_file_that_is_not_a_bundle_is_refused_and_a_missing_one_too() {
        let path = std::env::temp_dir().join(format!("nfsmw-trees-notbundle-{}", std::process::id()));
        std::fs::write(&path, b"not a bundle at all, not even close to one, and long enough to hold a header....")
            .unwrap();
        assert!(open(&path).is_err());
        let _ = std::fs::remove_file(&path);
        assert!(open(&path).is_err());
    }
}
