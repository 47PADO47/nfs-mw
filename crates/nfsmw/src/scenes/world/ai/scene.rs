//! The world scene's side of the traffic: where it spawns around and when it runs.

use blackbox_roads::Body;
use glam::{Vec2, Vec3};

use super::Focus;
use crate::scenes::world::space;
use crate::scenes::world::{View, WorldScene};

impl WorldScene {
    /// Where traffic spawns around: the car, or the free camera.
    pub(in crate::scenes::world) fn traffic_focus(&self) -> Focus {
        match (&self.drive, self.view) {
            (Some(drive), View::Chase) => {
                let (p, heading, speed) = (drive.position(), drive.heading(), drive.speed());
                // Physics space: the heading's direction and the car's velocity along it.
                let [fx, _, fz] = space::to_physics(Vec3::new(heading.cos(), heading.sin(), 0.0));
                let (half_width, half_length) = drive.half_extents();
                let body = Body {
                    position: Vec3::from(space::to_physics(p)),
                    velocity: Vec3::new(fx * speed, 0.0, fz * speed),
                    forward: Vec2::new(fx, fz),
                    half_width,
                    half_length,
                };
                Focus { position: [p.x, p.y], heading, speed, body: Some(body) }
            }
            _ => Focus {
                position: [self.camera.position.x, self.camera.position.y],
                heading: self.camera.yaw,
                speed: 0.0,
                body: None,
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
        let player = self.drive.as_mut().and_then(|drive| drive.sim_mut());
        traffic.collide(player);
    }
}
