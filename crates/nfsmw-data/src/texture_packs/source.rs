//! One source of replacement textures: a folder of loose DDS files, searched with its subfolders, and the
//! `TexturePackInfo.json` mappings of the NextGenGraphics packs inside it.

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

const INFO: &str = "TexturePackInfo.json";

/// Where a source comes from and where it ranks: a mod's `textures/` folder at the mod's priority, or the folder the
/// settings name at [`super::SETTINGS_PRIORITY`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSpec {
    /// The mod id, or `settings` for the settings' folder. For the logs and the F8 panel.
    pub id: String,
    pub root: PathBuf,
    pub priority: i32,
}

/// An indexed source: the file for each texture hash it names.
#[derive(Debug)]
pub struct Source {
    pub spec: SourceSpec,
    pub(super) files: HashMap<u32, PathBuf>,
    file_count: usize,
    pub(super) replaced: AtomicUsize,
}

/// The hashes a game id can mean: `0x` and eight hex digits is the hash itself, anything else a texture name.
pub(super) fn hashes(id: &str) -> Vec<u32> {
    let hex = id.strip_prefix("0x").or_else(|| id.strip_prefix("0X"));
    if let Some(hash) = hex.and_then(|digits| u32::from_str_radix(digits, 16).ok()) {
        return vec![hash];
    }
    vec![blackbox_hash::bstring_hash(id), blackbox_hash::bstring_hash(&id.to_ascii_uppercase())]
}

/// The quoted value that follows `"key":` at or after `from`, and where it ends.
fn value_after<'a>(text: &'a str, key: &str, from: usize) -> Option<(&'a str, usize)> {
    let at = from + text[from..].find(key)? + key.len();
    let open = at + text[at..].find('"')? + 1;
    let close = open + text[open..].find('"')?;
    Some((&text[open..close], close + 1))
}

/// The `(gameId, file)` pairs of a `TexturePackInfo.json`. A small scan, not a JSON parser: the packs in the wild
/// have trailing commas and stray whitespace.
pub(super) fn mappings(info: &Path) -> Vec<(String, PathBuf)> {
    let Ok(text) = std::fs::read_to_string(info) else {
        return Vec::new();
    };
    let folder = info.parent().unwrap_or(Path::new("."));
    let root = value_after(&text, "\"rootDirectory\"", 0).map_or(folder.to_owned(), |(name, _)| folder.join(name));
    let mut out = Vec::new();
    let mut at = 0;
    while let Some((id, next)) = value_after(&text, "\"gameId\"", at) {
        let Some((file, end)) = value_after(&text, "\"texturePath\"", next) else {
            break;
        };
        out.push((id.to_owned(), root.join(file.replace('\\', "/"))));
        at = end;
    }
    out
}

impl Source {
    /// Index every replacement under the spec's folder. A texture named by two files of the source keeps the first, in
    /// path order. A folder that cannot be read gives an empty source.
    pub fn scan(spec: SourceSpec) -> Self {
        let mut files = HashMap::new();
        let mut folders = vec![spec.root.clone()];
        while let Some(folder) = folders.pop() {
            let Ok(entries) = std::fs::read_dir(&folder) else {
                continue;
            };
            let mut paths: Vec<PathBuf> = entries.filter_map(|entry| entry.ok().map(|entry| entry.path())).collect();
            paths.sort();
            for path in paths {
                if path.is_dir() {
                    folders.push(path);
                    continue;
                }
                if path.file_name().is_some_and(|name| name == INFO) {
                    mappings(&path).into_iter().for_each(|(id, file)| add(&mut files, &id, file));
                    continue;
                }
                let dds = path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("dds"));
                let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()).filter(|_| dds) else {
                    continue;
                };
                let id = stem.to_owned();
                add(&mut files, &id, path);
            }
        }
        let file_count = files.values().collect::<HashSet<_>>().len();
        Self { spec, files, file_count, replaced: AtomicUsize::new(0) }
    }

    /// The texture hashes that have a file.
    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// The replacement files: a file named for a texture by both its file name and a pack's mapping counts once.
    pub fn file_count(&self) -> usize {
        self.file_count
    }

    /// How many game textures this source has replaced so far (a texture loaded twice counts twice).
    pub fn replaced(&self) -> usize {
        self.replaced.load(Ordering::Relaxed)
    }
}

fn add(files: &mut HashMap<u32, PathBuf>, id: &str, file: PathBuf) {
    for hash in hashes(id) {
        files.entry(hash).or_insert_with(|| file.clone());
    }
}
