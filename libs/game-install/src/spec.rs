//! What the library needs to know about one game.

/// A registry value holding the install directory (Windows only).
#[derive(Debug, Clone, Copy)]
pub struct RegistryValue {
    /// Key path under `HKLM` / `HKCU`, e.g. `SOFTWARE\EA GAMES\Need for Speed Most Wanted`.
    /// Read from the 32-bit view, where 32-bit installers write.
    pub key: &'static str,
    pub value: &'static str,
}

/// A known build of the game's executable.
#[derive(Debug, Clone, Copy)]
pub struct KnownBuild {
    /// Lower-case hex SHA-256 of the executable.
    pub sha256: &'static str,
    pub label: &'static str,
}

/// Everything game-specific about finding and checking an install.
#[derive(Debug, Clone, Copy)]
pub struct GameSpec {
    /// Display name, used in error messages.
    pub name: &'static str,
    /// Environment variable (and `.env` key) naming the install directory.
    pub env_var: &'static str,
    /// Directory name for the per-user config file.
    pub app_dir: &'static str,
    pub registry: &'static [RegistryValue],
    /// Files that must exist (relative, matched case-insensitively).
    pub required_files: &'static [&'static str],
    /// The executable to identify, if any (relative path).
    pub executable: Option<&'static str>,
    pub known_builds: &'static [KnownBuild],
}
