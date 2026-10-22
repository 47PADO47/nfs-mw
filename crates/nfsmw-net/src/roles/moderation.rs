//! Session bans on the server and the mute list of one client.

/// Most bans a session keeps; the oldest is forgotten past this.
pub const MAX_BANS: usize = 256;
/// Most players a client's mute list holds.
pub const MAX_LOCAL_MUTES: usize = 64;

/// One ban: the player and the time it ends, `None` for the rest of the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Ban {
    player_id: [u8; 16],
    until_ms: Option<u64>,
}

/// The session's bans, by player id (the hash of the identity key, so a new name does not lift a ban). In memory only:
/// a ban ends with the session unless the server crate stores it.
#[derive(Debug, Clone, Default)]
pub struct Bans {
    bans: Vec<Ban>,
}

impl Bans {
    /// No bans.
    pub fn new() -> Self {
        Self::default()
    }

    /// Bans `player_id` from `now_ms` for `minutes`, or for the rest of the session when `minutes` is 0. A second ban
    /// of the same player replaces the first. Past [`MAX_BANS`] the oldest ban is forgotten.
    pub fn ban(&mut self, player_id: [u8; 16], minutes: u32, now_ms: u64) {
        self.bans.retain(|ban| ban.player_id != player_id);
        let until_ms = (minutes > 0).then(|| now_ms.saturating_add(u64::from(minutes) * 60_000));
        if self.bans.len() >= MAX_BANS {
            self.bans.remove(0);
        }
        self.bans.push(Ban { player_id, until_ms });
    }

    /// Lifts the ban of `player_id`. `false` when there was none.
    pub fn lift(&mut self, player_id: [u8; 16]) -> bool {
        let before = self.bans.len();
        self.bans.retain(|ban| ban.player_id != player_id);
        self.bans.len() != before
    }

    /// Whether `player_id` is banned at `now_ms`.
    pub fn is_banned(&self, player_id: [u8; 16], now_ms: u64) -> bool {
        self.bans.iter().any(|ban| ban.player_id == player_id && ban.until_ms.is_none_or(|until| now_ms < until))
    }

    /// Forgets bans that ended before `now_ms`.
    pub fn expire(&mut self, now_ms: u64) {
        self.bans.retain(|ban| ban.until_ms.is_none_or(|until| now_ms < until));
    }

    /// Bans kept, ended or not.
    pub fn len(&self) -> usize {
        self.bans.len()
    }

    /// Whether no ban is kept.
    pub fn is_empty(&self) -> bool {
        self.bans.is_empty()
    }
}

/// A client's own mute list (MP-42): chat and quick messages from these players are not shown on this client. The
/// server never learns it.
#[derive(Debug, Clone, Default)]
pub struct LocalMutes {
    muted: Vec<[u8; 16]>,
}

impl LocalMutes {
    /// Nobody muted.
    pub fn new() -> Self {
        Self::default()
    }

    /// Mutes or unmutes `player_id`. `false` when the list is full ([`MAX_LOCAL_MUTES`]) and the player was not added.
    pub fn set(&mut self, player_id: [u8; 16], muted: bool) -> bool {
        self.muted.retain(|other| *other != player_id);
        if !muted {
            return true;
        }
        if self.muted.len() >= MAX_LOCAL_MUTES {
            return false;
        }
        self.muted.push(player_id);
        true
    }

    /// Whether a line from `player_id` is shown.
    pub fn shows(&self, player_id: [u8; 16]) -> bool {
        !self.muted.contains(&player_id)
    }
}
