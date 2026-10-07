//! Case-insensitive access to the install.
//!
//! Windows games address files case-insensitively, and shipped names often mix
//! cases (`GlobalB.lzc`, `GLOBALA.BUN`). On Linux the filesystem is
//! case-sensitive, so we index the whole tree once and resolve every lookup
//! through a lower-cased key.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    #[error("{0} is not a directory")]
    NotADirectory(PathBuf),
    #[error("could not index {path}: {source}")]
    Io { path: PathBuf, source: io::Error },
}

/// An indexed, read-only view of the install directory.
#[derive(Debug, Clone)]
pub struct GameDir {
    root: PathBuf,
    /// Lower-cased relative path with `/` separators -> real path.
    files: HashMap<String, PathBuf>,
}

/// Normalise a game-relative path: `/` separators, no leading `./` or `/`, lower case.
fn key(rel: &str) -> String {
    rel.replace('\\', "/").trim_start_matches("./").trim_start_matches('/').to_ascii_lowercase()
}

impl GameDir {
    /// Index every file under `root`.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, OpenError> {
        let root = root.into();
        if !root.is_dir() {
            return Err(OpenError::NotADirectory(root));
        }
        let mut files = HashMap::new();
        let mut stack = vec![root.clone()];
        while let Some(dir) = stack.pop() {
            let entries = std::fs::read_dir(&dir).map_err(|source| OpenError::Io { path: dir.clone(), source })?;
            for entry in entries {
                let entry = entry.map_err(|source| OpenError::Io { path: dir.clone(), source })?;
                let path = entry.path();
                let ty = entry.file_type().map_err(|source| OpenError::Io { path: path.clone(), source })?;
                if ty.is_dir() {
                    stack.push(path);
                } else if let Ok(rel) = path.strip_prefix(&root) {
                    let rel = rel.to_string_lossy();
                    files.insert(key(&rel), path);
                }
            }
        }
        log::debug!("indexed {} files under {}", files.len(), root.display());
        Ok(Self { root, files })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// The real path of a game-relative file, matched case-insensitively.
    pub fn resolve(&self, rel: &str) -> Option<&Path> {
        self.files.get(&key(rel)).map(PathBuf::as_path)
    }

    pub fn exists(&self, rel: &str) -> bool {
        self.resolve(rel).is_some()
    }

    /// Read a whole file.
    pub fn read(&self, rel: &str) -> io::Result<Vec<u8>> {
        std::fs::read(self.require(rel)?)
    }

    /// Open a file for reading (e.g. to read many ranges of a large stream file).
    pub fn open_file(&self, rel: &str) -> io::Result<std::fs::File> {
        std::fs::File::open(self.require(rel)?)
    }

    /// Read `len` bytes at `offset` of a file.
    pub fn read_range(&self, rel: &str, offset: u64, len: usize) -> io::Result<Vec<u8>> {
        use std::io::{Read, Seek, SeekFrom};
        let mut file = self.open_file(rel)?;
        file.seek(SeekFrom::Start(offset))?;
        let mut buf = vec![0; len];
        file.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn require(&self, rel: &str) -> io::Result<&Path> {
        self.resolve(rel).ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("{rel} is not in the install")))
    }

    /// Names (original case) of the immediate subdirectories of `rel`.
    pub fn subdirectories(&self, rel: &str) -> Vec<String> {
        let prefix = {
            let k = key(rel);
            if k.is_empty() { k } else { format!("{k}/") }
        };
        let mut out: Vec<String> = self
            .files
            .iter()
            .filter_map(|(k, real)| {
                let rest = k.strip_prefix(&prefix)?;
                let (_, tail) = rest.split_once('/')?;
                // Recover the original spelling from the real path.
                let depth = tail.matches('/').count() + 1;
                let mut p = real.as_path();
                for _ in 0..depth {
                    p = p.parent()?;
                }
                Some(p.file_name()?.to_string_lossy().into_owned())
            })
            .collect();
        out.sort_unstable_by_key(|s| s.to_ascii_lowercase());
        out.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_tree(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("game-install-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("GLOBAL")).unwrap();
        std::fs::create_dir_all(root.join("Cars/BMWM3GTR")).unwrap();
        std::fs::write(root.join("GLOBAL/GlobalB.lzc"), b"x").unwrap();
        std::fs::write(root.join("Cars/BMWM3GTR/GEOMETRY.BIN"), b"y").unwrap();
        root
    }

    #[test]
    fn lookups_ignore_case_and_separators() {
        let root = temp_tree("case");
        let dir = GameDir::open(&root).unwrap();
        assert_eq!(dir.file_count(), 2);
        assert!(dir.exists("global/GLOBALB.LZC"));
        assert!(dir.exists("GLOBAL\\globalb.lzc"));
        assert_eq!(dir.read("cars/bmwm3gtr/geometry.bin").unwrap(), b"y");
        assert_eq!(dir.read_range("GLOBAL/GLOBALB.LZC", 0, 1).unwrap(), b"x");
        assert!(dir.read_range("GLOBAL/GLOBALB.LZC", 0, 2).is_err());
        assert!(!dir.exists("GLOBAL/missing.bin"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn subdirectories_keep_original_case() {
        let root = temp_tree("subdirs");
        let dir = GameDir::open(&root).unwrap();
        assert_eq!(dir.subdirectories("CARS"), ["BMWM3GTR"]);
        let top = dir.subdirectories("");
        assert_eq!(top, ["Cars", "GLOBAL"]);
        std::fs::remove_dir_all(root).unwrap();
    }
}
