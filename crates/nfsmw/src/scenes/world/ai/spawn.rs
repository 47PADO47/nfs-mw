//! Putting a traffic car on the road.
//! Spec: `docs/specs/ai-traffic-spawning.md` (§6, §7).

use blackbox_collision::CollisionWorld;
use blackbox_roads::{LaneType, NavKind, RandomSource, RoadNav, SegmentFilter};
use glam::Vec3;
use nfsmw_data::car::physics::SurfaceTable;

use super::car::{AiCar, Role, Start};
use super::traffic::Cruise;
use super::{Focus, TrafficModel, TrafficWorld};
use crate::scenes::world::drive::WorldGround;
use crate::scenes::world::road::Spawn;
use crate::scenes::world::space;

/// Spawn distance ahead of the player: 200 m plus or minus 50, plus a second of the player's speed.
const AHEAD: f32 = 200.0;
const SPREAD: f32 = 50.0;
/// The wedge ahead the point is chosen in: plus or minus this many radians (45 degrees).
const WEDGE: f32 = std::f32::consts::FRAC_PI_4;
/// The final lane position must stay this far from the player or free camera, horizontally.
const MIN_FOCUS_GAP: f32 = 150.0;
/// No new car closer than this to another one; a trailer adds half its length to the distance, on either side.
const MIN_GAP: f32 = 20.0;
/// How far ahead of the car the cursor starts.
const START_LOOK_AHEAD: f32 = 30.0;
/// A new car starts at this fraction of the lower of its two cruising speeds.
const START_SPEED_FACTOR: f32 = 0.75;
/// A standing player always gets oncoming traffic; at this speed (m/s) half of it comes the same way.
const SAME_WAY_SPEED: f32 = 50.0;

/// Which car to put on the road.
#[derive(Debug, Clone, Copy)]
pub enum Pick {
    /// The traffic model at this index.
    Traffic(usize),
    /// The patrol cop model at this index.
    Patrol(usize),
}

impl TrafficWorld {
    /// Tries to add the car `pick` ahead of `focus`. Returns whether it worked.
    pub(super) fn spawn_one(
        &mut self,
        pick: Pick,
        cruise: Cruise,
        focus: Focus,
        collision: &CollisionWorld,
        surfaces: &SurfaceTable,
    ) -> bool {
        let model: &TrafficModel = match pick {
            Pick::Traffic(i) => &self.models[i],
            Pick::Patrol(i) => &self.cop_models[i].model,
        };
        let rng = &mut self.rng;
        let angle = focus.heading + (rng.next_f32() * 2.0 - 1.0) * WEDGE;
        let offset = (rng.next_f32() * 2.0 - 1.0) * SPREAD;
        let distance = AHEAD + offset + focus.speed.max(0.0);
        let at = Vec3::new(focus.position[0] + distance * angle.cos(), focus.position[1] + distance * angle.sin(), 0.0);
        let oncoming = rng.next_f32() <= 1.0 - 0.5 * (focus.speed / SAME_WAY_SPEED).clamp(0.0, 1.0);
        let dir = match oncoming {
            true => angle + std::f32::consts::PI,
            false => angle,
        };
        let heading = space::to_physics(Vec3::new(dir.cos(), dir.sin(), 0.0));
        let physics_at = Vec3::from(space::to_physics(at));

        let mut nav = RoadNav::new(NavKind::Direction, LaneType::Racing, SegmentFilter::default());
        if !nav.init_at_point(&self.network, &self.index, physics_at, Vec3::from(heading), false)
            || !nav.can_traffic_spawn(&self.network, rng)
        {
            return false;
        }
        let length = model.trailer.as_ref().map_or(0.0, |t| 2.0 * t.physics.spec.dimension.z);
        let others = self.cars.iter().map(|c| (c.physics_position(), c.trailer_length()));
        if !clear_spawn(nav.position, focus, length, others) {
            return false;
        }
        let render = space::to_render(nav.position.to_array());
        let facing = space::to_render(nav.forward.to_array());
        let spawn = Spawn { position: render, heading: facing.y.atan2(facing.x) };
        nav.half_width = model.physics.spec.dimension.x;
        nav.enable_trail(&self.network);
        nav.advance_with_lookahead(&self.network, START_LOOK_AHEAD, Vec3::ZERO, START_LOOK_AHEAD, rng);
        let start = Start {
            stagger: (rng.next_f32() * 10.0) as u32,
            speed: START_SPEED_FACTOR * cruise.street.min(cruise.highway),
            cruise,
            patrol: matches!(pick, Pick::Patrol(_)),
        };
        let car = AiCar::place(model, Role::Traffic, nav, spawn, start, &WorldGround { collision, surfaces });
        let Some(car) = car else { return false };
        let towing =
            car.trailer().map_or(String::new(), |t| format!(" with a {:.1} m trailer", 2.0 * t.half_dimensions().z));
        log::info!("traffic: {}{towing} at ({:.0}, {:.0}), {} m ahead", car.name, render.x, render.y, distance as i32);
        self.cars.push(car);
        true
    }
}

/// Checks the final lane position, after the random probe has been projected onto the road.
fn clear_spawn(position: Vec3, focus: Focus, length: f32, others: impl Iterator<Item = (Vec3, f32)>) -> bool {
    let centre = Vec3::from(space::to_physics(Vec3::new(focus.position[0], focus.position[1], 0.0)));
    let planar_distance = |other: Vec3| (position.x - other.x).hypot(position.z - other.z);
    if planar_distance(centre) < MIN_FOCUS_GAP {
        return false;
    }
    !others.into_iter().any(|(other, trailer)| planar_distance(other) < MIN_GAP + 0.5 * (length + trailer))
}

#[cfg(test)]
mod tests;
