//! The client mode of the world (ADR 0011 section 4, `docs/specs/multiplayer/game-wiring.md` section 4): the same
//! `WorldScene`, driven by a multiplayer client session. Single player is unchanged: without a client none of this
//! runs, and the frame's systems are chosen in one place (`frame_systems.rs`).
//!
//! In client mode the world runs no traffic, police, pursuit, race or career of its own (the server owns them). Its own
//! car is simulated locally for prediction: the session's predictor steps the real `Drive` through [`GameCar`], with
//! the world lent to it for the frame's session calls (`lend.rs`). Every other entity (remote players, traffic, police,
//! roadblock cars) is drawn from the session's interpolated snapshots through the car rigs and poses the world uses
//! for AI cars (`remote.rs`). The HUD reads the server's global block (`hud.rs`).
//!
//! - `car.rs`: [`GameCar`], the game's `PredictedCar`.
//! - `lend.rs`: lending the player's car and the static world to it.
//! - `frame.rs`: the client's frame: streaming stall, input, the session's update and due ticks, the drawn pose.
//! - `remote.rs`: the remote cars.
//! - `hud.rs`, `spectate.rs`: the HUD from the global block, the spectator's follow camera.

mod car;
mod frame;
mod hud;
#[cfg(test)]
mod installed;
mod lend;
mod remote;
mod spectate;
#[cfg(test)]
mod tests;

pub use car::GameCar;
#[cfg(test)]
use car::{motion_of, state_of};

use blackbox_gfx::RenderBackend;
use nfsmw_net::client_session::GlobalView;
use nfsmw_net::interp::Drawn;
use nfsmw_net::predict::InputState;
use nfsmw_sim::EntityId;

use super::WorldScene;
use crate::net::ClientHandle;

/// What the world keeps of its client.
pub struct ClientMode {
    client: ClientHandle,
    car: GameCar,
    remote: remote::RemoteCars,
    /// The remote entities at display time, this frame.
    display: Vec<(EntityId, Drawn)>,
    /// The world's own traffic and co-op went (once, at the first client frame).
    entered: bool,
    /// The car's area is not resident: `StreamingStall` was sent and the car is held.
    stalled: bool,
    /// Shift edges and gear keys of frames that had no due tick, for the next tick.
    held: InputState,
    /// The newest global block.
    global: Option<GlobalView>,
    /// A gameplay line (busted, results) and the seconds it stays.
    banner: Option<(String, f32)>,
}

impl ClientMode {
    fn new(client: ClientHandle, car: GameCar) -> Self {
        Self {
            client,
            car,
            remote: remote::RemoteCars::default(),
            display: Vec::new(),
            entered: false,
            stalled: false,
            held: InputState::default(),
            global: None,
            banner: None,
        }
    }

    /// The session's client, for the front end and tests.
    pub fn client(&self) -> &ClientHandle {
        &self.client
    }

    /// The remote entities drawn this frame.
    pub fn display(&self) -> &[(EntityId, Drawn)] {
        &self.display
    }

    /// Whether the car waits for its area to stream in.
    pub fn stalled(&self) -> bool {
        self.stalled
    }

    /// Frees the remote cars' rigs.
    pub(super) fn release(&mut self, renderer: &mut dyn RenderBackend) {
        self.remote.release(renderer);
    }
}

impl Drop for ClientMode {
    /// The client goes back to the `NetSession` systems, which pump it while no world drives it.
    fn drop(&mut self) {
        if let Ok(mut client) = self.client.try_borrow_mut() {
            client.set_attached(false);
        }
    }
}

impl WorldScene {
    /// Takes `client`: the world runs in client mode from the next frame on. Refused by a world that already has a
    /// client, or that has no car of its own (the fly-through viewer).
    pub(in crate::scenes::world) fn attach_client(&mut self, client: &ClientHandle) -> bool {
        if self.net_client.is_some() || (!self.players.has_lead() && self.pending_car.is_none()) {
            return false;
        }
        let Ok(mut inner) = client.try_borrow_mut() else { return false };
        inner.set_attached(true);
        let car = inner.car().clone();
        drop(inner);
        car.set_surfaces(self.physics.surfaces.clone());
        self.net_client = Some(ClientMode::new(client.clone(), car));
        true
    }

    /// The client of this world, if it runs in client mode.
    pub fn net_client(&self) -> Option<&ClientMode> {
        self.net_client.as_ref()
    }

    /// Once, at the first client frame: the world's own traffic and police, and a co-op session, leave; the server
    /// owns them now. A race that runs is left where it is (its frame is not run: `frame_systems.rs`).
    fn enter_client_mode(&mut self, renderer: &mut dyn RenderBackend) {
        if let Some(mut traffic) = self.traffic.take() {
            traffic.release(renderer);
        }
        if self.coop.is_some() {
            self.end_coop(renderer);
        }
        if self.race.is_some() {
            log::warn!("net: a race was running when the world became a client; it stops where it is");
        }
        log::info!("net: the world is a client now: no traffic, police or races of its own");
    }
}
