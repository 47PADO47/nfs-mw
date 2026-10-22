//! Host, admin and player permissions (MP-26), the session's bans, and the local mute list (MP-42).
//!
//! The server checks every command against the sender's [`Rank`] with [`permits`] and the target's rank with
//! [`may_target`]; the client's UI only hides buttons. A refused command from a client is a spoof (a normal UI never
//! sends it) and the caller counts it as a violation (`security-gameplay.md` section 5.2).
//!
//! - [`Rank`]: `Player`, `Admin`, `Host`. The host is the player whose game runs the listen server; a dedicated server
//!   has no host and its admins come from its config.
//! - [`Command`] and [`permits`]: what each rank may do. Only the host names admins.
//! - [`may_target`]: a moderation command must target a lower rank, never the sender itself.
//! - [`Bans`]: bans for a number of minutes or for the rest of the session, by player id, bounded.
//! - [`LocalMutes`]: the client's own mute list. Server mutes (the host's) live on the lobby member and stop the chat
//!   on the server; a local mute only hides lines on one client.

mod moderation;

#[cfg(test)]
mod tests;

pub use moderation::{Bans, LocalMutes, MAX_BANS, MAX_LOCAL_MUTES};

/// What a member may do in the session, lowest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Rank {
    #[default]
    Player,
    Admin,
    Host,
}

/// A command that needs a rank.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Command {
    /// Send `UpdateSettings`.
    ChangeSettings,
    /// Start the countdown.
    Start,
    /// Cancel the countdown.
    Cancel,
    /// Turn "everyone wait" on or off (MP-54).
    Hold,
    Kick,
    Ban,
    /// Server-enforced mute of a member's chat.
    Mute,
    /// End the race and start it again.
    Restart,
    ChangeMode,
    /// Make a member an admin or a player again.
    SetAdmin,
}

/// Whether `rank` may run `command`: the host everything, an admin everything but naming admins, a player nothing.
pub fn permits(rank: Rank, command: Command) -> bool {
    match rank {
        Rank::Host => true,
        Rank::Admin => command != Command::SetAdmin,
        Rank::Player => false,
    }
}

/// Whether a member of rank `actor` may kick, ban or mute a member of rank `target`: only a strictly lower rank, so an
/// admin cannot remove the host or another admin. Targeting oneself is refused by the caller (it knows the slots).
pub fn may_target(actor: Rank, target: Rank) -> bool {
    actor > target
}
