//! A split-screen match in the world: the second player's actions and camera, their pedals in the car, and the two
//! views. The second human is a racer of the race (`ai/car/racing.rs`), so the physics, the collisions, the ranking and
//! the finish are the race's. Spec: `docs/specs/split-screen.md`.

mod hud;
mod view;

use blackbox_gfx::Instance;
use glam::Vec3;

pub(super) use view::Viewer;

pub(super) use hud::map_position;

use super::drive::DriveInput;
use super::{SplitSetup, WorldScene};
use crate::input::{Action, ActionState};
use crate::settings::WheelOptions;
use crate::viewer::camera::{CameraCut, ChaseCamera};

/// The race index of the second player.
pub(super) const SECOND: usize = 1;

pub(super) struct SplitState {
    pub setup: SplitSetup,
    /// The second player's actions this frame (`Scene::set_second_input`).
    pub actions: ActionState,
    /// The second player's chase camera, following their racer.
    pub camera: ChaseCamera,
}

impl SplitState {
    pub fn new(setup: SplitSetup) -> Self {
        Self { setup, actions: ActionState::default(), camera: ChaseCamera::default() }
    }

    /// The pedals the second player holds, from their actions.
    pub fn pedals(&self) -> DriveInput {
        DriveInput::from_actions(&self.actions, WheelOptions::default())
    }
}

/// The bottom half of a split screen, for a split-screen race and for co-op alike: its draw list and its camera cut.
pub(super) struct SecondView {
    /// The second player's view of the world, rebuilt each frame.
    pub visible: Vec<Instance>,
    cut: CameraCut,
}

impl SecondView {
    /// A view whose first frame is a cut.
    pub fn new() -> Self {
        let mut cut = CameraCut::default();
        cut.arm();
        Self { visible: Vec::new(), cut }
    }

    /// Whether the second camera jumped since the last frame (the first frame of the match).
    pub fn take_cut(&mut self) -> bool {
        self.cut.take()
    }
}

impl WorldScene {
    /// Tells the second player's racer what their pedals ask, before the physics steps.
    pub(super) fn feed_second_player(&mut self) {
        let (Some(split), Some(traffic)) = (self.split.as_mut(), self.traffic.as_mut()) else { return };
        let pedals = split.pedals();
        traffic.set_racer_pedals(SECOND, pedals);
        split.camera.set_look_back(split.actions.pressed(Action::LookBack));
    }

    /// Moves the second player's camera after their car, on the frame's clock.
    pub(super) fn follow_second_player(&mut self, dt: f32) {
        let camera_shake = self.tuning.camera_shake;
        let (Some(split), Some(traffic)) = (self.split.as_mut(), self.traffic.as_ref()) else { return };
        split.camera.set_shake_intensity(camera_shake);
        let Some(car) = traffic.racer_followed(SECOND, 1.0) else { return };
        if dt == 0.0 {
            // A run with no frame time puts the camera where it belongs at once.
            split.camera.snap();
        }
        split.camera.update(car, dt, |_, _| None);
    }

    /// The second player's car as the race sees it, in render space; `None` outside a split-screen match.
    pub(super) fn second_player_position(&self) -> Option<Vec3> {
        self.split.as_ref()?;
        Some(self.traffic.as_ref()?.racer_followed(SECOND, 1.0)?.position)
    }

    /// The match is over: nothing of it stays in the world.
    pub(super) fn end_split(&mut self) {
        self.split = None;
    }
}
