//! One frame of a client world, in place of the single-player simulation (`frame_systems.rs`):
//!
//! 1. The first frame drops the world's own traffic, police and co-op (`enter_client_mode`).
//! 2. Streaming: while the player's surroundings are not resident the car is held and the server is told
//!    (`StreamingStall`); its input is neutral meanwhile.
//! 3. The player's input of the frame: the devices', neutral in the fly view; shift edges of a frame with no due tick
//!    wait for the next one. The reset key asks the server (`ResetRequest`).
//! 4. With the world lent to the car (`lend.rs`): `ClientSession::update` (snapshots, reconciliation, events), then
//!    `due_ticks` and `submit_input`, one predicted 60 Hz tick each: the session's clock is the fixed-step
//!    accumulator. Then the drawn pose, the remote entities and the global block.
//! 5. The car is drawn at the session's smoothed pose; the chase camera follows it; the remote cars, the HUD lines and
//!    the spectator camera follow the session.

use blackbox_gfx::RenderBackend;
use glam::{Quat, Vec3};
use nfsmw_net::client_session::SessionEvent;
use nfsmw_net::predict::InputState;

use super::super::drive::{CarPose, STEP};
use super::super::{View, WorldScene};
use crate::input::ActionState;

/// Seconds a gameplay line stays on screen.
const BANNER_SECONDS: f32 = 4.0;

/// What the session said this frame.
struct Heard {
    pose: Option<CarPose>,
    events: Vec<SessionEvent>,
    spectating: bool,
    follow: Option<nfsmw_sim::EntityId>,
}

impl WorldScene {
    /// The client's frame; returns the simulated time that passed (the frame's, the server never pauses).
    pub(in crate::scenes::world) fn update_net_client(
        &mut self,
        renderer: &mut dyn RenderBackend,
        input: &ActionState,
        dt: f32,
    ) -> f32 {
        let Some(mode) = self.net_client.as_ref() else { return 0.0 };
        let (client, car, entered) = (mode.client.clone(), mode.car.clone(), mode.entered);
        if !entered {
            self.enter_client_mode(renderer);
        }
        for drive in self.players.drives_mut() {
            drive.bar_speedbreaker(true);
        }
        let loaded = self
            .foci_with_speed()
            .all(|(focus, speed)| self.residency.simulation_ready(focus, 20.0 + speed * dt.max(STEP)));
        let input = self.client_input(input, loaded);
        let stalled = !loaded;
        car.set_hold(stalled);
        let lent = self.lend_world(&car);
        let heard = {
            let Ok(mut client) = client.try_borrow_mut() else {
                if lent {
                    self.return_world(&car);
                }
                return dt;
            };
            let mode = self.net_client.as_mut().expect("the client mode is set");
            client.pump();
            if mode.stalled != stalled {
                client.session_mut().streaming_stall(stalled);
            }
            if input.reset {
                client.session_mut().request_reset();
            }
            submit(client.session_mut(), &mut mode.held, input.vehicle);
            client.session_mut().remote_entities(&mut mode.display);
            mode.global = client.session().global().map(|(_, global)| *global);
            let events = client.take_scene_events();
            let session = client.session();
            Heard {
                pose: session.own_pose().map(|pose| car_pose(pose.position, pose.orientation)),
                events,
                spectating: session.is_spectator(),
                follow: session.follow_target(),
            }
        };
        if lent {
            self.return_world(&car);
        }
        if let Some(mode) = self.net_client.as_mut() {
            mode.entered = true;
            mode.stalled = stalled;
        }
        self.follow_server_time();
        self.show_client_frame(renderer, heard, dt);
        dt
    }

    /// The player's input of this frame as the session sends it.
    fn client_input(&mut self, actions: &ActionState, loaded: bool) -> ClientInput {
        let driving = self.view == View::Chase && loaded;
        let Some(drive) = self.players.lead().filter(|_| driving) else { return ClientInput::default() };
        let device = drive.device_input(actions);
        ClientInput { vehicle: device.to_vehicle(), reset: device.reset }
    }

    /// The drawn pose, the camera, the remote cars and what the session said.
    fn show_client_frame(&mut self, renderer: &mut dyn RenderBackend, heard: Heard, dt: f32) {
        if let Some(drive) = self.players.lead_mut() {
            let wheels = drive.pose().wheels;
            drive.set_net_pose(heard.pose.map(|pose| CarPose { wheels, ..pose }));
            drive.set_alpha(1.0);
        }
        let driving = self.view == View::Chase;
        let collision = self.residency.collision();
        if let Some(drive) = self.players.lead_mut().filter(|_| driving) {
            let look = drive.take_look(1);
            drive.follow(collision, look, dt);
        }
        self.update_remote_cars(renderer);
        for event in &heard.events {
            self.client_event(event);
        }
        if heard.spectating {
            self.follow_spectated(heard.follow, dt);
        }
        if let Some((_, seconds)) = self.net_client.as_mut().and_then(|mode| mode.banner.as_mut()) {
            *seconds -= dt;
        }
        if let Some(mode) = self.net_client.as_mut() {
            mode.banner = mode.banner.take().filter(|(_, seconds)| *seconds > 0.0);
        }
    }

    /// A gameplay event as a line on screen.
    fn client_event(&mut self, event: &SessionEvent) {
        let line = match event {
            SessionEvent::Results(results) => Some(format!("Race over: {} finishers", results.rows.len())),
            SessionEvent::Game(message) => super::hud::game_line(message),
            SessionEvent::Ended(reason) => Some(crate::net::client::end_line(reason)),
            _ => None,
        };
        let Some(line) = line else { return };
        if let Some(mode) = self.net_client.as_mut() {
            mode.banner = Some((line, BANNER_SECONDS));
        }
    }
}

/// The player's input of one frame.
#[derive(Debug, Clone, Copy, Default)]
struct ClientInput {
    vehicle: InputState,
    reset: bool,
}

/// Every due tick gets the frame's input; the shift edges and a gear key go with the first only, and wait in `held`
/// when no tick is due this frame.
fn submit<L, C>(session: &mut nfsmw_net::client_session::ClientSession<L, C>, held: &mut InputState, input: InputState)
where
    L: nfsmw_net::client_session::ClientLink,
    C: nfsmw_net::predict::PredictedCar + Clone,
{
    let mut first = with_edges(input, held);
    let Some(ticks) = session.due_ticks() else {
        *held = first;
        return;
    };
    *held = InputState::default();
    for tick in ticks {
        session.submit_input(tick, &first);
        first = without_edges(first);
    }
}

/// `input` with the edges `held` kept from frames that had no tick.
pub(super) fn with_edges(input: InputState, held: &InputState) -> InputState {
    InputState {
        shift_up: input.shift_up || held.shift_up,
        shift_down: input.shift_down || held.shift_down,
        gear_select: input.gear_select.or(held.gear_select),
        ..input
    }
}

/// `input` without its one-tick edges.
pub(super) fn without_edges(input: InputState) -> InputState {
    InputState { shift_up: false, shift_down: false, gear_select: None, ..input }
}

/// A car pose from a session pose (render space, quaternion `[x, y, z, w]`); the wheels at rest.
pub(super) fn car_pose(position: [f32; 3], orientation: [f32; 4]) -> CarPose {
    CarPose {
        position: Vec3::from_array(position),
        rotation: Quat::from_array(orientation).normalize(),
        wheels: Default::default(),
    }
}
