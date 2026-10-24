//! The real [`Link`]: the local Discord client's IPC pipe, through the `discord-rich-presence` crate.

use discord_rich_presence::activity::{Activity, Timestamps};
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};

use super::presence::Presence;
use super::worker::Link;

/// A connection to the Discord client of this machine, for the application `client_id`.
pub struct IpcLink(DiscordIpcClient);

impl IpcLink {
    pub fn new(client_id: &str) -> Self {
        Self(DiscordIpcClient::new(client_id))
    }
}

impl Link for IpcLink {
    fn connect(&mut self) -> Result<(), String> {
        DiscordIpc::connect(&mut self.0).map_err(|e| e.to_string())
    }

    fn set_activity(&mut self, presence: &Presence) -> Result<(), String> {
        DiscordIpc::set_activity(&mut self.0, activity_of(presence)).map_err(|e| e.to_string())
    }

    fn clear_activity(&mut self) {
        if let Err(e) = DiscordIpc::clear_activity(&mut self.0) {
            log::debug!("discord: the activity was not cleared ({e})");
        }
    }

    fn close(&mut self) {
        if let Err(e) = DiscordIpc::close(&mut self.0) {
            log::debug!("discord: the connection was not closed cleanly ({e})");
        }
    }
}

/// The wire form of a presence: the status line, and the start time when there is one.
fn activity_of(presence: &Presence) -> Activity<'_> {
    let activity = Activity::new().details(presence.details.as_str());
    let Some(started) = presence.started_at_ms else {
        return activity;
    };
    activity.timestamps(Timestamps::new().start(started))
}
