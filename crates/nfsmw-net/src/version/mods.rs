//! The mod manifest a peer sends and how a server compares it (MP-63, `security-gameplay.md` section 7). The mod API is
//! milestone 12, so every list is empty today and the check passes; the comparison is ready for when it is not.

use crate::message::DecodeError;
use crate::message::macros::{wire_enum, wire_struct};
use crate::message::{LimitError, List, Text, VersionText};

/// Most entries in a mod manifest.
pub const MAX_MODS: usize = 64;

/// A mod manifest: at most [`MAX_MODS`] entries.
pub type ModList = List<ModEntry, MAX_MODS>;

wire_enum! {
    /// What a mod contains.
    pub enum ModKind {
        /// A native library (and maybe content).
        Code = 0,
        /// Data files only.
        Content = 1,
    }
}

wire_struct! {
    /// One mod of a manifest: its id (1 to 64 of `[a-z0-9_.-]`), its version, the SHA-256 of its files, its kind, and
    /// whether its manifest marks it client-only (HUD, audio, post-processing), which is never compared.
    pub struct ModEntry {
        pub id: Text<1, 64>,
        pub version: VersionText,
        pub hash: [u8; 32],
        pub kind: ModKind,
        pub client_only: bool,
    }
    check(entry) {
        match is_mod_id(&entry.id) {
            true => Ok(()),
            false => Err(DecodeError::BadText),
        }
    }
}

impl ModEntry {
    /// A manifest entry, or [`LimitError`] for an id outside `[a-z0-9_.-]{1,64}` or a bad version string.
    pub fn new(id: &str, version: &str, hash: [u8; 32], kind: ModKind, client_only: bool) -> Result<Self, LimitError> {
        if !is_mod_id(id) {
            return Err(LimitError);
        }
        Ok(Self { id: Text::new(id)?, version: VersionText::new(version)?, hash, kind, client_only })
    }

    /// Whether the mod can change the simulation, so peers must agree on it.
    pub fn affects_sim(&self) -> bool {
        !self.client_only
    }
}

/// Whether `id` is 1 to 64 characters of `[a-z0-9_.-]`.
pub fn is_mod_id(id: &str) -> bool {
    let valid = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '.' | '-');
    (1..=64).contains(&id.len()) && id.chars().all(valid)
}

/// How strictly a server compares mod lists (`security-gameplay.md` section 7.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ModPolicy {
    /// No sim-affecting mods at all: the vanilla set.
    Off,
    /// Every mod the server requires must be present with the same version and hash; extra mods are ignored. The LAN
    /// default.
    #[default]
    Lenient,
    /// The sim-affecting sets must be identical. The online default.
    Strict,
}

/// The readable difference between two mod lists: ids the client lacks, has in excess, or has in another version or
/// build.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ModMismatch {
    pub missing: Vec<String>,
    pub extra: Vec<String>,
    pub different: Vec<String>,
}

impl ModMismatch {
    /// The number of differences, at most 255: the `detail` code of a `ModMismatch` rejection.
    pub fn detail(&self) -> u16 {
        let total = self.missing.len() + self.extra.len() + self.different.len();
        total.min(255) as u16
    }

    fn is_empty(&self) -> bool {
        self.missing.is_empty() && self.extra.is_empty() && self.different.is_empty()
    }
}

/// Compares the client's manifest `offered` with the server's `required` under `policy`. Client-only entries on either
/// side are ignored.
pub fn compare_mods(required: &[ModEntry], offered: &[ModEntry], policy: ModPolicy) -> Result<(), ModMismatch> {
    let required: Vec<&ModEntry> = required.iter().filter(|entry| entry.affects_sim()).collect();
    let offered: Vec<&ModEntry> = offered.iter().filter(|entry| entry.affects_sim()).collect();
    let mut diff = ModMismatch::default();
    if policy == ModPolicy::Off {
        diff.extra = offered.iter().map(|entry| entry.id.to_string()).collect();
        return finish(diff);
    }
    for want in &required {
        match offered.iter().find(|have| have.id == want.id) {
            None => diff.missing.push(want.id.to_string()),
            Some(have) if have.version != want.version || have.hash != want.hash || have.kind != want.kind => {
                diff.different.push(want.id.to_string())
            }
            Some(_) => {}
        }
    }
    if policy == ModPolicy::Strict {
        diff.extra = offered
            .iter()
            .filter(|have| !required.iter().any(|want| want.id == have.id))
            .map(|have| have.id.to_string())
            .collect();
    }
    finish(diff)
}

fn finish(diff: ModMismatch) -> Result<(), ModMismatch> {
    if diff.is_empty() {
        return Ok(());
    }
    Err(diff)
}
