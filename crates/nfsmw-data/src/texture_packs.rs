//! Replacement textures from loose DDS files, for HD texture packs. A rewrite extension: the original has none.
//!
//! The game names a folder once at start-up ([`set_root`]); it is searched with its subfolders. A DDS file replaces the game texture
//! its file name stands for: a name (`ARC_BROWNBRICKWALL.dds`) or the texture's hash (`0x17C498C9.dds`). A
//! `TexturePackInfo.json` beside a pack (the layout of the NextGenGraphics packs: `rootDirectory` and
//! `textureMappings` with a `gameId` and a `texturePath` each) is read too, so a file may be named differently from
//! its texture. The texture keeps its name, alpha rules and flags, and takes the file's size, format and mip chain.
//! Only the world's textures (the sections and the global packs) are replaced. Nothing is written to the install.

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{
        OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
};

use blackbox_tpk::Texture;

const INFO: &str = "TexturePackInfo.json";

#[derive(Debug, Default)]
pub struct TexturePacks {
    root: PathBuf,
    files: HashMap<u32, PathBuf>,
    /// The distinct files in `files`.
    file_count: usize,
}

/// The hashes a game id can mean: `0x` and eight hex digits is the hash itself, anything else a texture name.
fn hashes(id: &str) -> Vec<u32> {
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
fn mappings(info: &Path) -> Vec<(String, PathBuf)> {
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

impl TexturePacks {
    /// Index every replacement under `root`. A texture named by two files keeps the first, in path order.
    pub fn scan(root: &Path) -> Self {
        let mut packs = Self { root: root.to_owned(), ..Self::default() };
        let mut folders = vec![root.to_owned()];
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
                    mappings(&path).into_iter().for_each(|(id, file)| packs.add(&id, file));
                    continue;
                }
                let dds = path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("dds"));
                let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()).filter(|_| dds) else {
                    continue;
                };
                let id = stem.to_owned();
                packs.add(&id, path);
            }
        }
        packs.file_count = packs.files.values().collect::<HashSet<_>>().len();
        packs
    }

    fn add(&mut self, id: &str, file: PathBuf) {
        for hash in hashes(id) {
            self.files.entry(hash).or_insert_with(|| file.clone());
        }
    }

    /// The texture hashes that have a file.
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// The folder that was searched.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The replacement files: a file named for a texture by both its file name and a pack's mapping counts once.
    pub fn file_count(&self) -> usize {
        self.file_count
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Replace the pixels of every texture that has a file. A file that cannot be used leaves the texture alone.
    pub fn apply(&self, textures: &mut [Texture]) -> usize {
        let mut replaced = 0;
        for texture in textures {
            let Some(file) = self.files.get(&texture.name_hash) else {
                continue;
            };
            let image = std::fs::read(file).map_err(|error| error.to_string());
            let image = image.and_then(|bytes| blackbox_tpk::read_dds(&bytes).map_err(|error| error.to_string()));
            match image {
                Ok(image) => {
                    texture.replace_image(image);
                    replaced += 1;
                }
                Err(error) => log::debug!("texture pack: {}: {error}", file.display()),
            }
        }
        replaced
    }
}

/// The packs the game chose at start-up: unset until [`set_root`] runs, `None` inside when no folder is set.
static PACKS: OnceLock<Option<TexturePacks>> = OnceLock::new();
/// The game textures replaced so far.
static REPLACED: AtomicUsize = AtomicUsize::new(0);

/// Index the replacement textures under `root` (none for `None`). The game calls this once, before the world loads;
/// later calls change nothing, so a new folder needs a restart. Returns the packs in force.
pub fn set_root(root: Option<&Path>) -> Option<&'static TexturePacks> {
    PACKS.get_or_init(|| root.map(TexturePacks::scan)).as_ref()
}

/// The packs in force: `None` until [`set_root`] names a folder.
pub fn active() -> Option<&'static TexturePacks> {
    PACKS.get().and_then(Option::as_ref)
}

/// How many game textures the packs have replaced so far (a texture loaded twice counts twice).
pub fn replaced() -> usize {
    REPLACED.load(Ordering::Relaxed)
}

/// Apply the active packs, if any, to a pack's textures.
pub fn apply(textures: &mut [Texture]) {
    let Some(packs) = active().filter(|packs| !packs.is_empty()) else {
        return;
    };
    REPLACED.fetch_add(packs.apply(textures), Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_game_id_is_a_hash_or_a_name() {
        assert_eq!(hashes("0x17c498c9"), [0x17C4_98C9]);
        let name = blackbox_hash::bstring_hash("ARC_BROWNBRICKWALL");
        assert!(hashes("arc_brownbrickwall").contains(&name));
    }

    #[test]
    fn a_pack_info_maps_ids_to_files_under_its_root() {
        let dir = std::env::temp_dir().join(format!("nfsmw-texture-packs-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("RESOURCES")).unwrap();
        let info = dir.join(INFO);
        let text = r#"{ "rootDirectory": "RESOURCES", "textureMappings": [
            { "gameId": "0x2b3f2355", "texturePath": "a.dds" },
            { "gameId": "ROAD", "texturePath": "sub\\b.dds" }, ] }"#;
        std::fs::write(&info, text).unwrap();
        let found = mappings(&info);
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(found[0], ("0x2b3f2355".to_owned(), dir.join("RESOURCES/a.dds")));
        assert_eq!(found[1], ("ROAD".to_owned(), dir.join("RESOURCES/sub/b.dds")));
    }
}
