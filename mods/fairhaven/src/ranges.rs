//! A bundle file read by byte ranges: the reader the bundle crate asks for the table of resources and for the blocks of the
//! resources that are read. Positional reads, so loader threads read one file at the same time without a lock.

use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use chameleon_bundle::Ranges;

/// A file of the install, opened once.
#[derive(Debug)]
pub struct FileRanges {
    file: File,
    size: u64,
    path: PathBuf,
}

impl FileRanges {
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let size = file.metadata()?.len();
        Ok(Self { file, size, path: path.to_path_buf() })
    }

    /// The path the file was opened from.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(windows)]
fn read_exact_at(file: &File, mut buf: &mut [u8], mut offset: u64) -> io::Result<()> {
    use std::os::windows::fs::FileExt;
    while !buf.is_empty() {
        let read = file.seek_read(buf, offset)?;
        if read == 0 {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        buf = &mut buf[read..];
        offset += read as u64;
    }
    Ok(())
}

#[cfg(unix)]
fn read_exact_at(file: &File, buf: &mut [u8], offset: u64) -> io::Result<()> {
    use std::os::unix::fs::FileExt;
    file.read_exact_at(buf, offset)
}

impl Ranges for FileRanges {
    fn size(&self) -> u64 {
        self.size
    }

    fn read(&self, start: u64, count: usize) -> io::Result<Vec<u8>> {
        let mut bytes = vec![0; count];
        read_exact_at(&self.file, &mut bytes, start)?;
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_range_of_a_file_is_read_and_one_past_the_end_is_an_error() {
        let path = std::env::temp_dir().join(format!("fairhaven-ranges-{}.bin", std::process::id()));
        std::fs::write(&path, (0..=255u8).collect::<Vec<_>>()).unwrap();
        let ranges = FileRanges::open(&path).unwrap();
        assert_eq!(ranges.size(), 256);
        assert_eq!(ranges.read(250, 4).unwrap(), [250, 251, 252, 253]);
        assert!(ranges.read(254, 4).is_err());
        assert_eq!(ranges.path(), path.as_path());
        drop(ranges);
        std::fs::remove_file(&path).unwrap();
    }
}
