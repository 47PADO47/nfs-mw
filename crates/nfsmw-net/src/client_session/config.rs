//! What the game tells a session before it connects.

use crate::drop::RESUME_WINDOW_MS;
use crate::loadout::Loadout;
use crate::message::{CAP_CLUTCH, CAP_SPECTATE_ONLY, Name};
use crate::reconcile::Tuning;
use crate::version::{DataId, ModList, PROTO_MINOR, SimBuild};

/// The settings of a [`super::ClientSession`].
#[derive(Debug, Clone, PartialEq)]
pub struct SessionConfig {
    /// The player's display name.
    pub name: Name,
    /// The car the player asks for.
    pub loadout: Loadout,
    /// The client's mod list (empty until the mod API exists).
    pub mods: ModList,
    /// This build's protocol minor.
    pub proto_minor: u16,
    pub sim_build: SimBuild,
    pub data_id: DataId,
    /// `Auth` capability bits ([`CAP_CLUTCH`], `CAP_WHEEL`, [`CAP_SPECTATE_ONLY`]).
    pub caps: u8,
    /// The room password, if the player typed one (normalised by the caller).
    pub password: Option<String>,
    /// The sync-model tuning; its snapshot rate is replaced by the server's at join.
    pub tuning: Tuning,
    /// How long after a lost connection the session tries to resume its slot, milliseconds.
    pub resume_window_ms: u64,
    /// The pause between two reconnect attempts, milliseconds.
    pub reconnect_interval_ms: u64,
    /// Chat lines and system notices the lobby mirror keeps.
    pub chat_lines: usize,
}

impl SessionConfig {
    /// A racer with `name` and `loadout`, on this build's protocol, LAN tuning and the 30 s resume window.
    pub fn new(name: Name, loadout: Loadout, sim_build: SimBuild, data_id: DataId) -> Self {
        Self {
            name,
            loadout,
            mods: ModList::empty(),
            proto_minor: PROTO_MINOR,
            sim_build,
            data_id,
            caps: 0,
            password: None,
            tuning: Tuning::lan(),
            resume_window_ms: RESUME_WINDOW_MS,
            reconnect_interval_ms: 1000,
            chat_lines: 64,
        }
    }

    /// The same, joining as a spectator only.
    pub fn spectator(mut self) -> Self {
        self.caps |= CAP_SPECTATE_ONLY;
        self
    }

    /// Whether the player drives with a clutch (copied into every input frame).
    pub fn uses_clutch(&self) -> bool {
        self.caps & CAP_CLUTCH != 0
    }

    /// Whether the player only spectates.
    pub fn spectate_only(&self) -> bool {
        self.caps & CAP_SPECTATE_ONLY != 0
    }
}
