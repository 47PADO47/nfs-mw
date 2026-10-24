//! What the presence says: the game's state, and the start of the session when the time played is shown.

use bevy_ecs::prelude::Resource;

/// What the player is doing, as Discord shows it. Career, race and pursuit states are added here as the game
/// reports them; the presence follows the resource.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum PresenceStatus {
    /// Playing the game (the menus and the open world alike).
    #[default]
    Playing,
}

impl PresenceStatus {
    /// The line Discord shows under the application's name, in the current language.
    pub fn details(self) -> String {
        let id = match self {
            Self::Playing => "menus-presence-playing",
        };
        crate::i18n::text(id, &crate::i18n::Args::new())
    }
}

/// The activity to show: the status line, and the Unix time (in milliseconds) the session started, which Discord
/// turns into "elapsed" time. `None` shows the status line alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Presence {
    pub details: String,
    pub started_at_ms: Option<i64>,
}

/// The presence the settings ask for: nothing when Discord is off, else the status with the elapsed time when that
/// is on too.
pub fn presence_for(status: PresenceStatus, discord: bool, time_played: bool, started_at_ms: i64) -> Option<Presence> {
    if !discord {
        return None;
    }
    Some(Presence { details: status.details(), started_at_ms: time_played.then_some(started_at_ms) })
}
