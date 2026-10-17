//! Which mods may load: the `mods = off | allowlist | all` setting (ADR 0005 section 6), and the mods the player switched off.

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
    /// `mods = all` with some mods switched off (`mods_disabled`): every mod in the folders except the listed ids.
    Except(BTreeSet<String>),
}

impl Policy {
    /// Whether the policy lets the mod `id` load. `Off` allows nothing.
    pub fn allows(&self, id: &str) -> bool {
        match self {
            Policy::Off => false,
            Policy::Allowlist(ids) => ids.contains(id),
            Policy::All => true,
            Policy::Except(ids) => !ids.contains(id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(ids: &[&str]) -> BTreeSet<String> {
        ids.iter().map(|id| (*id).to_owned()).collect()
    }

    #[test]
    fn except_allows_every_mod_but_the_listed_ones() {
        let policy = Policy::Except(set(&["off.mod"]));
        assert!(!policy.allows("off.mod"));
        assert!(policy.allows("other.mod"));
        assert!(Policy::All.allows("off.mod"), "plain `all` allows every id");
    }

    #[test]
    fn allowlist_and_off_are_unchanged() {
        assert!(Policy::Allowlist(set(&["on.mod"])).allows("on.mod"));
        assert!(!Policy::Allowlist(set(&["on.mod"])).allows("other.mod"));
        assert!(!Policy::Off.allows("on.mod"));
    }
}
