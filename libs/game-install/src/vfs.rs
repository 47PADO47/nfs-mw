//! Case-insensitive access to the install, with optional overlay layers above it.
//!
//! Windows games address files case-insensitively, and shipped names often mix
//! cases (`GlobalB.lzc`, `GLOBALA.BUN`). On Linux the filesystem is
//! case-sensitive, so we index the whole tree once and resolve every lookup
//! through a lower-cased key.
//!
//! Layers: the install is the lowest layer. Each overlay (see
//! [`GameDir::add_overlay`]) is a directory whose files replace or add to the
//! files below it. The highest priority wins; at equal priority the overlay
//! added later wins. Overlays work on whole files only.

mod overlay;
#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use overlay::Overlay;
pub use overlay::{LayerError, LayerInfo, Provider};

#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    #[error("{0} is not a directory")]
    NotADirectory(PathBuf),
    #[error("could not index {path}: {source}")]
    Io { path: PathBuf, source: io::Error },
}

/// Lower-cased relative path with `/` separators -> real path, for one layer.
type FileMap = HashMap<String, PathBuf>;

/// An indexed, read-only view of the install directory and the overlays above it.
#[derive(Debug, Clone)]
pub struct GameDir {
    root: PathBuf,
    /// The install, the lowest layer. Shared by clones: the index is built once and never changes, and a clone
    /// of the directory (the game keeps many) is a reference count, not a copy of every path.
    files: Arc<FileMap>,
    /// Overlays in ascending priority. Equal priorities keep the order they were
    /// added in, so the last entry is the highest layer and is searched first.
    overlays: Vec<Overlay>,
}

/// Normalise a game-relative path: `/` or `\` separators, `.` and empty segments dropped,
/// lower case. A leading `/` is read as "from the root". Returns `None` for a path that
/// would leave its root: a `..` segment, or a segment with a colon (a drive prefix such as `C:`).
fn key(rel: &str) -> Option<String> {
    let mut parts = Vec::new();
    for part in rel.split(['/', '\\']) {
        match part {
            "" | "." => {}
            ".." => return None,
            _ if part.contains(':') => return None,
            _ => parts.push(part.to_ascii_lowercase()),
        }
    }
    Some(parts.join("/"))
}

/// The key prefix of the entries directly inside `rel`: empty for the root, else `dir/`.
fn child_prefix(rel: &str) -> Option<String> {
    let k = key(rel)?;
    if k.is_empty() {
        return Some(k);
    }
    Some(format!("{k}/"))
}

/// The original spelling of a directory. `real` is a file inside it and `tail` is the
/// part of the path below the directory.
fn dir_name(real: &Path, tail: &str) -> Option<String> {
    let depth = tail.matches('/').count() + 1;
    let mut dir = real;
    for _ in 0..depth {
        dir = dir.parent()?;
    }
    Some(dir.file_name()?.to_string_lossy().into_owned())
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
                    continue;
                }
                let Ok(rel) = path.strip_prefix(&root) else { continue };
                let Some(k) = key(&rel.to_string_lossy()) else { continue };
                files.insert(k, path);
            }
        }
        log::debug!("indexed {} files under {}", files.len(), root.display());
        Ok(Self { root, files: Arc::new(files), overlays: Vec::new() })
    }

    /// Add a layer above the ones already there. Its files replace or add to the files
    /// below. Symlinks are followed only when they resolve to a file inside `root`.
    pub fn add_overlay(&mut self, id: &str, root: impl AsRef<Path>, priority: i32) -> Result<(), LayerError> {
        if self.overlays.iter().any(|o| o.id == id) {
            return Err(LayerError::DuplicateId(id.to_owned()));
        }
        let overlay = Overlay::index(id, root.as_ref(), priority)?;
        self.overlays.push(overlay);
        // Stable sort: layers of equal priority keep the order they were added in.
        self.overlays.sort_by_key(|o| o.priority);
        Ok(())
    }

    /// The install root (overlays are not under it).
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Number of distinct files visible through all layers.
    pub fn file_count(&self) -> usize {
        self.layers_top_down().flat_map(|files| files.keys()).collect::<HashSet<_>>().len()
    }

    /// The layers from the highest to the lowest: the overlays first, then the install.
    pub fn layers(&self) -> Vec<LayerInfo<'_>> {
        let mut out: Vec<LayerInfo<'_>> = self
            .overlays
            .iter()
            .rev()
            .map(|o| LayerInfo {
                provider: Provider::Overlay(o.id.as_str()),
                priority: Some(o.priority),
                root: o.root.as_path(),
                file_count: o.files.len(),
            })
            .collect();
        out.push(LayerInfo {
            provider: Provider::Install,
            priority: None,
            root: &self.root,
            file_count: self.files.len(),
        });
        out
    }

    /// The real path of a game-relative file, matched case-insensitively. Overlays win over the install.
    pub fn resolve(&self, rel: &str) -> Option<&Path> {
        let k = key(rel)?;
        self.winner(&k).map(|(_, real)| real.as_path())
    }

    /// Which layer serves a game-relative file, for logs and the mod manager.
    pub fn provider(&self, rel: &str) -> Option<Provider<'_>> {
        let k = key(rel)?;
        self.winner(&k).map(|(provider, _)| provider)
    }

    pub fn exists(&self, rel: &str) -> bool {
        self.resolve(rel).is_some()
    }

    /// Size in bytes of the file that a lookup finds.
    pub fn size(&self, rel: &str) -> io::Result<u64> {
        Ok(std::fs::metadata(self.require(rel)?)?.len())
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

    /// Names (original case) of the files directly in `rel`, sorted. On a name clash the
    /// highest layer's spelling is used.
    pub fn files_in(&self, rel: &str) -> Vec<String> {
        let Some(prefix) = child_prefix(rel) else { return Vec::new() };
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for files in self.layers_top_down() {
            for (k, real) in files {
                let Some(rest) = k.strip_prefix(prefix.as_str()) else { continue };
                if rest.contains('/') {
                    continue;
                }
                if !seen.insert(k.as_str()) {
                    continue;
                }
                let Some(name) = real.file_name() else { continue };
                out.push(name.to_string_lossy().into_owned());
            }
        }
        out.sort_unstable_by_key(|s| s.to_ascii_lowercase());
        out
    }

    /// Names (original case) of the immediate subdirectories of `rel`, sorted. On a name
    /// clash the highest layer's spelling is used.
    pub fn subdirectories(&self, rel: &str) -> Vec<String> {
        let Some(prefix) = child_prefix(rel) else { return Vec::new() };
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for files in self.layers_top_down() {
            for (k, real) in files {
                let Some(rest) = k.strip_prefix(prefix.as_str()) else { continue };
                let Some((dir_key, tail)) = rest.split_once('/') else { continue };
                if !seen.insert(dir_key) {
                    continue;
                }
                let Some(name) = dir_name(real, tail) else { continue };
                out.push(name);
            }
        }
        out.sort_unstable_by_key(|s| s.to_ascii_lowercase());
        out
    }

    /// The overlays from the highest to the lowest, then the install.
    fn layers_top_down(&self) -> impl Iterator<Item = &FileMap> + '_ {
        self.overlays.iter().rev().map(|o| &*o.files).chain(std::iter::once(&*self.files))
    }

    /// The layer and real path of the highest file under a normalised key.
    fn winner(&self, k: &str) -> Option<(Provider<'_>, &PathBuf)> {
        for overlay in self.overlays.iter().rev() {
            if let Some(real) = overlay.files.get(k) {
                return Some((Provider::Overlay(overlay.id.as_str()), real));
            }
        }
        self.files.get(k).map(|real| (Provider::Install, real))
    }
}
