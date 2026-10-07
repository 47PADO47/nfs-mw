use std::path::PathBuf;

/// Where a game directory came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    CommandLine,
    /// The named environment variable.
    Environment(&'static str),
    DotEnv(PathBuf),
    ConfigFile(PathBuf),
    /// `HIVE\key\value`.
    Registry(String),
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CommandLine => write!(f, "--game-dir"),
            Self::Environment(var) => write!(f, "${var}"),
            Self::DotEnv(p) | Self::ConfigFile(p) => write!(f, "{}", p.display()),
            Self::Registry(k) => write!(f, "registry {k}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Discovered {
    pub path: PathBuf,
    pub source: Source,
}
