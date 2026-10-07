//! Case-insensitive access to the install.
//!
//! The game addresses files case-insensitively (it was written for Windows), and
//! the shipped names mix cases (`GlobalB.lzc`, `GLOBALA.BUN`). On Linux the
//! filesystem is case-sensitive, so we index the whole tree once and resolve
//! every lookup through a lower-cased key.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::{KNOWN_EXECUTABLES, REQUIRED_FILES, Validation};

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
        let path = self
            .resolve(rel)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("{rel} is not in the install")))?;
        std::fs::read(path)
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

    /// Check required files and identify `speed.exe`.
    pub fn validate(&self) -> Validation {
        let missing = REQUIRED_FILES.iter().copied().filter(|f| !self.exists(f)).collect();
        let exe_sha256 = self
            .read("speed.exe")
            .ok()
            .map(|bytes| Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect::<String>());
        let exe_version =
            exe_sha256.as_deref().and_then(|h| KNOWN_EXECUTABLES.iter().find(|(k, _)| *k == h).map(|(_, v)| *v));
        Validation { missing, exe_sha256, exe_version }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_tree(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("nfsmw-install-test-{name}-{}", std::process::id()));
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

    #[test]
    fn validation_reports_missing_files() {
        let root = temp_tree("validate");
        let v = GameDir::open(&root).unwrap().validate();
        assert!(!v.is_usable());
        assert!(v.missing.contains(&"TRACKS/L2RA.BUN"));
        assert!(!v.missing.contains(&"GLOBAL/GLOBALB.LZC"));
        assert!(v.exe_sha256.is_none());
        std::fs::remove_dir_all(root).unwrap();
    }
}
