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
use crate::scenes::world::props::PropWorld;
use crate::scenes::world::space;
use crate::viewer::camera::Lens;

mod cops;
mod guests;
pub(super) use guests::view_distance;
mod visibility;
use visibility::Viewing;

// The rewrite's own removal rules (spec §8 says the original has none of them). They apply only while the car is
// unseen, so a visible car is never removed by them.
/// Rewrite extension: an unseen car farther than this from the player (metres) is removed.
const DESPAWN_DISTANCE: f32 = 650.0;
/// Rewrite extension: an unseen car with no wheel on the ground for this long (seconds) fell off the map.
const MAX_AIRBORNE: f32 = 4.0;
/// Rewrite extension: an unseen car standing this long (seconds) is removed when it is more than
/// `STUCK_DISTANCE` metres from the player.
const STUCK_TIME: f32 = 20.0;
const STUCK_DISTANCE: f32 = 40.0;

/// A car model the manager needs the scene to load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelRequest {
    /// A `pvehicle` name from a traffic pattern.
    Traffic(String),
    /// A cop car that patrols.
    Patrol(String),
    /// A cop car a support request takes (`ai/support`).
    Support(String),
}

/// The cop cars that patrol among the traffic and how fast they go.
#[derive(Debug, Clone, PartialEq)]
pub struct PatrolPlan {
    pub cars: Vec<String>,
    pub cruise: Cruise,
    pub search_cruise: Cruise,
    pub levels: Vec<nfsmw_data::pursuit::HeatRow>,
}

impl PatrolPlan {
    /// Search-mode speeds are distinct from the normal patrol action's 35/55 mph targets.
    pub fn new(cars: Vec<String>, city_mph: f32, highway_mph: f32) -> Self {
        Self {
            cars,
            cruise: Cruise::patrol(35.0 * MPH, 55.0 * MPH),
            search_cruise: Cruise::patrol(city_mph * MPH, highway_mph * MPH),
            levels: Vec::new(),
        }
    }

    fn cars_at(&self, heat: u32) -> Vec<String> {
        let Some(row) = self.levels.get(heat.saturating_sub(1) as usize) else { return self.cars.clone() };
        nfsmw_data::pursuit::ground_cops(row).map(|cop| cop.name.clone()).collect()
    }

    fn limit(&self, heat: u32) -> usize {
        self.levels.get(heat.saturating_sub(1) as usize).map_or(1, |row| row.patrol_cars as usize)
    }
}

struct Cars<'a> {
    cars: &'a [super::AiCar],
    models: &'a [super::TrafficModel],
    others: u32,
}

impl Census for Cars<'_> {
    fn active(&self, name: &str) -> u32 {
        self.cars.iter().filter(|c| !c.is_cop() && !c.is_patrol() && c.name.eq_ignore_ascii_case(name)).count() as u32
    }

    fn others(&self) -> u32 {
        self.others
    }
    fn trailer_slots(&self, name: &str) -> u32 {
        u32::from(self.models.iter().any(|m| m.name.eq_ignore_ascii_case(name) && m.trailer.is_some()))
    }
}

impl TrafficWorld {
    pub(super) fn patrol_enabled(&self) -> bool {
        self.cop_share > 0 && self.patrol.is_some()
    }

    pub(super) fn search_cruise(&self) -> Cruise {
        if let Some(row) = self.police.level().and_then(|level| self.police.level_tuning(level)) {
            return Cruise::patrol(row.search_city_mph * MPH, row.search_highway_mph * MPH);
        }
        self.patrol.as_ref().map_or(Cruise::patrol(35.0 * MPH, 55.0 * MPH), |p| p.search_cruise)
    }

    /// Active vehicle budget and the legacy patrol switch (zero disables police patrols).
    pub fn configure(&mut self, cars: usize, cop_share_percent: u32) {
        self.target = cars.min(64);
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
        self.failed.insert(name.to_ascii_lowercase());
    }

    /// Asks for the traffic model `name` unless it is loaded or already asked for (a racer's car).
    pub(super) fn request_traffic_model(&mut self, name: &str) {
        self.request(ModelRequest::Traffic(name.to_owned()));
    }

    fn request(&mut self, request: ModelRequest) {
        let name = match &request {
            ModelRequest::Traffic(n) | ModelRequest::Patrol(n) | ModelRequest::Support(n) => n.to_ascii_lowercase(),
        };
        if self.known.insert(name) {
            self.requests.push(request);
        }
    }

    /// Asks for the cop car `name` of a support request, unless it is loaded or already asked for. Unlike `request`, a
    /// name that failed in an earlier chase is asked for again: each chase tries its own support models.
    pub(super) fn request_cop(&mut self, name: &str) {
        if self.has_cop_model(name) {
            return;
        }
        let request = ModelRequest::Support(name.to_owned());
        if !self.requests.contains(&request) {
            self.requests.push(request);
        }
    }

    /// One tick of the manager: follow the neighbourhood's pattern, advance the type timers and add at most
    /// one car if there is room for it.
    pub(super) fn manage(&mut self, tick: f32, focus: Focus, collision: &CollisionWorld, surfaces: &SurfaceTable) {
        if self.world_moment {
            return;
        }
        if self.needs_patrol() && self.spawn_patrol(focus, collision, surfaces) {
            return;
        }
        let density = self.density();
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
        if let Some(pattern) = self.patterns.as_ref().and_then(|p| p.current()) {
            self.timers.advance_population(tick, density, &pattern.rules, self.policy.common_rate_percent);
        }

        if self.population_slots(focus) >= self.target {
            return;
        }
        let Some(pattern) = self.patterns.as_ref().and_then(|p| p.current()).cloned() else { return };
        let census = Cars { cars: &self.cars, models: &self.models, others: self.other_slots(focus) as u32 };
        let Some(pick) =
            self.timers.next_population(&pattern.rules, self.target as u32, &census, self.policy.common_fill)
        else {
            return;
        };
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
        self.timers.reset(&pattern.rules, &mut self.rng.population);
        for rule in &pattern.rules {
            if !self.models.iter().any(|m| m.name.eq_ignore_ascii_case(&rule.name)) {
                self.request(ModelRequest::Traffic(rule.name.clone()));
            }
        }
    }

    /// A separate patrol budget, bounded by the installed heat row. A full traffic population cannot starve it.
    fn needs_patrol(&mut self) -> bool {
        let Some(plan) = self.patrol.as_ref().filter(|p| !p.cars.is_empty()) else { return false };
        if self.cop_share == 0 {
            return false;
        }
        // No new patrol while a spawn request waits to be placed (`wantpatrolcars = 0`).
        if self.pursuit.is_some()
            || !self.police.can_begin()
            || !self.cops.spawning_enabled()
            || !self.cops.requests.is_empty()
        {
            return false;
        }
        let heat = self.police.patrol_heat();
        let limit = plan.limit(heat);
        if self.cars.iter().filter(|car| car.is_patrol() && !car.is_wrecked()).count() >= limit {
            return false;
        }
        let cars = plan.cars_at(heat);
        for name in cars {
            if !self.cop_models.iter().any(|m| m.model.name.eq_ignore_ascii_case(&name)) {
                self.request(ModelRequest::Patrol(name));
            }
        }
        !self.cop_models.is_empty()
    }

    fn spawn_patrol(&mut self, focus: Focus, collision: &CollisionWorld, surfaces: &SurfaceTable) -> bool {
        let Some(plan) = self.patrol.as_ref() else { return false };
        let cars = plan.cars_at(self.police.patrol_heat());
        let candidates: Vec<usize> = (0..self.cop_models.len())
            .filter(|&i| cars.iter().any(|n| n.eq_ignore_ascii_case(&self.cop_models[i].model.name)))
            .collect();
        if candidates.is_empty() {
            return false;
        }
        let cruise = plan.cruise;
        let i = candidates[self.rng.population.index(candidates.len())];
        self.spawn_one(Pick::Patrol(i), cruise, focus, collision, surfaces)
    }

    /// Follows which parts of the cars the camera sees, then removes the cars that are gone for good (spec §8):
    /// unseen for longer than the density's time and farther than its distance, or, while unseen, the rewrite's
    /// extra rules. A car with a detached part in view stays. A tractor that is gone stays while its trailer is
    /// not, and a trailer that came loose and is gone is dropped on its own.
    pub(super) fn recycle(&mut self, dt: f32, focus: Focus, camera: &Lens, shown: &Lens, props: &PropWorld) {
        let density = self.density();
        let limits = Limits { time: manager::offscreen_time(density), distance: manager::offscreen_distance(density) };
        let mut view = Viewing::new(camera, focus).also_shown_through(shown);
        view.retire_distance = self.policy.retire_distance as f32;
        view.grace = self.policy.offscreen_seconds as f32;
        let base = cops::Base::new(camera, focus, self.pursuit.is_some());
        // A co-op guest's view keeps a traffic car as the lead's does; the police follow the lead's.
        let guests = self.guest_views();
        let views = || std::iter::once(&view).chain(&guests);
        self.cars.retain_mut(|car| {
            if car.is_racer() {
                return car.is_finite();
            }
            car.note_views(dt, |p, radius| views().any(|v| v.sees(p, radius)));
            if car.is_police() {
                return cops::keeps(car, &view, &base, MAX_AIRBORNE);
            }
            if car.loose_trailer().is_some_and(|t| !views().any(|v| Vitals::of_trailer(t).valid(v, limits))) {
                car.drop_trailer();
            }
            let valid = |v: &Viewing| {
                Vitals::of_car(car, parts_seen(car, props, v)).valid(v, limits)
                    || car.trailer().is_some_and(|t| Vitals::of_trailer(t).valid(v, limits))
            };
            let keep = car.is_finite() && views().any(valid);
            if !keep {
                log::debug!("traffic retired: {} at {:?}; finite={}, unseen={:.1}s, distance={:.0}m, airborne={:.1}s, idle={:.1}s", car.name, car.position(), car.is_finite(), car.offscreen_time(), view.away(car.position()), car.airborne_time(), car.idle_time());
            }
            keep
        });
    }
}

/// Whether a detached part of the car is in the camera's view (its parts live in the prop world).
fn parts_seen(car: &super::AiCar, props: &PropWorld, view: &Viewing) -> bool {
    car.part_owner().is_some_and(|owner| props.fragment_spheres(&owner).into_iter().any(|(c, r)| view.sees(c, r)))
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
    /// A detached part of the car is in view: the car stays (spec §8).
    detached_seen: bool,
    /// A traffic car blocked and standing for `ai-traffic-obstacles.md` §10.4's 60 s, with the avoidance setting on.
    stuck: bool,
}

impl Vitals {
    fn of_car(car: &super::AiCar, detached_seen: bool) -> Self {
        Self {
            position: car.position(),
            finite: car.is_finite(),
            airborne: car.airborne_time(),
            idle: car.idle_time(),
            offscreen: car.offscreen_time(),
            detached_seen,
            stuck: car.blocked_clock().is_some_and(|clock| clock.stuck()),
        }
    }

    fn of_trailer(trailer: &super::trailer::TrailerCar) -> Self {
        Self {
            position: trailer.position(),
            finite: trailer.is_finite(),
            airborne: trailer.airborne_time(),
            idle: trailer.idle_time(),
            offscreen: trailer.offscreen_time(),
            // Trailers have no part owner of their own in this port (spec §8 keep-alive is for the car's model).
            detached_seen: false,
            stuck: false,
        }
    }

    /// Whether the car may stay. A visible detached part keeps it. A seen car always stays (its off-screen time is
    /// 0 after this tick's update). An unseen car is removed by the original's rule (unseen longer than
    /// `limits.time` and more than `limits.distance` away), or by the rewrite's extra rules: airborne, far beyond
    /// the selected maximum range, or stuck. Ordinary removal waits for the selected offscreen grace and, in the
    /// rewrite, for the car to leave the minimap, so turning round finds the cars that were behind **[ours]**;
    /// invalid/off-world recovery remains exceptional. Visible finite cars stay protected.
    fn valid(&self, view: &Viewing, limits: Limits) -> bool {
        if !self.finite {
            return false;
        }
        if self.detached_seen {
            return true;
        }
        if self.offscreen <= 0.0 {
            return true;
        }
        let gone = self.airborne >= MAX_AIRBORNE
            || (self.offscreen > view.grace && view.away(self.position) >= view.retire_distance)
            || (view.forgotten(self.offscreen, self.position)
                && self.idle > STUCK_TIME
                && view.away(self.position) > STUCK_DISTANCE);
        let unseen_too_long = (self.offscreen > limits.time.max(view.grace)
            || (self.stuck && self.offscreen > view.grace))
            && view.distance(self.position) > limits.distance
            && view.forgotten(self.offscreen, self.position);
        !(gone || unseen_too_long)
    }
}

#[cfg(test)]
mod tests;
