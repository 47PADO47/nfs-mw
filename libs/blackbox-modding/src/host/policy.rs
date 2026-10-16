//! Which mods may load: the `mods = off | allowlist | all` setting (ADR 0005 section 6).

use std::collections::BTreeSet;

/// The setting that decides which discovered mods may load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Policy {
    /// `mods = off`, or `--no-mods`. Nothing is scanned and nothing is loaded.
    Off,
    /// `mods = allowlist`: only the listed ids may load. Every other mod is reported as not allowed.
    Allowlist(BTreeSet<String>),
    /// `mods = all`: every mod in the folders may load.
    All,
}

impl Policy {
    /// Whether the policy lets the mod `id` load. `Off` allows nothing.
    pub fn allows(&self, id: &str) -> bool {
        match self {
            Policy::Off => false,
            Policy::Allowlist(ids) => ids.contains(id),
            Policy::All => true,
        }
    }
}
