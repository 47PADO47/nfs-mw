//! What the host offers a mod: its game, its version and the interfaces it can provide.
//!
//! The offer is checked against manifests before anything is loaded (`requires`, `games.supported`). The interface
//! tables themselves are attached by the game's facade; until one is attached, the host answers a query with null
//! (ADR 0005 section 3, "absent").

use super::manifest::InterfaceVersion;

/// The host's side of the compatibility check.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HostOffer {
    /// The game id, for example `mygame`. Compared with `games.supported`.
    pub game_id: String,
    /// The game version, for diagnostics.
    pub game_version: String,
    /// The interfaces the host offers, with their versions.
    pub interfaces: Vec<InterfaceOffer>,
}

/// One interface the host offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterfaceOffer {
    /// Interface id, for example `bb.commands`.
    pub id: String,
    /// The version the host implements.
    pub version: InterfaceVersion,
}

impl HostOffer {
    /// An offer for `game_id` with no interfaces yet.
    pub fn new(game_id: &str, game_version: &str) -> Self {
        HostOffer { game_id: game_id.to_owned(), game_version: game_version.to_owned(), interfaces: Vec::new() }
    }

    /// Add an interface to the offer.
    pub fn with_interface(mut self, id: &str, version: InterfaceVersion) -> Self {
        self.interfaces.push(InterfaceOffer { id: id.to_owned(), version });
        self
    }

    /// The version the host offers for `id`, or `None` when it offers none.
    pub fn interface(&self, id: &str) -> Option<InterfaceVersion> {
        self.interfaces.iter().find(|offer| offer.id == id).map(|offer| offer.version)
    }
}
