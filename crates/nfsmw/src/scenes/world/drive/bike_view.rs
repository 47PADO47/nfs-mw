//! A bike's pose for its cameras (`viewer/camera/bike.rs`, `scenes/world/camera_bike.rs`).

use std::sync::Arc;

use glam::{Mat4, Quat, Vec3};
use nfsmw_data::car::mods::bike_camera::{self, BikeCamera};

use super::Drive;

/// Where a bike is for a camera mounted on it.
#[derive(Debug, Clone, Copy)]
pub struct BikeView {
    /// The bike's model frame in the world.
    pub transform: Mat4,
    /// The bike's attitude, lean and wheelie included.
    pub rotation: Quat,
    /// The rider's eyes in the model frame, when the rider is on the bike and has eyes.
    pub eye: Option<Vec3>,
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
        Some(BikeView { transform: pose.transform(), rotation: pose.rotation, eye })
    }

    /// Whether the camera is inside the rider's helmet, so his helmet and visor are left out of the picture.
    pub fn set_first_person(&mut self, on: bool) {
        self.rider.set_first_person(on);
    }
}
