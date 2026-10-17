//! The choice of files in a folder: by extension, in a fixed order.

use std::path::{Path, PathBuf};

/// The files whose extension is one of `extensions` (compared without case), sorted by file name, case-insensitively,
/// with the exact name breaking ties. The caller passes the files of one folder and leaves out subfolders. Names
/// that are not valid text, and files without an extension, are left out.
pub fn select(files: impl IntoIterator<Item = PathBuf>, extensions: &[&str]) -> Vec<PathBuf> {
    let mut picked: Vec<PathBuf> = files.into_iter().filter(|path| has_extension(path, extensions)).collect();
    picked.sort_by_cached_key(|path| sort_key(path));
    picked
}

fn has_extension(path: &Path, extensions: &[&str]) -> bool {
    let Some(extension) = path.extension().and_then(|e| e.to_str()) else { return false };
    extensions.iter().any(|wanted| wanted.eq_ignore_ascii_case(extension))
}

fn sort_key(path: &Path) -> (String, String) {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    (name.to_lowercase(), name.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(paths: &[PathBuf]) -> Vec<String> {
        paths.iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect()
    }

    #[test]
    fn only_the_listed_extensions_are_kept_in_any_case() {
        let files = ["a.MP3", "b.ogg", "c.txt", "d.Flac", "e", "f.wav", "g.m3u"].map(PathBuf::from);
        let picked = select(files, &["mp3", "ogg", "flac", "wav"]);
        assert_eq!(names(&picked), ["a.MP3", "b.ogg", "d.Flac", "f.wav"]);
    }

    #[test]
    fn names_sort_case_insensitively_and_ties_fall_back_to_the_exact_name() {
        let files = ["b.mp3", "B.mp3", "a.mp3", "A.mp3", "10.mp3", "2.mp3"].map(PathBuf::from);
        let picked = select(files, &["mp3"]);
        assert_eq!(names(&picked), ["10.mp3", "2.mp3", "A.mp3", "a.mp3", "B.mp3", "b.mp3"]);
    }

    #[test]
    fn the_order_is_by_file_name_and_not_by_folder() {
        let files = [PathBuf::from("a/two.mp3"), PathBuf::from("z/one.mp3")];
        assert_eq!(select(files, &["mp3"]), [PathBuf::from("z/one.mp3"), PathBuf::from("a/two.mp3")]);
    }

    #[test]
    fn no_files_or_no_matches_give_an_empty_list() {
        assert!(select(Vec::new(), &["mp3"]).is_empty());
        assert!(select([PathBuf::from("x.txt")], &["mp3"]).is_empty());
    }
}
