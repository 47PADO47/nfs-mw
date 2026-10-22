//! The GPS route of the drive: a route from the car to a point on the road, followed by the `blackbox_roads` GPS
//! navigator, drawn as the original's direction arrow over the car and as a blip on the minimap. Spec:
//! `docs/specs/gps-route.md`.
//!
//! The route is set from the world map (Activate GPS), the console (`gps <x> <y>`) and the mods (`set_waypoint`).

mod arrow;

use std::f32::consts::{PI, TAU};

use blackbox_gfx::Instance;
use blackbox_hash::bstring_hash;
use blackbox_roads::{Gps, GpsEnd, GpsState};
use glam::{Quat, Vec3};

use super::{View, WorldScene, space};
use crate::hud::{GPS, MapBlip};

/// The original's arrow model: `MARKER_DIRECTION_AID` in `GLOBAL/InGameA.bun`, loaded with the shared models.
const ARROW_MODEL: &str = "MARKER_DIRECTION_AID";
/// How far ahead of the car the arrow lies, metres along the route's direction.
const AHEAD: f32 = 0.0;
/// The arrow floats in the sky above the car: its height above the car's origin, metres. From the chase camera a flat arrow
/// this high shows its whole face.
const SKY_HEIGHT: f32 = 2.3;
/// The arrow model's size on screen (it is about 2.9 m across in the game).
const ARROW_SCALE: f32 = 0.6;
/// The bob of the arrow: amplitude in metres and cycles per second.
const BOB: f32 = 0.12;
const BOB_RATE: f32 = 0.5;
/// The pulse while searching for the route again: scale amplitude and cycles per second.
const PULSE: f32 = 0.1;
const PULSE_RATE: f32 = 2.5;
/// How fast the arrow turns towards the route (per second, the exponential rate).
const TURN_RATE: f32 = 8.0;

/// The route being followed and the arrow's animation.
#[derive(Default)]
pub struct Guide {
    route: Option<Gps>,
    clock: f32,
    /// The route's bearing from the car's straight ahead, radians, positive to the right.
    bearing: f32,
}

impl WorldScene {
    /// Sets the route to the point `target` (render space x, y, on the road the car is near), or clears it with
    /// `None`. Returns whether the route is now set: a point with no road or no way to it sets nothing.
    pub(super) fn route_to(&mut self, target: Option<[f32; 2]>) -> bool {
        let Some([x, y]) = target else {
            self.guide.route = None;
            return true;
        };
        let (Some(drive), Some(traffic)) = (self.players.lead(), self.traffic.as_ref()) else {
            return false;
        };
        let pose = drive.physics_pose();
        let (net, index) = traffic.road_graph();
        // The target is taken at the car's height; the route's own search puts it on the road.
        let goal = physics_point(Vec3::new(x, y, pose.position.z));
        let route = Gps::engage(
            net,
            index,
            physics_point(pose.position),
            physics_direction(pose.rotation * Vec3::X),
            goal,
            0.0,
        );
        let Some(route) = route else {
            log::info!("gps: no route to ({x:.0}, {y:.0})");
            return false;
        };
        log::info!("gps: route to ({x:.0}, {y:.0}), {} segments", route.path().len());
        self.guide.bearing = bearing(&route, pose.rotation, pose.position).unwrap_or(self.guide.bearing);
        self.guide.route = Some(route);
        true
    }

    /// One frame of the route: follows the car, turns the arrow and clears the route when it ends.
    pub(super) fn update_route(&mut self, dt: f32) {
        self.guide.clock += dt;
        let (Some(route), Some(drive), Some(traffic)) =
            (self.guide.route.as_mut(), self.players.lead(), self.traffic.as_ref())
        else {
            return;
        };
        let pose = drive.physics_pose();
        let (net, index) = traffic.road_graph();
        let heading = physics_direction(pose.rotation * Vec3::X);
        let ended = route.update(net, index, physics_point(pose.position), heading, drive.speed().abs(), dt);
        if let Some(end) = ended {
            let text = match end {
                GpsEnd::Finished => "finished",
                GpsEnd::Lost => "lost",
            };
            log::info!("gps: route {text}");
            self.guide.route = None;
            return;
        }
        let Some(wanted) = bearing(route, pose.rotation, pose.position) else { return };
        self.guide.bearing = turn_towards(self.guide.bearing, wanted, dt);
    }

    /// The minimap blip of the route's target, if a route is set.
    pub(super) fn route_blip(&self) -> Option<MapBlip> {
        let route = self.guide.route.as_ref()?;
        let [x, y, _] = space::to_render(route.target().to_array()).to_array();
        Some(MapBlip { style: GPS, position: [x, y], active: true, heading: None })
    }

    /// The arrow for this frame, over the car while driving in the chase view. `None` without a route, or when the
    /// game's arrow model is not in the install.
    pub(super) fn route_arrow(&self) -> Option<Instance> {
        let route = self.guide.route.as_ref()?;
        let drive = self.players.lead()?;
        if self.view != View::Chase {
            return None;
        }
        let mesh = self.residency.shared.meshes.get(&bstring_hash(ARROW_MODEL))?.0;
        let pose = drive.pose();
        let clock = self.guide.clock;
        let bob = BOB * (clock * TAU * BOB_RATE).sin();
        let scale = match route.state() {
            GpsState::Searching => ARROW_SCALE * (1.0 + PULSE * (clock * TAU * PULSE_RATE).sin()),
            GpsState::Tracking => ARROW_SCALE,
        };
        // The route's direction: the car's heading turned by the bearing, which is positive to the right (clockwise).
        let yaw = heading_of(pose.rotation) - self.guide.bearing;
        // Flat, floating above the car and turned along the route.
        let (sin, cos) = yaw.sin_cos();
        let origin = pose.position + Vec3::new(cos, sin, 0.0) * AHEAD + Vec3::Z * (SKY_HEIGHT + bob);
        Some(Instance::new(mesh, arrow::transform(origin, yaw, scale)))
    }
}

/// A render-space point as physics space.
fn physics_point(p: Vec3) -> Vec3 {
    Vec3::from_array(space::to_physics(p))
}

/// A render-space direction as physics space: the conversion is linear, so a direction takes the same map.
fn physics_direction(d: Vec3) -> Vec3 {
    Vec3::from_array(space::to_physics(d))
}

/// The route's bearing from the car's straight ahead, radians, positive to the right. `None` when the arrow point is on
/// the car.
fn bearing(route: &Gps, rotation: Quat, car: Vec3) -> Option<f32> {
    let aim = space::to_render(route.arrow_point().to_array());
    let delta = (aim - car).truncate();
    if delta.length_squared() <= 1e-6 {
        return None;
    }
    let forward = (rotation * Vec3::X).truncate();
    let left = (rotation * Vec3::Y).truncate();
    Some((-delta.dot(left)).atan2(delta.dot(forward)))
}

/// The car's heading in the world: the angle of its forward direction about up.
fn heading_of(rotation: Quat) -> f32 {
    let forward = rotation * Vec3::X;
    forward.y.atan2(forward.x)
}

/// Moves `bearing` towards `wanted` along the shorter way, at the turn rate.
fn turn_towards(bearing: f32, wanted: f32, dt: f32) -> f32 {
    let diff = (wanted - bearing + PI).rem_euclid(TAU) - PI;
    bearing + diff * (1.0 - (-TURN_RATE * dt).exp())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_arrow_turns_the_short_way_round() {
        // From just under +PI to just over -PI is a small turn through PI, not a near-full turn back.
        let turned = turn_towards(3.0, -3.0, 1.0);
        assert!(turned > 3.0, "{turned}");
        let far = turn_towards(0.0, PI / 2.0, 10.0);
        assert!((far - PI / 2.0).abs() < 1e-3, "{far}");
    }
}
