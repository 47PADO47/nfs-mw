//! Checking that a directory is a usable install.

use sha2::{Digest, Sha256};

use crate::{GameDir, GameSpec};

/// Result of [`GameDir::validate`].
#[derive(Debug, Clone)]
pub struct Validation {
    pub missing: Vec<&'static str>,
    /// SHA-256 of the spec's executable, if present.
    pub exe_sha256: Option<String>,
    /// Label of the matching [`crate::KnownBuild`].
    pub exe_build: Option<&'static str>,
}

impl Validation {
    pub fn is_usable(&self) -> bool {
        self.missing.is_empty()
    }
}

impl GameDir {
    /// The spec's required files that the install does not have.
    ///
    /// Cheap: it looks at file names only and never reads file contents, so the
    /// normal open path uses it. [`GameDir::validate`] returns the same list.
    pub fn missing_files(&self, spec: &GameSpec) -> Vec<&'static str> {
        spec.required_files.iter().copied().filter(|f| !self.exists(f)).collect()
    }

    /// Check the spec's required files and identify its executable.
    ///
    /// Reads and hashes the whole executable, so call it only where the exact
    /// build matters (for example `check-install`). Use [`GameDir::missing_files`]
    /// to check that the install is usable.
    pub fn validate(&self, spec: &GameSpec) -> Validation {
        let missing = self.missing_files(spec);
        let exe_sha256 =
            spec.executable.and_then(|exe| self.read(exe).ok()).map(|bytes| hex_lower(&Sha256::digest(&bytes)));
        let exe_build =
            exe_sha256.as_deref().and_then(|h| spec.known_builds.iter().find(|b| b.sha256 == h).map(|b| b.label));
        Validation { missing, exe_sha256, exe_build }
    }
}

/// Lower-case hex of `bytes`, written into one pre-sized buffer.
fn hex_lower(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut hex = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        hex.push(char::from(DIGITS[usize::from(b >> 4)]));
        hex.push(char::from(DIGITS[usize::from(b & 0x0f)]));
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::hex_lower;
    use crate::{GameDir, GameSpec};

    const SPEC: GameSpec = GameSpec {
        name: "Test Game",
        env_var: "TEST_GAME_DIR",
        app_dir: "test-game",
        registry: &[],
        required_files: &["DATA/MAIN.BIN", "DATA/OTHER.BIN"],
        executable: Some("game.exe"),
        known_builds: &[],
    };

    #[test]
    fn reports_missing_files_and_unknown_exe() {
        let root = std::env::temp_dir().join(format!("game-install-validate-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("Data")).unwrap();
        std::fs::write(root.join("Data/Main.bin"), b"x").unwrap();
        std::fs::write(root.join("GAME.EXE"), b"MZ").unwrap();
        let dir = GameDir::open(&root).unwrap();
        assert_eq!(dir.missing_files(&SPEC), ["DATA/OTHER.BIN"]);
        let v = dir.validate(&SPEC);
        assert!(!v.is_usable());
        assert_eq!(v.missing, ["DATA/OTHER.BIN"]);
        assert!(v.exe_sha256.is_some());
        assert_eq!(v.exe_build, None);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn hex_is_lower_case_and_zero_padded() {
        assert_eq!(hex_lower(&[0x00, 0x0f, 0xa0, 0xff]), "000fa0ff");
        assert_eq!(hex_lower(&[]), "");
    }
}
