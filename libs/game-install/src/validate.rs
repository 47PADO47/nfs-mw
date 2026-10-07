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
    /// Check the spec's required files and identify its executable.
    pub fn validate(&self, spec: &GameSpec) -> Validation {
        let missing = spec.required_files.iter().copied().filter(|f| !self.exists(f)).collect();
        let exe_sha256 = spec
            .executable
            .and_then(|exe| self.read(exe).ok())
            .map(|bytes| Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect::<String>());
        let exe_build =
            exe_sha256.as_deref().and_then(|h| spec.known_builds.iter().find(|b| b.sha256 == h).map(|b| b.label));
        Validation { missing, exe_sha256, exe_build }
    }
}

#[cfg(test)]
mod tests {
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
        let v = GameDir::open(&root).unwrap().validate(&SPEC);
        assert!(!v.is_usable());
        assert_eq!(v.missing, ["DATA/OTHER.BIN"]);
        assert!(v.exe_sha256.is_some());
        assert_eq!(v.exe_build, None);
        std::fs::remove_dir_all(root).unwrap();
    }
}
