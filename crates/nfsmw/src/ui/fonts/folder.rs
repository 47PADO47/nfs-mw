//! The user font folder: the files that hold custom fonts, by tier. Rule: `docs/specs/custom-fonts.md`, section 2.

use std::path::{Path, PathBuf};

use super::tier::Tier;

/// One font file and the tier it is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontFile {
    pub tier: Tier,
    pub path: PathBuf,
}

/// The font files under `dir`: the files of `high/`, `mid/` and `low/` (each folder in file-name order), then the files
/// directly in `dir`, which count as Mid. Subfolders of a tier folder are not searched. A missing folder holds no files.
pub fn scan(dir: &Path) -> Vec<FontFile> {
    let mut files: Vec<FontFile> = Tier::ALL.into_iter().flat_map(|tier| list(&dir.join(tier.name()), tier)).collect();
    files.extend(list(dir, Tier::Mid));
    files
}

/// The files directly in `dir`, sorted by file name without case.
fn list(dir: &Path, tier: Tier) -> Vec<FontFile> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.is_file()).collect();
    paths.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()));
    paths.into_iter().map(|path| FontFile { tier, path }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_folder_holds_no_fonts() {
        assert!(scan(Path::new("definitely/not/a/font/folder")).is_empty());
    }

    #[test]
    fn tier_folders_come_first_then_loose_files_as_mid() {
        let dir = std::env::temp_dir().join(format!("nfsmw-font-scan-{}", std::process::id()));
        for sub in ["high", "mid", "low"] {
            std::fs::create_dir_all(dir.join(sub)).unwrap();
        }
        std::fs::write(dir.join("high").join("b.ffn"), b"x").unwrap();
        std::fs::write(dir.join("high").join("A.ffn"), b"x").unwrap();
        std::fs::write(dir.join("low").join("font_mw_body.ffn"), b"x").unwrap();
        std::fs::write(dir.join("mid").join("font_x.ffn"), b"x").unwrap();
        std::fs::write(dir.join("loose.ffn"), b"x").unwrap();
        std::fs::create_dir_all(dir.join("mid").join("nested")).unwrap();
        std::fs::write(dir.join("mid").join("nested").join("hidden.ffn"), b"x").unwrap();

        let names: Vec<(Tier, String)> = scan(&dir)
            .into_iter()
            .map(|f| (f.tier, f.path.file_name().unwrap().to_string_lossy().into_owned()))
            .collect();
        std::fs::remove_dir_all(&dir).unwrap();

        assert_eq!(
            names,
            [
                (Tier::High, "A.ffn".to_owned()),
                (Tier::High, "b.ffn".to_owned()),
                (Tier::Mid, "font_x.ffn".to_owned()),
                (Tier::Low, "font_mw_body.ffn".to_owned()),
                (Tier::Mid, "loose.ffn".to_owned()),
            ]
        );
    }
}
