//! The traffic manager's population: which car to add next, which cars to recycle, which models to ask for.
//! Spec: `docs/specs/ai-traffic-spawning.md` (§1, §5, §8).

use blackbox_collision::CollisionWorld;
use blackbox_roads::RandomSource;
use glam::Vec3;
use nfsmw_data::car::physics::SurfaceTable;

use super::manager::{self, Census, TypeTimers};
use super::spawn::Pick;
use super::traffic::{Cruise, MPH};
use super::{Focus, Patterns, TrafficWorld};
use crate::scenes::world::space;

/// Cars farther than this from the player (metres) are removed whether they are seen or not.
const DESPAWN_DISTANCE: f32 = 350.0;
/// A car with no wheel on the ground for this long (seconds) fell off the map and is removed.
const MAX_AIRBORNE: f32 = 4.0;
/// A car standing this long (seconds) is removed when it is more than `STUCK_DISTANCE` metres from the player.
const STUCK_TIME: f32 = 20.0;
const STUCK_DISTANCE: f32 = 40.0;
/// A car within this distance (metres) counts as seen, and one further off within `VIEW_HALF_ANGLE` of the
/// way the player faces.
const ALWAYS_SEEN: f32 = 15.0;
const VIEW_HALF_ANGLE: f32 = 1.05;

/// A car model the manager needs the scene to load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelRequest {
    /// A `pvehicle` name from a traffic pattern.
    Traffic(String),
    /// A cop car that patrols.
    Patrol(String),
}

/// The cop cars that patrol among the traffic and how fast they go.
#[derive(Debug, Clone, PartialEq)]
pub struct PatrolPlan {
    pub cars: Vec<String>,
    pub cruise: Cruise,
}

impl PatrolPlan {
    /// The search-mode speeds are given in miles per hour.
    pub fn new(cars: Vec<String>, city_mph: f32, highway_mph: f32) -> Self {
        Self { cars, cruise: Cruise::patrol(city_mph * MPH, highway_mph * MPH) }
    }
}

struct Cars<'a> {
    cars: &'a [super::AiCar],
    others: u32,
}

impl Census for Cars<'_> {
    fn active(&self, name: &str) -> u32 {
        self.cars.iter().filter(|c| !c.is_cop() && !c.is_patrol() && c.name.eq_ignore_ascii_case(name)).count() as u32
    }

    fn others(&self) -> u32 {
        self.others
    }
}

impl TrafficWorld {
    /// How many cars to keep on the road and what percent of them patrol as cops.
    pub fn configure(&mut self, cars: usize, cop_share_percent: u32) {
        self.target = cars;
        self.cop_share = cop_share_percent.min(100);
    }

    pub fn set_patterns(&mut self, patterns: Patterns) {
        self.patterns = Some(patterns);
        self.timers = TypeTimers::default();
    }

    pub fn set_patrol(&mut self, plan: PatrolPlan) {
        self.patrol = Some(plan);
    }

    /// The models the manager is waiting for; each is returned once.
    pub fn take_model_requests(&mut self) -> Vec<ModelRequest> {
        std::mem::take(&mut self.requests)
    }

    /// A model could not be loaded: its cars are not spawned.
    pub fn model_failed(&mut self, name: &str) {
        log::warn!("traffic: no {name} cars: the model did not load");
        self.known.insert(name.to_ascii_lowercase());
    }

    fn request(&mut self, request: ModelRequest) {
        let name = match &request {
            ModelRequest::Traffic(n) | ModelRequest::Patrol(n) => n.to_ascii_lowercase(),
        };
        if self.known.insert(name) {
            self.requests.push(request);
        }
    }

    fn chasing_cops(&self) -> u32 {
        self.cars.iter().filter(|c| c.is_cop()).count() as u32
    }

    /// One tick of the manager: follow the neighbourhood's pattern, advance the type timers and add at most
    /// one car if there is room for it.
    pub(super) fn manage(&mut self, tick: f32, focus: Focus, collision: &CollisionWorld, surfaces: &SurfaceTable) {
        let density = manager::density(self.target > 0, self.pursuit.is_some());
        if density == 0.0 {
            return;
        }
        let at = focus
            .body
            .map_or(Vec3::from(space::to_physics(Vec3::new(focus.position[0], focus.position[1], 0.0))), |b| {
                b.position
            });
        let mut changed = !self.timers.started() && self.patterns.as_ref().is_some_and(|p| p.current().is_some());
        if let Some(patterns) = self.patterns.as_mut() {
            changed |= patterns.select(at);
        }
        if changed {
            self.start_pattern();
        }
        self.timers.advance(tick, density);

        let traffic = self.cars.iter().filter(|c| !c.is_cop()).count();
        if traffic >= self.target {
            return;
        }
        if self.roll_patrol() {
            self.spawn_patrol(focus, collision, surfaces);
            return;
        }
        let Some(pattern) = self.patterns.as_ref().and_then(|p| p.current()).cloned() else { return };
        let census = Cars { cars: &self.cars, others: 1 + self.chasing_cops() };
        let Some(pick) = self.timers.next_type(&pattern.rules, self.target as u32, &census) else { return };
        let name = pattern.rules[pick].name.clone();
        let Some(model) = self.models.iter().position(|m| m.name.eq_ignore_ascii_case(&name)) else {
            self.request(ModelRequest::Traffic(name));
            return;
        };
        if self.spawn_one(Pick::Traffic(model), pattern.cruise, focus, collision, surfaces) {
            self.timers.spawned(&pattern.rules, &name);
        }
    }

    fn start_pattern(&mut self) {
        let Some(pattern) = self.patterns.as_ref().and_then(|p| p.current()).cloned() else { return };
        log::info!("traffic: pattern {} ({} car types)", pattern.name, pattern.rules.len());
        self.timers.reset(&pattern.rules, &mut self.rng);
        for rule in &pattern.rules {
            if !self.models.iter().any(|m| m.name.eq_ignore_ascii_case(&rule.name)) {
                self.request(ModelRequest::Traffic(rule.name.clone()));
            }
        }
    }

    /// Whether the next car is a patrol cop: one in `cop_share` percent.
    fn roll_patrol(&mut self) -> bool {
        let Some(plan) = self.patrol.as_ref().filter(|p| !p.cars.is_empty()) else { return false };
        if self.cop_share == 0 {
            return false;
        }
        let cars = plan.cars.clone();
        for name in cars {
            if !self.cop_models.iter().any(|m| m.model.name.eq_ignore_ascii_case(&name)) {
                self.request(ModelRequest::Patrol(name));
            }
        }
        self.rng.next_f32() * 100.0 < self.cop_share as f32 && !self.cop_models.is_empty()
    }

    fn spawn_patrol(&mut self, focus: Focus, collision: &CollisionWorld, surfaces: &SurfaceTable) {
        let Some(plan) = self.patrol.as_ref() else { return };
        let candidates: Vec<usize> = (0..self.cop_models.len())
            .filter(|&i| plan.cars.iter().any(|n| n.eq_ignore_ascii_case(&self.cop_models[i].model.name)))
            .collect();
        if candidates.is_empty() {
            return;
        }
        let cruise = plan.cruise;
        let i = candidates[self.rng.index(candidates.len())];
        self.spawn_one(Pick::Patrol(i), cruise, focus, collision, surfaces);
    }

    /// Follows which cars the player can see and removes the ones that are gone for good: far away, seen by
    /// nobody for a while, fallen off the map, or stuck. A tractor that is gone stays while its trailer is not
    /// (spec §8), and a trailer that came loose and is gone is dropped on its own.
    pub(super) fn recycle(&mut self, dt: f32, focus: Focus) {
        let density = manager::density(self.target > 0, self.pursuit.is_some());
        let (distance, time) = (manager::offscreen_distance(density), manager::offscreen_time(density));
        self.cars.retain_mut(|car| {
            if car.is_cop() {
                return car.is_finite();
            }
            car.note_views(dt, |p| in_view(focus, p.x, p.y, away(focus, p)));
            let limits = Limits { time, distance };
            if car.loose_trailer().is_some_and(|t| !Vitals::of_trailer(t).valid(focus, limits)) {
                car.drop_trailer();
            }
            car.is_finite()
                && (Vitals::of_car(car).valid(focus, limits)
                    || car.trailer().is_some_and(|t| Vitals::of_trailer(t).valid(focus, limits)))
        });
    }
}

/// How long out of view and how far away a body may be before it is removed.
#[derive(Clone, Copy)]
struct Limits {
    time: f32,
    distance: f32,
}

/// What decides whether a car, or a trailer, stays on the road.
struct Vitals {
    /// Render space.
    position: Vec3,
    finite: bool,
    airborne: f32,
    idle: f32,
    offscreen: f32,
}

impl Vitals {
    fn of_car(car: &super::AiCar) -> Self {
        Self {
            position: car.position(),
            finite: car.is_finite(),
            airborne: car.airborne_time(),
            idle: car.idle_time(),
            offscreen: car.offscreen_time(),
        }
    }

    fn of_trailer(trailer: &super::trailer::TrailerCar) -> Self {
        Self {
            position: trailer.position(),
            finite: trailer.is_finite(),
            airborne: trailer.airborne_time(),
            idle: trailer.idle_time(),
            offscreen: trailer.offscreen_time(),
        }
    }

    fn valid(&self, focus: Focus, limits: Limits) -> bool {
        let away = away(focus, self.position);
        self.finite
            && self.airborne < MAX_AIRBORNE
            && away < DESPAWN_DISTANCE
            && !(self.idle > STUCK_TIME && away > STUCK_DISTANCE)
            && !(self.offscreen > limits.time && away > limits.distance)
    }
}

/// Distance from the player to `p` (render space) on the map, metres.
fn away(focus: Focus, p: Vec3) -> f32 {
    (p.x - focus.position[0]).hypot(p.y - focus.position[1])
}

/// Whether a car at map position `(x, y)`, `away` metres from the player, is in the player's view: close by,
/// or in the cone the player faces.
fn in_view(focus: Focus, x: f32, y: f32, away: f32) -> bool {
    if away < ALWAYS_SEEN {
        return true;
    }
    let to_car = (y - focus.position[1]).atan2(x - focus.position[0]);
    let mut diff = (to_car - focus.heading).rem_euclid(std::f32::consts::TAU);
    if diff > std::f32::consts::PI {
        diff -= std::f32::consts::TAU;
    }
    diff.abs() < VIEW_HALF_ANGLE
}

#[cfg(test)]
mod tests {
    use super::*;

    fn focus() -> Focus {
        Focus { position: [0.0, 0.0], heading: 0.0, speed: 0.0, body: None }
    }

    #[test]
    fn close_cars_and_cars_ahead_are_seen() {
        let f = focus();
        assert!(in_view(f, 5.0, 5.0, 7.0), "close");
        assert!(in_view(f, 100.0, 10.0, 100.5), "ahead");
        assert!(!in_view(f, -100.0, 0.0, 100.0), "behind");
        assert!(!in_view(f, 0.0, 100.0, 100.0), "to the side");
    }

    #[test]
    fn the_patrol_speeds_are_converted_from_mph() {
        let plan = PatrolPlan::new(vec!["copmidsize".into()], 50.0, 71.0);
        assert!((plan.cruise.street - 22.352).abs() < 1e-2);
        assert!(plan.cruise.highway > plan.cruise.street);
        assert!(!plan.cruise.limit_acceleration);
    }
}
