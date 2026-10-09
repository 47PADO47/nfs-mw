//! Putting a traffic car on the road.
//! Spec: `docs/specs/ai-traffic-spawning.md` (§6, §7).

use blackbox_collision::CollisionWorld;
use blackbox_roads::{LaneType, NavKind, RandomSource, RoadNav, SegmentFilter};
use glam::Vec3;
use nfsmw_data::car::physics::SurfaceTable;

use super::car::{AiCar, Start};
use super::{Focus, TrafficWorld};
use crate::scenes::world::drive::WorldGround;
use crate::scenes::world::road::Spawn;
use crate::scenes::world::space;

/// Spawn distance ahead of the player: 200 m plus or minus 50, plus a second of the player's speed.
const AHEAD: f32 = 200.0;
const SPREAD: f32 = 50.0;
/// The wedge ahead the point is chosen in: plus or minus this many radians (45 degrees).
const WEDGE: f32 = std::f32::consts::FRAC_PI_4;
/// No new car closer than this to another one.
const MIN_GAP: f32 = 20.0;
/// How far ahead of the car the cursor starts.
const START_LOOK_AHEAD: f32 = 30.0;
/// A new car starts at this fraction of the posted speed.
const START_SPEED_FACTOR: f32 = 0.75;

impl TrafficWorld {
    /// Tries to add one car ahead of `focus`. Returns whether it worked.
    pub(super) fn spawn_one(&mut self, focus: Focus, collision: &CollisionWorld, surfaces: &SurfaceTable) -> bool {
        if self.models.is_empty() {
            return false;
        }
        let rng = &mut self.rng;
        let angle = focus.heading + (rng.next_f32() * 2.0 - 1.0) * WEDGE;
        let offset = (rng.next_f32() * 2.0 - 1.0) * SPREAD;
        let distance = AHEAD + offset + focus.speed.max(0.0);
        let at = Vec3::new(focus.position[0] + distance * angle.cos(), focus.position[1] + distance * angle.sin(), 0.0);
        // A standing player always gets oncoming traffic, a fast one half and half.
        let oncoming = rng.next_f32() <= 1.0 - 0.5 * (focus.speed / 50.0).clamp(0.0, 1.0);
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
        if self.cars.iter().any(|c| c.physics_position().distance(nav.position) < MIN_GAP) {
            return false;
        }
        let render = space::to_render(nav.position.to_array());
        let facing = space::to_render(nav.forward.to_array());
        let spawn = Spawn { position: render, heading: facing.y.atan2(facing.x) };
        let model = &self.models[rng.index(self.models.len())];
        nav.half_width = model.physics.spec.dimension.x;
        nav.enable_trail(&self.network);
        nav.advance_with_lookahead(&self.network, START_LOOK_AHEAD, Vec3::ZERO, START_LOOK_AHEAD, rng);
        let stagger = (rng.next_f32() * 10.0) as u32;
        let start_speed = START_SPEED_FACTOR * super::traffic::STREET_SPEED;
        let start = Start { stagger, speed: start_speed };
        let car = AiCar::place(model, nav, spawn, start, &WorldGround { collision, surfaces });
        let Some(car) = car else { return false };
        log::info!("traffic: {} at ({:.0}, {:.0}), {} m ahead", car.name, render.x, render.y, distance as i32);
        self.cars.push(car);
        true
    }
}
