//! Scans the mod roots: one folder per mod, read in name order, checked against the policy and the host's offer
//! (ADR 0005 section 3). Nothing here opens a library; that is the loader's job.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::abi::{ABI_VERSION, AbiVersion, check_handshake};

use super::manifest::{self, MANIFEST_FILE, Manifest, ManifestError};
use super::offer::HostOffer;
use super::policy::Policy;
use super::reject::{RejectReason, Rejected};

/// A mod that passed discovery: its manifest, its folder and the library file for this platform, if it has one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// The checked manifest.
    pub manifest: Manifest,
    /// The mod folder.
    pub dir: PathBuf,
    /// The library file, when the manifest names one for this platform. `None` for a content-only mod.
    pub library: Option<PathBuf>,
}

/// What discovery found: the mods that may go on to ordering, and the ones refused with a reason.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Discovery {
    /// Mods to order and load. Sorted by id after duplicates are resolved.
    pub candidates: Vec<Candidate>,
    /// Mods refused here, in scan order.
    pub rejected: Vec<Rejected>,
}

/// Scan `roots` for mods. With [`Policy::Off`] nothing is scanned at all. A root that does not exist is not an error:
/// a fresh install has no mods folder yet.
pub fn discover(roots: &[PathBuf], policy: &Policy, offer: &HostOffer) -> Discovery {
    if *policy == Policy::Off {
        return Discovery::default();
    }
    let mut found = Discovery::default();
    for root in roots {
        scan_root(root, policy, offer, &mut found);
    }
    resolve_duplicates(&mut found);
    found
}

/// Check a manifest against the host's offer: the game, the ABI, and the required interfaces. Dependencies are not
/// checked here; they need the other mods (see `order`).
pub fn check_offer(manifest: &Manifest, offer: &HostOffer) -> Result<(), RejectReason> {
    if !manifest.games.supports(&offer.game_id) {
        return Err(RejectReason::GameUnsupported { host: offer.game_id.clone() });
    }
    if check_handshake(ABI_VERSION, manifest.api).is_err() {
        return Err(RejectReason::AbiNotOffered { module: abi_text(manifest.api), host: abi_text(ABI_VERSION) });
    }
    for required in &manifest.required_interfaces {
        let Some(offered) = offer.interface(&required.id) else {
            return Err(RejectReason::InterfaceMissing { id: required.id.clone() });
        };
        if offered.major != required.version.major || offered.minor < required.version.minor {
            return Err(RejectReason::InterfaceVersion {
                id: required.id.clone(),
                required: required.version.to_string(),
                offered: offered.to_string(),
            });
        }
    }
    Ok(())
}

fn abi_text(version: AbiVersion) -> String {
    format!("{}.{}", version.major, version.minor)
}

fn scan_root(root: &Path, policy: &Policy, offer: &HostOffer, out: &mut Discovery) {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) => {
            if error.kind() != io::ErrorKind::NotFound {
                log::warn!("cannot read the mods folder {}: {error}", root.display());
            }
            return;
        }
    };
    let mut dirs: Vec<PathBuf> =
        entries.filter_map(Result::ok).map(|entry| entry.path()).filter(|path| path.is_dir()).collect();
    dirs.sort();
    for dir in &dirs {
        scan_folder(dir, policy, offer, out);
    }
}

fn scan_folder(dir: &Path, policy: &Policy, offer: &HostOffer, out: &mut Discovery) {
    let folder = folder_name(dir);
    let text = match fs::read_to_string(dir.join(MANIFEST_FILE)) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return refuse(out, None, folder, dir, RejectReason::ManifestMissing);
        }
        Err(error) => {
            return refuse(out, None, folder, dir, RejectReason::Manifest(ManifestError::Read(error.to_string())));
        }
    };
    let manifest = match manifest::parse(&text) {
        Ok(manifest) => manifest,
        Err(error) => return refuse(out, None, folder, dir, RejectReason::Manifest(error)),
    };
    let id = Some(manifest.id.clone());
    if !policy.allows(&manifest.id) {
        return refuse(out, id, folder, dir, RejectReason::NotAllowed);
    }
    if let Err(reason) = check_offer(&manifest, offer) {
        return refuse(out, id, folder, dir, reason);
    }
    let library = match library_path(&manifest, dir) {
        Ok(library) => library,
        Err(reason) => return refuse(out, id, folder, dir, reason),
    };
    out.candidates.push(Candidate { manifest, dir: dir.to_owned(), library });
}

/// The library file for this platform. A mod without a name for this platform is content-only and has none.
fn library_path(manifest: &Manifest, dir: &Path) -> Result<Option<PathBuf>, RejectReason> {
    let Some(file) = manifest.library.file_for(std::env::consts::OS) else {
        return Ok(None);
    };
    let path = dir.join(file);
    if !path.is_file() {
        return Err(RejectReason::LibraryMissing { file: file.to_owned() });
    }
    Ok(Some(path))
}

/// Keep one candidate per id: the highest version. An equal version keeps the one found first (roots in order, then
/// folder names in order). The others are reported as duplicates.
fn resolve_duplicates(found: &mut Discovery) {
    let mut kept: BTreeMap<String, Candidate> = BTreeMap::new();
    let mut losers: Vec<(Candidate, String)> = Vec::new();
    for candidate in std::mem::take(&mut found.candidates) {
        let id = candidate.manifest.id.clone();
        let new_version = candidate.manifest.version.clone();
        let current_version = kept.get(&id).map(|current| current.manifest.version.clone());
        match current_version {
            None => {
                kept.insert(id, candidate);
            }
            Some(current_version) if new_version > current_version => {
                if let Some(previous) = kept.insert(id, candidate) {
                    losers.push((previous, new_version.to_string()));
                }
            }
            Some(current_version) => losers.push((candidate, current_version.to_string())),
        }
    }
    for (loser, kept_version) in losers {
        let folder = folder_name(&loser.dir);
        found.rejected.push(Rejected {
            id: Some(loser.manifest.id.clone()),
            folder,
            dir: loser.dir,
            reason: RejectReason::Duplicate { kept: kept_version },
        });
    }
    found.candidates = kept.into_values().collect();
}

fn refuse(out: &mut Discovery, id: Option<String>, folder: String, dir: &Path, reason: RejectReason) {
    out.rejected.push(Rejected { id, folder, dir: dir.to_owned(), reason });
}

fn folder_name(dir: &Path) -> String {
    dir.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default()
}

#[cfg(test)]
mod tests;
