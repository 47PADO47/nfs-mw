//! The Bevy side of `NetSession`: the session goes into the front end's `LanBackend` resource as its `NetFrontend`,
//! with what the command line asked for at start, and systems run each frame before the scene's update: the session's
//! own update, attaching the client to the world scene, and the Discord party (`discord.rs`).

use bevy_app::{App, Plugin, Update};
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::IntoScheduleConfigs;
use game_install::GameDir;

use super::client::PlayerSetup;
use super::discord::{accept_joins, publish_party};
use super::options::ServeOptions;
use super::session::NetSession;
use crate::app::{FrameSet, Host};
use crate::frontend::lan::LanBackend;

/// What the command line asked the session to do at start.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum NetStart {
    /// Nothing: the front end may start a session later.
    #[default]
    Idle,
    /// `--host`: a listen server, the player in it.
    Host(ServeOptions),
    /// `--connect ADDR`.
    Join(std::net::SocketAddr),
}

/// Adds the game's `NetSession` as the front end's network.
pub struct NetPlugin {
    pub dir: Option<GameDir>,
    pub player: PlayerSetup,
    pub start: NetStart,
    /// The player's identity key file (`net_identity_file`); `None` makes a new key every run.
    pub identity_file: Option<std::path::PathBuf>,
}

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        let mut session = NetSession::new(self.dir.clone(), self.player.clone());
        session.set_identity_file(self.identity_file.clone());
        let started = match &self.start {
            NetStart::Idle => Ok(()),
            NetStart::Host(options) => {
                session.set_auto_ready(true);
                session.host(options.clone())
            }
            NetStart::Join(addr) => {
                session.set_auto_ready(true);
                session.join(*addr, None)
            }
        };
        if let Err(error) = started {
            log::error!("net: {error:#}");
        }
        app.insert_non_send(LanBackend::new(Box::new(session))).add_systems(
            Update,
            (update_session, attach_client, publish_party, accept_joins).chain().in_set(FrameSet::Commands),
        );
    }
}

/// The game's session in the front end's resource.
pub(super) fn session(backend: &mut LanBackend) -> Option<&mut NetSession> {
    backend.get()?.as_any_mut()?.downcast_mut::<NetSession>()
}

/// The session's own work of the frame.
fn update_session(mut backend: NonSendMut<LanBackend>) {
    if let Some(session) = session(&mut backend) {
        session.update();
    }
}

/// Hands the client to the scene once a world scene that can run in client mode is up; a scene that takes it drives
/// the session from then on.
fn attach_client(mut host: NonSendMut<Host>, mut backend: NonSendMut<LanBackend>) {
    let Some(session) = session(&mut backend) else { return };
    let Some(client) = session.client() else { return };
    let attached = client.try_borrow().is_ok_and(|client| client.is_attached());
    if attached {
        return;
    }
    if host.scene.attach_net_client(client) {
        log::info!("net: the world runs in client mode");
    }
}
