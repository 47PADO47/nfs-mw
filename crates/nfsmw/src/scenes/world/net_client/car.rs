//! The owner's car of a multiplayer client as the session predicts it: `GameCar`, the game's
//! [`PredictedCar`] (`docs/specs/multiplayer/sync-model.md` section 2, `client-session.md` section 5).
//!
//! The session owns its car (it clones a template when the first own-car block places it), but a tick of the real car
//! needs the world: the player's [`Drive`], the collision and the props. So a `GameCar` is a shared handle, and the
//! scene *lends* those three to it for the length of one frame's session calls (`lend.rs`): every `step`, `save`,
//! `restore` and `set_state` then works on the scene's own car, with the same physics step the server runs
//! (`Drive::predict_step`, the `Drive::set_remote_input` path). Outside a lend the car answers from the last state it
//! saw, a step does nothing, and a state set meanwhile is applied at the next lend.

use std::cell::RefCell;
use std::rc::Rc;

use blackbox_collision::CollisionWorld;
use glam::{Quat, Vec3};
use nfsmw_data::car::physics::SurfaceTable;
use nfsmw_net::predict::{CarState, InputState, PredictedCar};

use super::super::drive::{Drive, DriveInput, DriveSave, SimMotion};
use super::super::props::PropWorld;

/// What a predicted tick needs of the world, lent for one frame's session calls.
pub struct Lent {
    pub drive: Drive,
    pub collision: CollisionWorld,
    pub props: PropWorld,
}

#[derive(Default)]
struct Desk {
    lent: Option<Lent>,
    /// The road grips, given once by the scene.
    surfaces: SurfaceTable,
    /// The reduced state now: refreshed after every change while lent, kept while not.
    state: CarState,
    /// A state set while nothing was lent (or before the car stood on a road), for the next lend.
    pending: Option<CarState>,
    /// The car's surroundings are not resident: steps leave the car where it is (`StreamingStall`).
    hold: bool,
    /// Physics ticks the car ran, replays included.
    steps: u64,
}

/// The player's car of a client world, shared between the session's predictor and the scene.
#[derive(Clone, Default)]
pub struct GameCar {
    desk: Rc<RefCell<Desk>>,
}

/// Everything a later predicted tick depends on (`Drive::predict_save`), and the reduced state with it.
#[derive(Debug, Clone)]
pub struct GameCarSave {
    drive: Option<DriveSave>,
    state: CarState,
}

impl GameCar {
    pub fn new() -> Self {
        Self::default()
    }

    /// The road grips the steps use.
    pub(in crate::scenes::world) fn set_surfaces(&self, surfaces: SurfaceTable) {
        self.desk.borrow_mut().surfaces = surfaces;
    }

    /// Lends the world for this frame's session calls; a state set meanwhile is applied now (or, while the car still
    /// waits for its road, the car is asked onto the road there).
    pub(in crate::scenes::world) fn lend(&self, mut lent: Lent) {
        let mut desk = self.desk.borrow_mut();
        if let Some(state) = desk.pending.take() {
            if !apply(&mut lent.drive, &state) {
                desk.pending = Some(state);
            }
        }
        if let Some(motion) = lent.drive.motion() {
            desk.state = state_of(&motion, desk.state.boost);
        }
        desk.lent = Some(lent);
    }

    /// Takes the world back at the end of the frame's session calls.
    pub(in crate::scenes::world) fn take_back(&self) -> Option<Lent> {
        self.desk.borrow_mut().lent.take()
    }

    /// Whether the world is lent now.
    #[cfg(test)]
    pub fn is_lent(&self) -> bool {
        self.desk.borrow().lent.is_some()
    }

    /// Hold the car (its area is not resident) or let it step again.
    pub(in crate::scenes::world) fn set_hold(&self, hold: bool) {
        self.desk.borrow_mut().hold = hold;
    }

    /// Physics ticks run so far, replays included.
    #[cfg(test)]
    pub fn steps(&self) -> u64 {
        self.desk.borrow().steps
    }

    /// Whether a state waits for the next lend.
    #[cfg(test)]
    pub fn has_pending(&self) -> bool {
        self.desk.borrow().pending.is_some()
    }
}

impl PredictedCar for GameCar {
    type Saved = GameCarSave;

    fn step(&mut self, input: &InputState) {
        let mut desk = self.desk.borrow_mut();
        let desk = &mut *desk;
        if desk.hold {
            return;
        }
        let Some(lent) = desk.lent.as_mut() else { return };
        let input = DriveInput::from_vehicle(input);
        if lent.drive.predict_step(input, &lent.collision, &mut lent.props, &desk.surfaces) {
            desk.steps += 1;
        }
        if let Some(motion) = lent.drive.motion() {
            desk.state = state_of(&motion, desk.state.boost);
        }
    }

    fn state(&self) -> CarState {
        self.desk.borrow().state
    }

    fn set_state(&mut self, state: &CarState) {
        let mut desk = self.desk.borrow_mut();
        desk.state = *state;
        let applied = desk.lent.as_mut().is_some_and(|lent| apply(&mut lent.drive, state));
        desk.pending = (!applied).then_some(*state);
    }

    fn save(&self) -> GameCarSave {
        let desk = self.desk.borrow();
        GameCarSave { drive: desk.lent.as_ref().and_then(|lent| lent.drive.predict_save()), state: desk.state }
    }

    fn restore(&mut self, saved: &GameCarSave) {
        let mut desk = self.desk.borrow_mut();
        if let (Some(lent), Some(drive)) = (desk.lent.as_mut(), saved.drive.as_ref()) {
            lent.drive.predict_restore(drive);
        }
        desk.state = saved.state;
    }
}

/// Puts `drive` in `state`. A car that does not stand on a road yet is asked onto the road nearest the state's place,
/// facing its way, and `false` says the state still waits.
fn apply(drive: &mut Drive, state: &CarState) -> bool {
    if drive.motion().is_none() || drive.waiting_for_road().is_some() {
        let forward = Quat::from_array(state.orientation) * Vec3::X;
        drive.respawn_near([state.position[0], state.position[1]], Some(forward.y.atan2(forward.x)));
        return false;
    }
    drive.set_motion(&motion_of(state));
    true
}

/// The reduced state of a motion; `boost` (the speedbreaker meter) is not part of the physics and is kept.
pub fn state_of(motion: &SimMotion, boost: f32) -> CarState {
    CarState {
        position: motion.position.to_array(),
        orientation: motion.rotation.to_array(),
        linear_velocity: motion.linear.to_array(),
        angular_velocity: motion.angular.to_array(),
        rpm: motion.rpm,
        gear: motion.gear,
        nitrous: motion.nitrous,
        boost,
    }
}

/// The motion of a reduced state (its engine fields are not written back to the car).
pub fn motion_of(state: &CarState) -> SimMotion {
    SimMotion {
        position: Vec3::from_array(state.position),
        rotation: Quat::from_array(state.orientation),
        linear: Vec3::from_array(state.linear_velocity),
        angular: Vec3::from_array(state.angular_velocity),
        rpm: state.rpm,
        gear: state.gear,
        nitrous: state.nitrous,
    }
}
