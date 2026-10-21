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
/// The cop manager's spawn window around the player, metres (`docs/specs/ai-pursuit-cops.md` section 3.1).
const COP_MIN_DISTANCE: f32 = 150.0;
const COP_MAX_DISTANCE: f32 = 400.0;
/// The most the player's velocity shifts the patrol probe's disc, metres.
const COP_MAX_SHIFT: f32 = 100.0;
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
        let rng = &mut self.rng.population;
        let probe = match pick {
            Pick::Traffic(_) => traffic_probe(rng, focus, &self.policy),
            Pick::Patrol(_) => patrol_probe(rng, cop_probe_focus(&self.cops, focus)),
        };
        let Probe { distance, at, dir } = probe;
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
        let spaced = match pick {
            Pick::Traffic(_) => clear_spawn(nav.position, focus, self.policy.spawn_min as f32, length, others),
            // A patrol keeps the cop manager's lane spacing and window, measured from the camera.
            Pick::Patrol(_) => {
                cop_spawn_point_valid(&self.cops, nav.position, focus) && cop_lane_clear(&self.cars, &nav)
            }
        };
        if !spaced {
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

/// Where one spawn attempt probes the road: the distance from the player it aims for, the point (render space) and the
/// direction the lane is picked in (radians from +X).
struct Probe {
    distance: f32,
    at: Vec3,
    dir: f32,
}

/// The traffic probe: a wedge ahead of the player, pushed out by the speed along it (`docs/specs/ai-traffic-spawning.md`).
fn traffic_probe(rng: &mut impl RandomSource, focus: Focus, policy: &crate::settings::TrafficPolicy) -> Probe {
    let angle = focus.heading + (rng.next_f32() * 2.0 - 1.0) * WEDGE;
    let projected = projected_speed(focus, angle);
    let distance =
        policy.spawn_min as f32 + rng.next_f32() * (policy.spawn_max - policy.spawn_min) as f32 + projected.max(0.0);
    let oncoming = rng.next_f32() <= 1.0 - 0.5 * (projected / SAME_WAY_SPEED).clamp(0.0, 1.0);
    let dir = match oncoming {
        true => angle + std::f32::consts::PI,
        false => angle,
    };
    Probe { distance, at: probe_point(focus, distance, angle), dir }
}

/// The patrol probe: the cop manager picks a random road segment of the ring around the player (a segment qualifies when
/// an end lies within `COP_MAX_DISTANCE + 50` of the player shifted by their velocity) and a random point along it, so a
/// patrol can appear on any side, not only ahead (`docs/specs/ai-pursuit-cops.md` section 3.1). The probe is a random
/// point of that disc, uniform by area, on either lane direction; the spawn window is then tested on the road point.
fn patrol_probe(rng: &mut impl RandomSource, focus: Focus) -> Probe {
    let angle = rng.next_f32() * std::f32::consts::TAU;
    let (near, far) = (COP_MIN_DISTANCE, COP_MAX_DISTANCE);
    let distance = (near * near + rng.next_f32() * (far * far - near * near)).sqrt();
    let shift = focus.body.map_or(Vec3::ZERO, |body| Vec3::new(body.velocity.x, 0.0, body.velocity.z));
    let shift = space::to_render(shift.to_array());
    let mut at = probe_point(focus, distance, angle);
    at.x += shift.x.clamp(-COP_MAX_SHIFT, COP_MAX_SHIFT);
    at.y += shift.y.clamp(-COP_MAX_SHIFT, COP_MAX_SHIFT);
    let dir = angle + std::f32::consts::PI * f32::from(rng.next_f32() < 0.5);
    Probe { distance, at, dir }
}

/// The cop spawn window and zone rule (`CheckSpawnPosition` with the cop manager's 150 to 400 m): the point on the road
/// is between the two distances from the player horizontally and outside the no-cop-spawn zones.
fn cop_spawn_point_valid(cops: &super::cop_manager::CopManager, position: Vec3, focus: Focus) -> bool {
    let centre = Vec3::from(space::to_physics(Vec3::new(focus.position[0], focus.position[1], 0.0)));
    let gap = cops.window_gap(position, centre);
    (COP_MIN_DISTANCE..=COP_MAX_DISTANCE).contains(&gap) && !cops.bars_spawn(position)
}

/// The lane spacing of `CheckSpawnPosition` for a cop put on the traffic lanes at `nav`: 15 m from a car in the same lane
/// heading the same way (or a wreck), 5 m from a car in another lane, 40 m from a car off the traffic lanes.
pub(super) fn cop_lane_clear(cars: &[AiCar], nav: &RoadNav) -> bool {
    let others = cars.iter().map(|car| (car.physics_position().distance(nav.position), car.spacing_neighbour()));
    blackbox_pursuit::spawn_rules::clear(true, nav.lane, nav.node_ind, others)
}

/// The focus a patrol's probe is drawn around: the camera's place when the world gives it, else the player's.
fn cop_probe_focus(cops: &super::cop_manager::CopManager, focus: Focus) -> Focus {
    let player = Vec3::from(space::to_physics(Vec3::new(focus.position[0], focus.position[1], 0.0)));
    let base = space::to_render(cops.base(player).to_array());
    Focus { position: [base.x, base.y], ..focus }
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
        let index = self.rng.population.index(self.models.len());
        let physics_at = Vec3::from(space::to_physics(Vec3::new(at[0], at[1], 0.0)));
        let dir = Vec3::from(space::to_physics(Vec3::new(heading.cos(), heading.sin(), 0.0)));
        let mut nav = RoadNav::new(NavKind::Direction, LaneType::Racing, SegmentFilter::default());
        if !nav.init_at_point(&self.network, &self.index, physics_at, dir, false)
            || !nav.can_traffic_spawn(&self.network, &mut self.rng.population)
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
        nav.advance_with_lookahead(
            &self.network,
            START_LOOK_AHEAD,
            Vec3::ZERO,
            START_LOOK_AHEAD,
            &mut self.rng.population,
        );
        let cruise = Cruise::default();
        let start = Start {
            stagger: (self.rng.population.next_f32() * 10.0) as u32,
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
