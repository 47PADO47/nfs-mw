//! The exported meshes of a bike: every `.glb` below a folder, found by file stem and level of detail.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::glb::Glb;

/// The `.glb` files of an export folder, by lower-case file name.
pub struct Exports {
    files: HashMap<String, PathBuf>,
}

impl Exports {
    pub fn scan(root: &Path) -> Result<Self> {
        let mut files = HashMap::new();
        let mut pending = vec![root.to_path_buf()];
        while let Some(dir) = pending.pop() {
            for entry in std::fs::read_dir(&dir).with_context(|| format!("reading {}", dir.display()))? {
                let path = entry?.path();
                if path.is_dir() {
                    pending.push(path);
                    continue;
                }
                let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue };
                if name.to_ascii_lowercase().ends_with(".glb") {
                    files.insert(name.to_ascii_lowercase(), path);
                }
            }
        }
        Ok(Self { files })
    }

    /// The file of `stem` at level of detail `level`, or the nearest coarser-numbered level below it that exists.
    pub fn find(&self, stem: &str, level: u32) -> Option<&PathBuf> {
        (0..=level).rev().find_map(|n| {
            let name = if n == 0 { format!("{stem}.glb") } else { format!("{stem}_lod{n}.glb") };
            self.files.get(&name.to_ascii_lowercase())
        })
    }

    pub fn load(&self, stem: &str, level: u32) -> Result<Glb> {
        let path =
            self.find(stem, level).with_context(|| format!("no export of {stem} (level {level}) in the folder"))?;
        let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        Glb::parse(&bytes).with_context(|| format!("reading {}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_level_falls_back_to_a_finer_one() {
        let mut files = HashMap::new();
        files.insert("a.glb".to_owned(), PathBuf::from("a0"));
        files.insert("a_lod1.glb".to_owned(), PathBuf::from("a1"));
        let exports = Exports { files };
        assert_eq!(exports.find("a", 3), Some(&PathBuf::from("a1")));
        assert_eq!(exports.find("A", 0), Some(&PathBuf::from("a0")));
        assert_eq!(exports.find("b", 2), None);
    }
}
