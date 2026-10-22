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
    /// The second player's view of the world, rebuilt each frame.
    pub visible: Vec<Instance>,
    cut: CameraCut,
}

impl SplitState {
    pub fn new(setup: SplitSetup) -> Self {
        let mut cut = CameraCut::default();
        cut.arm();
        Self { setup, actions: ActionState::default(), camera: ChaseCamera::default(), visible: Vec::new(), cut }
    }

    /// The pedals the second player holds, from their actions.
    pub fn pedals(&self) -> DriveInput {
        DriveInput::from_actions(&self.actions, WheelOptions::default())
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
        let (Some(split), Some(traffic)) = (self.split.as_mut(), self.traffic.as_ref()) else { return };
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

    /// The middle of the two cars while a split-screen match is on, so the world around both stays loaded.
    pub(super) fn split_focus(&self, first: Vec3) -> Option<[f32; 2]> {
        let second = self.second_player_position()?;
        let middle = (first + second) * 0.5;
        Some([middle.x, middle.y])
    }

    /// The match is over: nothing of it stays in the world.
    pub(super) fn end_split(&mut self) {
        self.split = None;
    }
}
