//! The world scene's side of the traffic: where it spawns around and when it runs.

use super::Focus;
use crate::scenes::world::{View, WorldScene};

impl WorldScene {
    /// Where traffic spawns around: the car, or the free camera.
    pub(in crate::scenes::world) fn traffic_focus(&self) -> Focus {
        match (&self.drive, self.view) {
            (Some(drive), View::Chase) => {
                let p = drive.position();
                Focus { position: [p.x, p.y], heading: drive.heading(), speed: drive.speed() }
            }
            _ => Focus {
                position: [self.camera.position.x, self.camera.position.y],
                heading: self.camera.yaw,
                speed: 0.0,
            },
        }
    }

    /// Runs the computer-driven cars around the player (or the free camera).
    pub(in crate::scenes::world) fn update_traffic(&mut self, dt: f32) {
        let focus = self.traffic_focus();
        let loaded = self.residency.complete();
        let Some(traffic) = self.traffic.as_mut() else { return };
        let (collision, props) = self.residency.world_parts();
        traffic.update(dt, focus, loaded, collision, props, &self.physics.surfaces);
    }
}
