//! The Bevy side of `NetSession`: the resource, what the command line asked for at start, and the two systems of each
//! frame (before the scene's update): the session's own update, then attaching the client to the world scene.

use bevy_app::{App, Plugin, Update};
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::IntoScheduleConfigs;
use game_install::GameDir;

use super::client::PlayerSetup;
use super::options::ServeOptions;
use super::session::NetSession;
use crate::app::{FrameSet, Host};

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

/// Adds the `NetSession` resource and its systems.
pub struct NetPlugin {
    pub dir: Option<GameDir>,
    pub player: PlayerSetup,
    pub start: NetStart,
}

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        let mut session = NetSession::new(self.dir.clone(), self.player.clone());
        let started = match &self.start {
            NetStart::Idle => Ok(()),
            NetStart::Host(options) => {
                session.set_auto_ready(true);
                session.host(options.clone())
            }
            NetStart::Join(addr) => {
                session.set_auto_ready(true);
                session.join(*addr)
            }
        };
        if let Err(error) = started {
            log::error!("net: {error:#}");
        }
        app.insert_non_send(session)
            .add_systems(Update, (update_session, attach_client).chain().in_set(FrameSet::Commands));
    }
}

/// The session's own work of the frame.
fn update_session(mut session: NonSendMut<NetSession>) {
    session.update();
}

/// Hands the client to the scene once a world scene that can run in client mode is up; a scene that takes it drives
/// the session from then on.
fn attach_client(mut host: NonSendMut<Host>, session: NonSend<NetSession>) {
    let Some(client) = session.client() else { return };
    let attached = client.try_borrow().is_ok_and(|client| client.is_attached());
    if attached {
        return;
    }
    if host.scene.attach_net_client(client) {
        log::info!("net: the world runs in client mode");
    }
}
