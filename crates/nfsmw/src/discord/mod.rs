//! Discord Rich Presence: shows the game (and, optionally, the time played) on the user's Discord profile.
//!
//! The `discord` and `discord_time_played` settings switch it on and off, live. The Discord client is talked to on
//! a thread of its own ([`worker`]), so the frame never waits for it. Without a running Discord, nothing is shown
//! and nothing is reported beyond a debug log line.

mod link;
mod plugin;
mod presence;
mod worker;

#[cfg(test)]
mod tests;

pub use plugin::DiscordPlugin;

/// The Discord application that owns the presence (its client id, from the Discord developer portal).
///
/// PLACEHOLDER: no Discord application has been created for this project yet. The owner must create one at
/// <https://discord.com/developers/applications>, copy its Application ID here (the digits only, as a string),
/// and give it a name and icon there: Discord shows the application's name next to "Playing". Until a real id is
/// pasted in, the presence is not started at all, so no connection to Discord is attempted.
pub const DISCORD_APP_ID: &str = "PLACEHOLDER_DISCORD_APP_ID";

/// Whether [`DISCORD_APP_ID`] is a real Discord application id, not the placeholder.
pub fn app_id_is_set() -> bool {
    is_app_id(DISCORD_APP_ID)
}

/// A Discord application id is a non-empty run of digits (a snowflake).
fn is_app_id(id: &str) -> bool {
    !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit())
}
