//! Why a mod is not loaded. Each rejected mod carries one reason; the log and the `mods` console command show it.

use std::path::PathBuf;

use super::discovery::Candidate;
use super::manifest::ManifestError;

/// One reason a mod is not loaded (ADR 0005 section 3).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RejectReason {
    /// The manifest does not parse or fails validation.
    #[error("{0}")]
    Manifest(ManifestError),
    /// The folder has no `mod.toml`.
    #[error("the folder has no mod.toml")]
    ManifestMissing,
    /// The policy (`mods = allowlist`) does not list the mod.
    #[error("not in mods.allowed")]
    NotAllowed,
    /// Another folder holds the same id with a higher version, which is kept.
    #[error("duplicate id; version {kept} is kept")]
    Duplicate { kept: String },
    /// The mod does not declare support for this game.
    #[error("the mod supports other games, not {host}")]
    GameUnsupported { host: String },
    /// The ABI major differs, or the mod was built for a newer minor than this host.
    #[error("the mod is built for ABI {module}, this host speaks {host}")]
    AbiNotOffered { module: String, host: String },
    /// A required interface is not offered at all.
    #[error("required interface {id} is not offered")]
    InterfaceMissing { id: String },
    /// A required interface is offered in another major, or in a minor below the one the mod needs.
    #[error("required interface {id} is offered as {offered}, the mod needs {required}")]
    InterfaceVersion { id: String, required: String, offered: String },
    /// A dependency is not present.
    #[error("dependency {id} is missing")]
    DependencyMissing { id: String },
    /// A dependency is present in a version the requirement does not accept.
    #[error("dependency {id} is version {found}, the mod needs {requirement}")]
    DependencyVersion { id: String, requirement: String, found: String },
    /// A dependency was itself rejected.
    #[error("dependency {id} was rejected")]
    DependencyRejected { id: String },
    /// The mod is part of a dependency or load-order cycle.
    #[error("dependency cycle: {}", members.join(" -> "))]
    Cycle { members: Vec<String> },
    /// The manifest names a library for this platform, but the file is not in the folder.
    #[error("library {file} is not in the folder")]
    LibraryMissing { file: String },
    /// The library was found, but could not be loaded or did not pass the handshake.
    #[error("{0}")]
    LoadFailed(String),
}

/// A mod that is not loaded, with its reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rejected {
    /// The mod id, when the manifest was read far enough to know it.
    pub id: Option<String>,
    /// The folder name, always known.
    pub folder: String,
    /// The folder path.
    pub dir: PathBuf,
    /// Why it is not loaded.
    pub reason: RejectReason,
}

impl Rejected {
    /// The rejection of a candidate that passed discovery but failed later, with its id and folder.
    pub fn of_candidate(candidate: Candidate, reason: RejectReason) -> Self {
        let folder = candidate.dir.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
        Rejected { id: Some(candidate.manifest.id), folder, dir: candidate.dir, reason }
    }

    /// The id when known, else the folder name. Used in log lines.
    pub fn name(&self) -> &str {
        match &self.id {
            Some(id) => id,
            None => &self.folder,
        }
    }

    /// One line for the log: the name and the reason.
    pub fn summary(&self) -> String {
        format!("mod {} rejected: {}", self.name(), self.reason)
    }
}
