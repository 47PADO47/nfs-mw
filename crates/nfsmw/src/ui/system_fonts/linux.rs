//! The font files tried on Linux. Without a fontconfig dependency, the font folders are walked a few levels deep for
//! known file names, and each role takes the ones it wants in its own order of preference.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::Candidate;

/// How many folder levels below a root are read. Distributions keep fonts two or three levels down
/// (`/usr/share/fonts/truetype/dejavu/`).
pub const SEARCH_DEPTH: usize = 4;

/// Numbers: condensed and heavy first, then the regular faces most systems have.
pub const NUMERIC_FILES: &[&str] = &[
    "DejaVuSansCondensed-Bold.ttf",
    "LiberationSansNarrow-Bold.ttf",
    "NotoSans-CondensedSemiBold.ttf",
    "NotoSans-Regular.ttf",
    "DejaVuSans.ttf",
];
/// Labels: a neutral regular face, condensed where one is installed.
pub const LABEL_FILES: &[&str] =
    &["NotoSans-Regular.ttf", "DejaVuSansCondensed.ttf", "LiberationSans-Regular.ttf", "DejaVuSans.ttf"];
/// Spaced captions: a neutral regular face at its full width.
pub const LABEL_WIDE_FILES: &[&str] = &["NotoSans-Regular.ttf", "LiberationSans-Regular.ttf", "DejaVuSans.ttf"];

/// The candidates of each role, in `Role::ALL` order, found under `roots`. The folders are walked once for every role.
pub fn candidates(roots: &[PathBuf]) -> [Vec<Candidate>; 3] {
    let wanted: Vec<&str> = NUMERIC_FILES.iter().chain(LABEL_FILES).chain(LABEL_WIDE_FILES).copied().collect();
    let found = search(roots, &wanted, SEARCH_DEPTH);
    let pick = |names: &[&str]| -> Vec<Candidate> {
        names.iter().filter_map(|name| found.get(*name)).map(|path| Candidate::new(path.clone(), 0, &[])).collect()
    };
    [pick(NUMERIC_FILES), pick(LABEL_FILES), pick(LABEL_WIDE_FILES)]
}

/// The first path of each of `names` under `roots`, by file name. A root reads `depth` levels of folders, itself
/// included; a name found under an earlier root, or earlier in a folder's sorted listing, wins.
pub fn search(roots: &[PathBuf], names: &[&str], depth: usize) -> HashMap<String, PathBuf> {
    let mut found = HashMap::new();
    for root in roots {
        walk(root, names, depth, &mut found);
    }
    found
}

fn walk(dir: &Path, names: &[&str], depth: usize, found: &mut HashMap<String, PathBuf>) {
    if depth == 0 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut paths: Vec<PathBuf> = entries.filter_map(|entry| entry.ok().map(|entry| entry.path())).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            walk(&path, names, depth - 1, found);
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else { continue };
        if !names.contains(&name) || found.contains_key(name) {
            continue;
        }
        let key = name.to_owned();
        found.insert(key, path);
    }
}
