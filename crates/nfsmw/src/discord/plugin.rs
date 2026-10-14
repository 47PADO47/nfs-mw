//! The Bevy side: starts the presence thread and tells it what to show when the settings or the status change.

use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use bevy_app::{App, Plugin, Update};
use bevy_ecs::prelude::*;

use super::link::IpcLink;
use super::presence::{Presence, PresenceStatus, presence_for};
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

impl Plugin for DiscordPlugin {
    fn build(&self, app: &mut App) {
        if !app_id_is_set() {
            log::debug!("discord: no application id is set ({DISCORD_APP_ID}), so there is no presence");
            return;
        }
        let (commands, received) = mpsc::channel();
        let spawned = thread::Builder::new().name("discord-presence".into()).spawn(move || {
            Worker::new(|| IpcLink::new(DISCORD_APP_ID)).run(received);
        });
        let Ok(_thread) = spawned else {
            log::debug!("discord: the presence thread could not start");
            return;
        };
        app.init_resource::<PresenceStatus>()
            .insert_resource(DiscordPresence { commands, started_at_ms: unix_ms_now(), sent: None })
            .add_systems(Update, sync_presence.in_set(FrameSet::Commands));
    }
}

/// Sends the presence to the thread when the settings, the status or the presence's state change. Nothing is
/// sent in between: the thread keeps showing the last one.
fn sync_presence(settings: Res<Settings>, status: Res<PresenceStatus>, mut presence: ResMut<DiscordPresence>) {
    if !(settings.is_changed() || status.is_changed() || presence.sent.is_none()) {
        return;
    }
    let wanted = presence_for(*status, settings.discord, settings.discord_time_played, presence.started_at_ms);
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
