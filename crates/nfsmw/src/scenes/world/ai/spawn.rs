//! Putting a traffic car on the road.
//! Spec: `docs/specs/ai-traffic-spawning.md` (§6, §7).

use blackbox_collision::CollisionWorld;
use blackbox_roads::{LaneType, NavKind, RandomSource, RoadNav, SegmentFilter};
use glam::Vec3;
use nfsmw_data::car::physics::SurfaceTable;

use super::car::{AiCar, Role, Start};
use super::population_stats::Rejection;
use super::traffic::Cruise;
use super::{Focus, TrafficModel, TrafficWorld};
use crate::scenes::world::drive::WorldGround;
use crate::scenes::world::road::Spawn;
use crate::scenes::world::space;

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
        self.population_stats.attempts += 1;
        match self.try_spawn_one(pick, cruise, focus, collision, surfaces) {
            Ok(()) => {
                self.population_stats.spawned += 1;
                true
            }
            Err(reason) => {
                self.population_stats.rejected[reason as usize] += 1;
                false
            }
        }
    }

    fn try_spawn_one(
        &mut self,
        pick: Pick,
        cruise: Cruise,
        focus: Focus,
        collision: &CollisionWorld,
        surfaces: &SurfaceTable,
    ) -> Result<(), Rejection> {
        let model: &TrafficModel = match pick {
            Pick::Traffic(i) => &self.models[i],
            Pick::Patrol(i) => &self.cop_models[i].model,
        };
        let occupied = self.population_slots(focus);
        let rng = &mut self.rng;
        let angle = focus.heading + (rng.next_f32() * 2.0 - 1.0) * WEDGE;
        let projected = projected_speed(focus, angle);
        let distance = self.policy.spawn_min as f32
            + rng.next_f32() * (self.policy.spawn_max - self.policy.spawn_min) as f32
            + projected.max(0.0);
        let at = probe_point(focus, distance, angle);
        let oncoming = rng.next_f32() <= 1.0 - 0.5 * (projected / SAME_WAY_SPEED).clamp(0.0, 1.0);
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
            return Err(Rejection::Lane);
        }
        let slots = 1 + usize::from(model.trailer.is_some());
        // Patrols use the heat row's separate population cap, including when traffic is disabled.
        if matches!(pick, Pick::Traffic(_)) && occupied + slots > self.target {
            return Err(Rejection::Budget);
        }
        if collision.resident_near(nav.position.to_array(), 40.0) == Some(false) {
            return Err(Rejection::Collision);
        }
        let render_at = space::to_render(nav.position.to_array());
        if glam::Vec2::new(render_at.x - focus.position[0], render_at.y - focus.position[1]).length() > distance + 50.0
        {
            return Err(Rejection::Projection);
        }
        let length = model.trailer.as_ref().map_or(0.0, |t| 2.0 * t.physics.spec.dimension.z);
        let others = self.cars.iter().map(|c| (c.physics_position(), c.trailer_length()));
        if !clear_spawn(nav.position, focus, self.policy.spawn_min as f32, length, others) {
            return Err(Rejection::Spacing);
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
        let Some(car) = car else { return Err(Rejection::Ground) };
        let towing =
            car.trailer().map_or(String::new(), |t| format!(" with a {:.1} m trailer", 2.0 * t.half_dimensions().z));
        log::info!("traffic: {}{towing} at ({:.0}, {:.0}), {} m ahead", car.name, render.x, render.y, distance as i32);
        self.cars.push(car);
        Ok(())
    }
}

impl TrafficWorld {
    /// Puts a traffic car on the road at the map position `at` (x, y), facing `heading` (radians from +X): a random loaded
    /// traffic model, on the nearest lane that can take a car, with the spacing from the other cars that the manager uses.
    /// This is the mods' spawn (`bb.vehicles` `spawn_traffic`). The manager uses a minimum distance; a mod asked for this
    /// point, so a point nearer than [`MIN_FOCUS_GAP`] to `focus` is allowed, and logged as one that may pop into view.
    /// Returns whether the car stands: `false` while no traffic model is loaded or the road has no place for it.
    pub fn spawn_traffic_at(
        &mut self,
        at: [f32; 2],
        heading: f32,
        focus: Focus,
        collision: &CollisionWorld,
        surfaces: &SurfaceTable,
    ) -> bool {
        if self.models.is_empty() {
            return false;
        }
        let distance = (at[0] - focus.position[0]).hypot(at[1] - focus.position[1]);
        if distance < MIN_FOCUS_GAP {
            log::warn!(
                "traffic at ({:.0}, {:.0}) is {distance:.0} m from the player: it may pop into view",
                at[0],
                at[1]
            );
        }
        let index = self.rng.index(self.models.len());
        let physics_at = Vec3::from(space::to_physics(Vec3::new(at[0], at[1], 0.0)));
        let dir = Vec3::from(space::to_physics(Vec3::new(heading.cos(), heading.sin(), 0.0)));
        let mut nav = RoadNav::new(NavKind::Direction, LaneType::Racing, SegmentFilter::default());
        if !nav.init_at_point(&self.network, &self.index, physics_at, dir, false)
            || !nav.can_traffic_spawn(&self.network, &mut self.rng)
        {
            return false;
        }
        let model = &self.models[index];
        let length = model.trailer.as_ref().map_or(0.0, |t| 2.0 * t.physics.spec.dimension.z);
        let others = self.cars.iter().map(|c| (c.physics_position(), c.trailer_length()));
        if !clear_of_cars(nav.position, length, others) {
            return false;
        }
        let render = space::to_render(nav.position.to_array());
        let facing = space::to_render(nav.forward.to_array());
        let spawn = Spawn { position: render, heading: facing.y.atan2(facing.x) };
        nav.half_width = model.physics.spec.dimension.x;
        nav.enable_trail(&self.network);
        nav.advance_with_lookahead(&self.network, START_LOOK_AHEAD, Vec3::ZERO, START_LOOK_AHEAD, &mut self.rng);
        let cruise = Cruise::default();
        let start = Start {
            stagger: (self.rng.next_f32() * 10.0) as u32,
            speed: START_SPEED_FACTOR * cruise.street.min(cruise.highway),
            cruise,
            patrol: false,
        };
        let Some(car) = AiCar::place(model, Role::Traffic, nav, spawn, start, &WorldGround { collision, surfaces })
        else {
            return false;
        };
        log::info!("traffic: {} at ({:.0}, {:.0}), asked for by a mod", car.name, render.x, render.y);
        self.cars.push(car);
        true
    }
}

/// The distance on the ground between two points (the height is not used).
fn planar_distance(a: Vec3, b: Vec3) -> f32 {
    (a.x - b.x).hypot(a.z - b.z)
}

/// Preserve the driven centre's elevation when projecting onto a sloping road's 3D curve.
fn probe_point(focus: Focus, distance: f32, angle: f32) -> Vec3 {
    Vec3::new(
        focus.position[0] + distance * angle.cos(),
        focus.position[1] + distance * angle.sin(),
        focus.body.map_or(0.0, |body| body.position.y),
    )
}

/// The spacing from the other cars: a trailer adds half its length to the distance, on either side.
fn clear_of_cars(position: Vec3, length: f32, others: impl Iterator<Item = (Vec3, f32)>) -> bool {
    !others.into_iter().any(|(other, trailer)| planar_distance(position, other) < MIN_GAP + 0.5 * (length + trailer))
}

fn projected_speed(focus: Focus, angle: f32) -> f32 {
    let direction = Vec3::from(space::to_physics(Vec3::new(angle.cos(), angle.sin(), 0.0)));
    match focus.body {
        Some(body) => body.velocity.dot(direction),
        None => focus.speed * (angle - focus.heading).cos(),
    }
}

/// Checks the final lane position, after the random probe has been projected onto the road: it is not in the view of the
/// player exclusion radius, and it keeps the spacing from the other cars.
fn clear_spawn(
    position: Vec3,
    focus: Focus,
    minimum: f32,
    length: f32,
    others: impl Iterator<Item = (Vec3, f32)>,
) -> bool {
    let centre = Vec3::from(space::to_physics(Vec3::new(focus.position[0], focus.position[1], 0.0)));
    if planar_distance(position, centre) < minimum {
        return false;
    }
    clear_of_cars(position, length, others)
}

#[cfg(test)]
mod tests;
