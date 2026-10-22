//! The pose a NIS scene gives the player's car while the scene plays. The physics stays frozen under it (the world does
//! not step while a scene plays), and the car is drawn at the scene's pose instead. The scene's side is in
//! `nis/car_world.rs`.

use super::{CarPose, Drive};

impl Drive {
    /// Sets the pose a NIS scene gives the car, or clears it with `None`: the car then draws its physics pose again.
    pub fn set_nis_pose(&mut self, pose: Option<CarPose>) {
        self.nis_pose = pose;
    }

    /// The pose the car is drawn at now: the NIS scene's pose while a scene drives it, otherwise the physics pose.
    pub fn render_pose(&self) -> CarPose {
        self.nis_pose.unwrap_or_else(|| self.pose())
    }
}
