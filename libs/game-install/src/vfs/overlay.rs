//! Overlay layers: a directory whose files replace or add to the files of the layers below.
//!
//! An overlay is indexed once, when it is added. Nothing under it is written, and
//! nothing outside its root is reachable: a symlink counts only when it resolves to
//! a file inside the root, and is otherwise skipped.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::{FileMap, key};

/// Why an overlay could not be added.
#[derive(Debug, thiserror::Error)]
pub enum LayerError {
    #[error("{0} is not a directory")]
    NotADirectory(PathBuf),
    #[error("could not index {path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("a layer with id `{0}` is already added")]
    DuplicateId(String),
}

/// The layer that serves a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider<'a> {
    /// The install, the lowest layer.
    Install,
    /// An overlay, named by the id it was added with.
    Overlay(&'a str),
}

/// One layer, as listed by [`super::GameDir::layers`].
#[derive(Debug, Clone, Copy)]
pub struct LayerInfo<'a> {
    pub provider: Provider<'a>,
    /// `None` for the install, which sits below every overlay.
    pub priority: Option<i32>,
    pub root: &'a Path,
    /// Files this layer holds, before any are shadowed by a higher layer.
    pub file_count: usize,
}

/// An indexed overlay directory.
#[derive(Debug, Clone)]
pub(super) struct Overlay {
    pub(super) id: String,
    pub(super) priority: i32,
    /// The root as the caller gave it.
    pub(super) root: PathBuf,
    /// Shared by clones, like the install's index.
    pub(super) files: Arc<FileMap>,
}

impl Overlay {
    /// Index `root`. Every file found must sit under the canonical root.
    pub(super) fn index(id: &str, root: &Path, priority: i32) -> Result<Self, LayerError> {
        let anchor = root.canonicalize().map_err(|source| LayerError::Io { path: root.to_path_buf(), source })?;
        if !anchor.is_dir() {
            return Err(LayerError::NotADirectory(root.to_path_buf()));
        }
        let mut files = FileMap::new();
        let mut stack = vec![anchor.clone()];
        while let Some(dir) = stack.pop() {
            let entries = std::fs::read_dir(&dir).map_err(|source| LayerError::Io { path: dir.clone(), source })?;
            for entry in entries {
                let entry = entry.map_err(|source| LayerError::Io { path: dir.clone(), source })?;
                let path = entry.path();
                let ty = entry.file_type().map_err(|source| LayerError::Io { path: path.clone(), source })?;
                if ty.is_symlink() {
                    index_link(&path, &anchor, &mut files);
                    continue;
                }
                if ty.is_dir() {
                    stack.push(path);
                    continue;
                }
                insert(&mut files, &anchor, &path, path.clone());
            }
        }
        log::debug!("indexed {} files in overlay {} under {}", files.len(), id, anchor.display());
        Ok(Self { id: id.to_owned(), priority, root: root.to_path_buf(), files: Arc::new(files) })
    }
}

/// Record `real` under the game key of `link`, its place inside the layer.
fn insert(files: &mut FileMap, anchor: &Path, link: &Path, real: PathBuf) {
    let Ok(rel) = link.strip_prefix(anchor) else { return };
    let Some(k) = key(&rel.to_string_lossy()) else { return };
    files.insert(k, real);
}

/// Index a symlink only when it resolves to a file inside the layer root. A link that
/// points outside, to a directory, or to nothing is skipped and logged.
fn index_link(link: &Path, anchor: &Path, files: &mut FileMap) {
    let target = match link.canonicalize() {
        Ok(target) => target,
        Err(err) => {
            log::warn!("skipping overlay link {}: {err}", link.display());
            return;
        }
    };
    if !target.starts_with(anchor) || !target.is_file() {
        log::warn!("skipping overlay link {}: it does not resolve to a file inside its layer", link.display());
        return;
    }
    insert(files, anchor, link, target);
}
