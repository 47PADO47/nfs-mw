//! What the cinematic moment of a pursuit breaker asks of the police: the cops chasing the player, and a hold on the
//! busted meter while the cut plays. Spec: `docs/specs/pursuit-breaker-moment.md`.

use glam::Vec3;

use super::TrafficWorld;

impl TrafficWorld {
    /// The ground cops chasing the player (not wrecked), render space.
    pub fn pursuer_positions(&self) -> Vec<Vec3> {
        self.cars.iter().filter(|car| car.is_cop()).map(|car| car.position()).collect()
    }

    /// The game camera the helicopter keeps out of (`AvoidCamera`), render space; `None` clears it.
    pub fn set_heli_camera(&mut self, camera: Option<(Vec3, Vec3)>) {
        self.police.set_heli_camera(camera.map(|(position, forward)| blackbox_pursuit::helicopter::camera::Camera {
            position: Vec3::from_array(crate::scenes::world::space::to_physics(position)),
            forward: Vec3::from_array(crate::scenes::world::space::to_physics(forward)),
        }));
    }

    /// Holds the busted meter where it is while a cinematic moment plays: it neither fills nor drains.
    pub fn hold_busted(&mut self, hold: bool) {
        self.police.hold_busted(hold);
    }
}
