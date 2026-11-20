//! Replacement textures from loose DDS files: the `textures/` folders of mods and the HD texture pack folder the
//! settings name. A rewrite extension: the original has none. The rules are in `docs/texture-packs.md`.
//!
//! Each source is a folder with a priority: a mod's folder has the mod's `[content] priority`, the settings' folder
//! [`SETTINGS_PRIORITY`], below every mod. A texture takes the file of the highest source that names it; at equal
//! priority the source listed later wins, as the `files/` layers do. A file keeps the texture's name, alpha rules and
//! flags, and brings its own size, format and mip chain.
//!
//! A replacement never undoes a higher mod. A texture read from a file inside a loaded mod's folder (its `files/` layer
//! or its own map packs) is replaced only by a source of at least that mod's priority: the floor of the file. A file of
//! the install has no floor.

mod source;
#[cfg(test)]
mod tests;

pub use source::{Source, SourceSpec};

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
};

use blackbox_tpk::Texture;
use game_install::GameDir;

/// The priority of the settings' folder: below every mod, so a mod's own textures are never replaced by it.
pub const SETTINGS_PRIORITY: i32 = i32::MIN;

/// A loaded mod's folder: a file inside it is served at the mod's priority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModFolder {
    pub dir: PathBuf,
    pub priority: i32,
}

/// Every source of the run, indexed.
#[derive(Debug, Default)]
pub struct TexturePacks {
    /// In the order given.
    sources: Vec<Source>,
    /// The winning source of each texture hash.
    best: HashMap<u32, usize>,
    /// Each loaded mod's folder with its priority, as given and as the file system resolves it: the install's layers
    /// hand out paths under the resolved folder (links and junctions followed), map packs under the given one.
    mod_roots: Vec<(PathBuf, i32)>,
    /// The largest side a replacement keeps, in pixels: larger files drop their largest stored mip levels. 0 is no cap.
    max_size: u32,
    /// Texture hashes that a higher source took from a lower one.
    shadowed: usize,
    /// Replacements a higher mod's own copy of the texture held back.
    held_back: AtomicUsize,
}

impl TexturePacks {
    /// Index `specs` (lowest first) and remember the loaded mods' folders for the floor rule.
    pub fn new(specs: Vec<SourceSpec>, mods: Vec<ModFolder>, max_size: u32) -> Self {
        let sources: Vec<Source> = specs.into_iter().map(Source::scan).collect();
        let mut order: Vec<usize> = (0..sources.len()).collect();
        order.sort_by_key(|&i| sources[i].spec.priority);
        let mut best = HashMap::new();
        let mut shadowed = 0;
        for i in order {
            for &hash in sources[i].files.keys() {
                if best.insert(hash, i).is_some() {
                    shadowed += 1;
                }
            }
        }
        let mut mod_roots = Vec::new();
        for m in mods {
            if let Ok(resolved) = m.dir.canonicalize() {
                mod_roots.push((resolved, m.priority));
            }
            mod_roots.push((m.dir, m.priority));
        }
        Self { sources, best, mod_roots, max_size, shadowed, held_back: AtomicUsize::new(0) }
    }

    /// The sources, in the order given.
    pub fn sources(&self) -> &[Source] {
        &self.sources
    }

    /// The texture hashes that have a file in some source.
    pub fn len(&self) -> usize {
        self.best.len()
    }

    pub fn is_empty(&self) -> bool {
        self.best.is_empty()
    }

    /// Texture hashes that more than one source names (the higher one is used).
    pub fn shadowed(&self) -> usize {
        self.shadowed
    }

    pub fn max_size(&self) -> u32 {
        self.max_size
    }

    /// Replacements held back so far because a higher mod supplies the file the texture was read from.
    pub fn held_back(&self) -> usize {
        self.held_back.load(Ordering::Relaxed)
    }

    /// The floor of a file on disk: the priority of the loaded mod whose folder holds it (the innermost, if folders
    /// nest), or `None` for a file of the install.
    pub fn floor(&self, path: &Path) -> Option<i32> {
        self.mod_roots
            .iter()
            .filter(|(root, _)| path.starts_with(root))
            .max_by_key(|(root, _)| root.components().count())
            .map(|&(_, priority)| priority)
    }

    /// Replace the pixels of every texture that has a file in a source at or above `floor`. A file that cannot be
    /// used leaves the texture alone. Returns how many were replaced.
    pub fn apply_above(&self, floor: Option<i32>, textures: &mut [Texture]) -> usize {
        let mut replaced = 0;
        for texture in textures {
            let Some(&index) = self.best.get(&texture.name_hash) else {
                continue;
            };
            let source = &self.sources[index];
            if floor.is_some_and(|floor| source.spec.priority < floor) {
                self.held_back.fetch_add(1, Ordering::Relaxed);
                continue;
            }
            let file = &source.files[&texture.name_hash];
            let image = std::fs::read(file).map_err(|error| error.to_string());
            let image = image
                .and_then(|bytes| blackbox_tpk::read_dds(&texture.name, &bytes).map_err(|error| error.to_string()));
            let mut image = match image {
                Ok(image) => image,
                Err(error) => {
                    log::debug!("texture replacement {}: {}: {error}", source.spec.id, file.display());
                    continue;
                }
            };
            image.limit_size(self.max_size);
            texture.take_pixels(image);
            source.replaced.fetch_add(1, Ordering::Relaxed);
            replaced += 1;
        }
        replaced
    }

    /// One line per source and one for the whole, for the start-up log.
    pub fn describe(&self) -> Vec<String> {
        let mut lines: Vec<String> = self
            .sources
            .iter()
            .map(|s| {
                format!(
                    "{} ({}): {} file(s) under {}",
                    s.spec.id,
                    priority(s.spec.priority),
                    s.file_count(),
                    s.spec.root.display()
                )
            })
            .collect();
        let cap = match self.max_size {
            0 => "no size cap".to_owned(),
            max => format!("at most {max} px a side"),
        };
        lines.push(format!(
            "{} texture(s) have a replacement, {} named by more than one source; {cap}",
            self.len(),
            self.shadowed
        ));
        lines
    }
}

/// A priority as the logs and the F8 panel show it.
pub fn priority(priority: i32) -> String {
    match priority {
        SETTINGS_PRIORITY => "lowest".to_owned(),
        p => format!("priority {p}"),
    }
}

/// The sources of this run: set once, when the install opens with its mods, before any scene loads.
static PACKS: OnceLock<TexturePacks> = OnceLock::new();
/// Game textures replaced so far, over every source.
static REPLACED: AtomicUsize = AtomicUsize::new(0);

/// Make `packs` the sources of this run. The first call counts; a later one changes nothing (a new folder needs a
/// restart) and returns the sources already in force.
pub fn seal(packs: TexturePacks) -> &'static TexturePacks {
    if PACKS.set(packs).is_err() {
        log::debug!("texture replacement: the sources were already set for this run; the first set is kept");
    }
    PACKS.get().expect("set just above")
}

/// The sources in force: `None` until [`seal`] runs.
pub fn active() -> Option<&'static TexturePacks> {
    PACKS.get()
}

/// How many game textures have been replaced so far (a texture loaded twice counts twice).
pub fn replaced() -> usize {
    REPLACED.load(Ordering::Relaxed)
}

/// Apply the sources to textures read from the file `path` on disk, with that file's floor.
pub fn apply(path: &Path, textures: &mut [Texture]) {
    let Some(packs) = active().filter(|packs| !packs.is_empty()) else {
        return;
    };
    REPLACED.fetch_add(packs.apply_above(packs.floor(path), textures), Ordering::Relaxed);
}

/// Apply the sources to textures read from the install's file `rel`, with the floor of the layer that serves it.
pub fn apply_file(dir: &GameDir, rel: &str, textures: &mut [Texture]) {
    let Some(path) = dir.resolve(rel) else {
        return;
    };
    apply(path, textures);
}
