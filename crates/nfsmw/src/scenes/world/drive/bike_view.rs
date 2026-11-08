//! A bike's pose for its cameras (`viewer/camera/bike.rs`, `scenes/world/camera_bike.rs`).

use std::sync::Arc;

use glam::{Mat4, Quat, Vec3};
use nfsmw_data::car::mods::bike_camera::{self, BikeCamera};

use super::rig::DashState;
use super::{Drive, RiderVisibility};
use crate::input::{Action, ActionState};
use crate::viewer::camera::head_look::HeadLook;

/// Where a bike is for a camera mounted on it.
#[derive(Debug, Clone, Copy)]
pub struct BikeView {
    /// The bike's model frame in the world.
    pub transform: Mat4,
    /// The bike's attitude, lean and wheelie included.
    pub rotation: Quat,
    /// The rider's eyes in the model frame, when the rider is on the bike and has eyes.
    pub eye: Option<Vec3>,
    /// Where the rider's head is turned by the look stick, for the helmet camera.
    pub head: HeadLook,
}

impl BikeView {
    /// The model frame's origin in the world.
    pub fn origin(&self) -> Vec3 {
        self.transform.transform_point3(Vec3::ZERO)
    }
}

impl Drive {
    /// The camera set of the car if it is a bike that has one.
    pub fn bike_cameras(&self) -> Option<Arc<BikeCamera>> {
        self.sim.as_ref().filter(|sim| sim.is_bike())?;
        bike_camera::of(&self.car_name)
    }

    /// The bike as the camera sees it: as it is drawn now when `presented`, else as the physics has it this tick.
    pub fn bike_view(&self, presented: bool) -> Option<BikeView> {
        self.sim.as_ref()?;
        let pose = match presented {
            true => self.render_pose(),
            false => self.physics_pose(),
        };
        let eye = self.rider.eye().filter(|_| presented);
        Some(BikeView { transform: pose.transform(), rotation: pose.rotation, eye, head: self.rider.head_look() })
    }

    /// One frame of the look axes for the helmet camera of a bike: the pad's right stick (X; Y is the gear shift there), the
    /// mouse's speed and the look keys. The rider's head follows them and springs back when they are let go; a car never reads
    /// it (`bike_cameras`). `dt` is the frame's time (0 while the world is frozen). A drive script's `look=` is stepped with
    /// the physics instead (`simulation.rs`).
    pub fn update_head_look(&mut self, actions: &ActionState, dt: f32) {
        if self.script.is_some() {
            return;
        }
        self.rider.look_around([actions.value(Action::HeadLookX), actions.value(Action::HeadLookY)], dt);
    }

    /// Paint the bike's cluster screen with what the bike does now (`rig/dash.rs`), when it has one and the live screen is on. Returns the
    /// microseconds a repaint took.
    pub fn feed_dash(&self, renderer: &mut dyn blackbox_gfx::RenderBackend, live: bool) -> Option<u128> {
        let now = std::time::Instant::now();
        if !live {
            self.rig.rest_dash(renderer, now);
            return None;
        }
        let t = &self.telemetry;
        let rpm_max = if t.max_rpm > 0.0 { t.max_rpm } else { t.red_line };
        self.rig.feed_dash(
            renderer,
            &DashState { speed_kmh: t.speed_mps * 3.6, gear: t.gear, rpm: t.rpm, rpm_max },
            now,
        )
    }

    /// What the camera shows of the rider (`RiderVisibility`): the helmet and visor are left out of the picture from inside the
    /// helmet, all of him when it looks back.
    pub fn set_rider_visibility(&mut self, visibility: RiderVisibility) {
        self.rider.set_visibility(visibility);
    }
}
