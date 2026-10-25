//! Discord Rich Presence: shows the game (and, optionally, the time played) on the user's Discord profile.
//!
//! The `discord`, `discord_time_played` and `discord_join` settings switch it, the time played and the join invite on
//! and off, live. In a LAN game the presence carries a party: friends see its size and can ask to join, and Discord
//! hands the server's address to their game as the join secret (`presence::join_address`). The Discord client is
//! talked to on a thread of its own ([`worker`]), so the frame never waits for it. Without a running Discord, nothing
//! is shown and nothing is reported beyond a debug log line.

mod link;
mod plugin;
mod presence;
mod worker;

#[cfg(test)]
mod tests;

pub use plugin::{DiscordJoins, DiscordPlugin};
pub use presence::{LanGame, PresenceParty, PresenceStatus};

/// The Discord application that owns the presence (its client id, from the Discord developer portal).
///
/// The Discord application's Application ID, from <https://discord.com/developers/applications>. Discord shows the
/// application's name and icon next to "Playing". Without a real id the presence is not started at all.
pub const DISCORD_APP_ID: &str = "1563936724869058625";

/// Whether [`DISCORD_APP_ID`] is a real Discord application id, not the placeholder.
pub fn app_id_is_set() -> bool {
    is_app_id(DISCORD_APP_ID)
}

/// A Discord application id is a non-empty run of digits (a snowflake).
fn is_app_id(id: &str) -> bool {
    !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit())
}
