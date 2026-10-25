//! The Bevy side: starts the presence thread and tells it what to show when the settings, the status or the LAN game
//! change, and hands the game the join secrets that friends' Discords send.

use std::net::SocketAddr;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Mutex, PoisonError};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use bevy_app::{App, Plugin, Update};
use bevy_ecs::prelude::*;

use super::link::IpcLink;
use super::presence::{Allowed, Presence, PresenceParty, PresenceStatus, join_address, presence_for};
use super::worker::{Command, Worker};
use super::{DISCORD_APP_ID, app_id_is_set};
use crate::app::FrameSet;
use crate::settings::Settings;

/// Adds Discord Rich Presence. It is always added: the `discord` setting is read every time it changes, so the
/// presence can be switched on and off while the game runs.
pub struct DiscordPlugin;

/// The link to the presence thread, and what was last asked of it.
#[derive(Resource)]
pub struct DiscordPresence {
    commands: Sender<Command>,
    /// The Unix time (in milliseconds) the session started: Discord counts the elapsed time from it.
    started_at_ms: i64,
    /// The last presence sent, `Some(None)` for hidden. `None` before anything was sent.
    sent: Option<Option<Presence>>,
}

impl Drop for DiscordPresence {
    /// The thread is not stopped by the channel closing (its links hold senders too), so it is told.
    fn drop(&mut self) {
        let _ = self.commands.send(Command::Quit);
    }
}

/// The join secrets that friends' Discords handed to this game, as they came.
#[derive(Resource)]
pub struct DiscordJoins(Mutex<Receiver<String>>);

impl DiscordJoins {
    /// The servers friends asked this player to join since the last call. A secret that is not a server address is
    /// dropped: it came from another player's Discord.
    pub fn take(&self) -> Vec<SocketAddr> {
        let received = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let mut servers = Vec::new();
        while let Ok(secret) = received.try_recv() {
            match join_address(&secret) {
                Some(server) => servers.push(server),
                None => log::debug!("discord: ignoring a join secret that is not a server address"),
            }
        }
        servers
    }
}

impl Plugin for DiscordPlugin {
    fn build(&self, app: &mut App) {
        if !app_id_is_set() {
            log::debug!("discord: no application id is set ({DISCORD_APP_ID}), so there is no presence");
            return;
        }
        let Ok(client_id) = DISCORD_APP_ID.parse::<u64>() else {
            log::debug!("discord: the application id {DISCORD_APP_ID} is not a number, so there is no presence");
            return;
        };
        let (commands, received) = mpsc::channel();
        let (joined, joins) = mpsc::channel();
        let requests = commands.clone();
        let spawned = thread::Builder::new().name("discord-presence".into()).spawn(move || {
            Worker::new(|| IpcLink::new(client_id, requests.clone(), joined.clone())).run(received);
        });
        let Ok(_thread) = spawned else {
            log::debug!("discord: the presence thread could not start");
            return;
        };
        app.init_resource::<PresenceStatus>()
            .init_resource::<PresenceParty>()
            .insert_resource(DiscordJoins(Mutex::new(joins)))
            .insert_resource(DiscordPresence { commands, started_at_ms: unix_ms_now(), sent: None })
            .add_systems(Update, sync_presence.in_set(FrameSet::Commands));
    }
}

/// Sends the presence to the thread when the settings, the status, the LAN game or the presence's state change. Nothing is
/// sent in between: the thread keeps showing the last one.
fn sync_presence(
    settings: Res<Settings>,
    status: Res<PresenceStatus>,
    party: Res<PresenceParty>,
    mut presence: ResMut<DiscordPresence>,
) {
    let _trace = blackbox_trace::scope("Discord: presence update");
    if !(settings.is_changed() || status.is_changed() || party.is_changed() || presence.sent.is_none()) {
        return;
    }
    let allowed =
        Allowed { discord: settings.discord, time_played: settings.discord_time_played, join: settings.discord_join };
    let wanted = presence_for(*status, party.0.as_ref(), allowed, presence.started_at_ms);
    if presence.sent.as_ref() == Some(&wanted) {
        return;
    }
    let command = match &wanted {
        Some(shown) => Command::Show(shown.clone()),
        None => Command::Hide,
    };
    presence.sent = Some(wanted);
    if presence.commands.send(command).is_err() {
        log::debug!("discord: the presence thread has stopped");
    }
}

/// The current Unix time in milliseconds, or 0 if the clock is before 1970.
fn unix_ms_now() -> i64 {
    let since_epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    i64::try_from(since_epoch.as_millis()).unwrap_or(0)
}
