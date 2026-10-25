//! What the presence says: the game's state, the LAN game the player is in, and the start of the session when the time
//! played is shown.

use std::net::SocketAddr;

use bevy_ecs::prelude::Resource;

/// What the player is doing, as Discord shows it. Career, race and pursuit states are added here as the game
/// reports them; the presence follows the resource.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum PresenceStatus {
    /// Playing the game (the menus and the open world alike).
    #[default]
    Playing,
    /// Seated in a LAN game (its lobby or its world).
    Lan,
}

impl PresenceStatus {
    /// The line Discord shows under the application's name, in the current language.
    pub fn details(self) -> String {
        let id = match self {
            Self::Playing => "menus-presence-playing",
            Self::Lan => "menus-presence-lan",
        };
        crate::i18n::text(id, &crate::i18n::Args::new())
    }
}

/// The LAN game the player is seated in, for the party Discord shows next to the activity. `None` outside a game. The
/// game's network fills it in (`crate::net`); the presence follows the resource.
#[derive(Resource, Debug, Default, Clone, PartialEq, Eq)]
pub struct PresenceParty(pub Option<LanGame>);

/// A LAN game as the party shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanGame {
    /// Where the game's server listens, as a friend reaches it. The party and the join invite are made from it.
    pub host: SocketAddr,
    pub players: u8,
    pub max_players: u8,
    /// A friend may join now: the room has no password, a free seat, and no race is running.
    pub open: bool,
}

/// The party of the presence: Discord shows its size, and "Ask to Join" when it carries a join secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Party {
    pub id: String,
    pub size: (u32, u32),
    /// The text Discord hands to the friend's game when they join: the server's address.
    pub join: Option<String>,
}

impl Party {
    fn of(game: &LanGame) -> Self {
        Self {
            id: format!("nfsmw:{}", game.host),
            size: (u32::from(game.players), u32::from(game.max_players.max(game.players))),
            join: game.open.then(|| game.host.to_string()),
        }
    }
}

/// The server a join secret names. The secret came from another player's Discord: anything but a socket address is
/// refused, so a name is never looked up and nothing else is acted on.
pub fn join_address(secret: &str) -> Option<SocketAddr> {
    secret.trim().parse().ok()
}

/// The activity to show: the status line, the Unix time (in milliseconds) the session started, which Discord turns
/// into "elapsed" time, and the party. `None` shows the status line alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Presence {
    pub details: String,
    pub started_at_ms: Option<i64>,
    pub party: Option<Party>,
}

/// What the settings allow of the presence: it shows at all, with the time played, with a party friends can join.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Allowed {
    pub discord: bool,
    pub time_played: bool,
    pub join: bool,
}

/// The presence the settings ask for: nothing when Discord is off, else the status with the elapsed time and the LAN
/// game's party when those are on too.
pub fn presence_for(
    status: PresenceStatus,
    game: Option<&LanGame>,
    allowed: Allowed,
    started_at_ms: i64,
) -> Option<Presence> {
    if !allowed.discord {
        return None;
    }
    let party = game.filter(|_| allowed.join).map(Party::of);
    Some(Presence { details: status.details(), started_at_ms: allowed.time_played.then_some(started_at_ms), party })
}
