//! An AI racer's think: path to the next checkpoint, steer point past traffic, speed from the car's limits and the
//! skill. Specs: `docs/specs/ai-racers.md`, `docs/specs/ai-racers-route.md`, `docs/specs/ai-driver-speed-skill.md`.

use blackbox_driver::{
    CarLimits, NitrousLimits, NitrousTimer, NitrousView, SpeedGovernor, StuckDetector, potential_acceleration,
    potential_speed, ramp,
};
use blackbox_roads::{
    PathState, PathType, RandomSource, RoadNav, RoadNetwork, SegmentIndex, trail_curvature, update_occluded_position,
};
use blackbox_vehicle::performance::Performance;
use glam::Vec3;
use nfsmw_data::pursuit::AiVehicle;

use super::car::Ctx;
use super::racer_route::follow_route;
use super::traffic::CarState;

#[cfg(test)]
mod installed_tests;
mod off_path;
#[cfg(test)]
mod opening_roll_tests;
#[cfg(test)]
mod pace_tests;

/// Racers decide at 30 Hz in the 60 Hz simulation (`ai-simulation-lod.md`, section 4).
pub const RACER_THINK_STEPS: u32 = 2;
const THINK_PERIOD: f32 = RACER_THINK_STEPS as f32 / 60.0;
/// The path to the target is searched again this often, seconds.
const REPATH_PERIOD: f32 = 1.0;
/// How hard a racer's stand-in brakes, m/s² (`docs/specs/ai-simulation-lod.md` section 3.2).
const STAND_IN_BRAKING: f32 = 30.0;
const CATCH_UP_ACCELERATION: blackbox_driver::Table<4> =
    blackbox_driver::Table { data: [1.0, 1.33, 1.66, 2.0], min: 0.0, max: 1.0 };
/// The look-ahead follows the requested speed: 30 m at rest to 100 m at 100 m/s.
fn look_ahead(speed_limit: f32) -> f32 {
    30.0 + 70.0 * ramp(speed_limit, 0.0, 100.0)
}

/// The limits of a racer after matching it to the player's car. `bias` is the character's `MinimumAIPerformance`: 0
/// takes the weaker of the two cars, 1 keeps the racer's own numbers, in between mixes them
/// (`docs/specs/ai-racers.md` section 2).
pub fn matched_limits(own: &Performance, ai: &AiVehicle, player: &Performance, bias: f32) -> CarLimits {
    let bias = bias.clamp(0.0, 1.0);
    let mix = |own: f32, theirs: f32| {
        let weaker = own.min(theirs);
        weaker + (own - weaker) * bias
    };
    CarLimits {
        top_speed: mix(own.top_speed, player.top_speed),
        start_grip: mix(own.start_grip, player.start_grip),
        end_grip: mix(own.end_grip, player.end_grip),
        top_speed_multiplier: ai.top_speed_multiplier,
        acceleration_multiplier: ai.acceleration_multiplier,
    }
}

/// One racer's state.
#[derive(Debug, Clone)]
pub struct RacerState {
    /// The racer's index in the race.
    pub id: usize,
    performance: Performance,
    player: Performance,
    limits: CarLimits,
    governor: SpeedGovernor,
    think_delta: f32,
    pub stuck: StuckDetector,
    /// The next gate, in physics space; `None` after the finish (the racer cruises on).
    goal: Option<Vec3>,
    repath: f32,
    path_state: Option<PathState>,
    /// The base skill of the character (0 to 1) and the catch-up offset added to it.
    pub base_skill: f32,
    pub glue: f32,
    /// Held on the grid until GO.
    pub held: bool,
    /// Native drag style excludes the non-drag racer steering-drag rule.
    pub drag: bool,
    /// Where the racer was told to cruise to after finishing: a speed (m/s).
    cruise: f32,
    /// The avoidance radius the racer advertises to the other cars: its last road look-ahead.
    avoid_radius: f32,
    /// The road the racer follows through the gates, with the shortcuts it rolled for (`race/shortcuts.rs`); `None`
    /// before the race gives it one, or without a road network.
    pub route: Option<std::sync::Arc<blackbox_roads::Route>>,
    /// The leg of the route the racer is on (`race/route.rs`, `leg_of`); `None` after the finish.
    leg: Option<usize>,
    /// The last search put the navigator on the route (`racer_route.rs`).
    on_route: bool,
    /// A person drives this racer (split screen): the pedals they hold this frame. The think, the driver, the emulation
    /// and the catch-up leave the racer alone (`docs/specs/split-screen.md` section 3).
    pub human: Option<crate::scenes::world::drive::DriveInput>,
    /// The nitrous the racer may use, its on/off timer, the button it holds, and its speed at the last think (for its
    /// acceleration), `docs/specs/ai-driver-speed-skill.md` section 7.1.
    nitrous: NitrousLimits,
    nitrous_timer: NitrousTimer,
    nitrous_on: bool,
    last_speed: Option<f32>,
}

/// What a racer's think decided.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RacerThink {
    pub target: Vec3,
    pub speed: f32,
}

/// The car as the think sees it beyond its state: the slope under it (the forward axis's height), the nitrous tank
/// level (0..1) and whether the nitrous button is held.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TankView {
    pub slope_forward_y: f32,
    pub tank: f32,
    pub held: bool,
}

impl RacerState {
    /// Releases an authored rolling start with the driver already moving at its carried speed.
    pub(super) fn roll_from_grid(&mut self, speed: f32) {
        self.held = false;
        self.governor = SpeedGovernor::begin(speed);
        self.last_speed = Some(speed);
    }

    pub(super) fn set_think_delta(&mut self, dt: f32) {
        self.think_delta = dt;
    }

    /// A temporary player autopilot preserves the speed supplied by its ownership handoff.
    pub(super) fn set_cruise_speed(&mut self, speed: f32) {
        self.cruise = speed.max(0.0);
    }

    pub fn new(
        id: usize,
        performance: Performance,
        ai: AiVehicle,
        player: Performance,
        bias: f32,
        base_skill: f32,
        speed: f32,
    ) -> Self {
        Self {
            nitrous: NitrousLimits::default(),
            nitrous_timer: NitrousTimer::default(),
            nitrous_on: false,
            last_speed: None,
            id,
            limits: matched_limits(&performance, &ai, &player, bias),
            performance,
            player,
            governor: SpeedGovernor::begin(speed),
            think_delta: THINK_PERIOD,
            stuck: StuckDetector::default(),
            goal: None,
            repath: 0.0,
            path_state: None,
            base_skill: base_skill.clamp(0.0, 1.0),
            glue: 0.0,
            held: true,
            drag: false,
            cruise: 25.0,
            avoid_radius: blackbox_roads::DEFAULT_AVOID_RADIUS,
            route: None,
            leg: None,
            on_route: false,
            human: None,
        }
    }

    /// The skill the driver uses now: the base skill plus the catch-up offset, kept in 0..1
    /// (`docs/specs/ai-driver-speed-skill.md` section 8.1).
    pub fn skill(&self) -> f32 {
        (self.base_skill + self.glue).clamp(0.0, 1.0)
    }

    /// The avoidance radius the racer advertises: the look-ahead of its last road think (`DEFAULT_AVOID_RADIUS` before
    /// one).
    pub fn avoid_radius(&self) -> f32 {
        self.avoid_radius
    }

    /// Native race catch-up force budget; humans never receive it.
    pub fn cheat(&self) -> f32 {
        if self.id == 0 || self.human.is_some() {
            return 0.0;
        }
        (self.base_skill + self.glue - 1.0).clamp(0.0, 1.0) * 0.5
    }

    pub fn steering_drag(&self) -> Option<f32> {
        if self.id == 0 || self.human.is_some() || self.drag {
            return None;
        }
        Some(0.23 + (0.5 - 0.23) * self.cheat())
    }

    fn action_acceleration(&self, speed: f32, slope_forward_y: f32, burning: bool) -> f32 {
        let table = self.performance.acceleration_at(speed).min(self.player.acceleration_at(speed));
        let scale = CATCH_UP_ACCELERATION.lookup(self.cheat());
        let nos = burning.then_some(self.nitrous.boost);
        potential_acceleration(table * scale, &self.limits, self.skill(), nos, slope_forward_y)
    }

    /// Gives the racer the nitrous it may use.
    pub fn with_nitrous(mut self, nitrous: NitrousLimits) -> Self {
        self.nitrous = nitrous;
        self
    }

    /// The nitrous button the racer holds (its last think's choice).
    pub fn nitrous(&self) -> bool {
        self.nitrous_on
    }

    /// Where the racer drives to (physics space). `None` once it has finished.
    pub fn set_goal(&mut self, goal: Option<Vec3>) {
        if goal != self.goal {
            self.repath = 0.0;
        }
        self.goal = goal;
    }

    /// The leg of its route the racer is on now. A new leg searches again at once.
    pub fn set_leg(&mut self, leg: Option<usize>) {
        if leg != self.leg {
            self.repath = 0.0;
        }
        self.leg = leg;
    }

    /// The speed of the racer's stand-in after `dt` seconds at `current` m/s, when the think asks for `wanted`
    /// (`ai/car/stand_in.rs`).
    pub fn stand_in_speed(&self, current: f32, wanted: f32, dt: f32) -> f32 {
        let accel = self.performance.acceleration_at(current);
        stand_in_speed(current, wanted, accel, self.limits.top_speed, self.skill(), dt)
    }

    /// The think: a path to the goal, the look-ahead point and the requested speed.
    pub fn think<R: RandomSource>(
        &mut self,
        ctx: &mut Ctx<'_, R>,
        nav: &mut RoadNav,
        car: &CarState,
        traffic: impl IntoIterator<Item: Into<blackbox_roads::Neighbour>>,
        check_off_path: bool,
        tank: TankView,
    ) -> RacerThink {
        let (net, index, rng) = (ctx.net, ctx.index, &mut *ctx.rng);
        let Some(goal) = self.goal else {
            return self.cruise_think(ctx, nav, car, traffic);
        };
        self.repath -= self.think_delta;
        let to_goal = Vec3::new(goal.x - car.body.position.x, 0.0, goal.z - car.body.position.z);
        if self.repath <= 0.0 || nav.dead_end {
            self.repath = REPATH_PERIOD;
            self.repath_to(net, index, nav, car, goal);
        }
        let look = look_ahead(self.governor.speed_limit);
        self.avoid_radius = look;
        let gap = nav.position.distance(car.body.position);
        let toward = to_goal.try_normalize().unwrap_or(Vec3::ZERO);
        if gap < look {
            nav.advance_with_lookahead(net, look - gap, toward, look, rng);
        } else if !self.on_route
            && gap > 2.5 * look
            && nav.init_at_point(
                net,
                index,
                car.body.position,
                Vec3::new(car.body.forward.x, 0.0, car.body.forward.y),
                false,
            )
        {
            nav.enable_trail(net);
            self.repath = 0.0;
        }
        if check_off_path && let Some(fresh) = off_path::candidate(net, index, nav, &car.body) {
            log::debug!(
                "racer {}: off its path at ({:.0}, {:.0}); segment {} -> {}",
                self.id,
                car.body.position.x,
                car.body.position.z,
                nav.segment,
                fresh.segment
            );
            *nav = fresh;
            self.repath_to(net, index, nav, car, goal);
            self.repath = REPATH_PERIOD;
            let gap = nav.position.distance(car.body.position);
            nav.advance_with_lookahead(net, (look - gap).max(0.0), toward, look, rng);
        }
        // Staging on the grid: a walking-pace request at the plain road point (`ai-driver-speed-skill.md` section 1).
        let burning = tank.held && tank.tank > 0.0;
        let acceleration = self.last_speed.map_or(0.0, |last| (car.forward_speed - last) / self.think_delta.max(1e-3));
        self.last_speed = Some(car.forward_speed);
        if self.held {
            self.nitrous_on = false;
            self.nitrous_timer.seconds = (self.nitrous_timer.seconds - self.think_delta).min(0.0);
            let speed = self.governor.stage(car.forward_speed, self.skill());
            return RacerThink { target: nav.position, speed };
        }
        let occlusion = nav.trail().and_then(|trail| {
            update_occluded_position(trail, &car.body, nav.position, nav.forward, traffic, false, nav.half_width)
        });
        let curvature = match (&occlusion, nav.trail()) {
            (Some(o), Some(trail)) => {
                trail_curvature(trail.cookies(), o.current_index, car.body.position, o, nav.position, car.forward_speed)
            }
            _ => nav.curvature,
        };
        let speed = car.forward_speed;
        let skill = self.skill();
        let potential = potential_speed(&self.limits, curvature, skill, None);
        let reference = speed.abs().max(self.governor.speed_limit);
        let accel = self.action_acceleration(reference, tank.slope_forward_y, burning);
        let wanted = self.governor.update(speed, potential, accel, skill, self.think_delta);
        let view = NitrousView {
            speed,
            speed_limit: wanted,
            potential_speed: potential,
            potential_accel: accel,
            acceleration,
            skill,
            burning,
            tank: tank.tank,
            barred: occlusion.as_ref().is_some_and(|o| o.occluded()),
        };
        self.nitrous_on = self.nitrous_timer.decide(&self.nitrous, &view, self.think_delta);
        let target = occlusion.map_or(nav.position, |o| o.position);
        log::trace!(
            "racer {}: at ({:.0}, {:.0}) segment {} on_route {} target ({:.0}, {:.0}) speed {wanted:.1}",
            self.id,
            car.body.position.x,
            car.body.position.z,
            nav.segment,
            self.on_route,
            target.x,
            target.z
        );
        RacerThink { target, speed: wanted }
    }

    /// Searches the racer's road again: along its route (`racer_route.rs`), or, on a leg with no road or with no route,
    /// the shortest road to `goal`.
    fn repath_to(&mut self, net: &RoadNetwork, index: &SegmentIndex, nav: &mut RoadNav, car: &CarState, goal: Vec3) {
        let at = car.body.position;
        let follow = match (&self.route, self.leg) {
            (Some(route), Some(leg)) => follow_route((net, index), route, leg, nav, at),
            _ => None,
        };
        self.on_route = follow.is_some();
        if let Some(follow) = follow {
            log::debug!("racer {}: on its route, leg {:?}: {follow:?}", self.id, self.leg);
            nav.dead_end = false;
            return;
        }
        let state = nav.find_path_to(net, index, goal, None, PathType::Racer);
        self.path_state = Some(state);
        log::debug!(
            "racer {}: path from ({:.0}, {:.0}) to ({:.0}, {:.0}): {state:?}, {} segment(s)",
            self.id,
            at.x,
            at.z,
            goal.x,
            goal.z,
            nav.path.len()
        );
        if state == PathState::NoWay {
            nav.dead_end = false;
        }
    }

    /// After the finish: follow the road at a cruising speed.
    fn cruise_think<R: RandomSource>(
        &mut self,
        ctx: &mut Ctx<'_, R>,
        nav: &mut RoadNav,
        car: &CarState,
        traffic: impl IntoIterator<Item: Into<blackbox_roads::Neighbour>>,
    ) -> RacerThink {
        let (net, rng) = (ctx.net, &mut *ctx.rng);
        let look = look_ahead(self.cruise);
        self.avoid_radius = look;
        let gap = nav.position.distance(car.body.position);
        if gap < look {
            nav.advance_with_lookahead(net, look - gap, Vec3::ZERO, look, rng);
        }
        let occlusion = nav.trail().and_then(|trail| {
            update_occluded_position(trail, &car.body, nav.position, nav.forward, traffic, false, nav.half_width)
        });
        let target = occlusion.map_or(nav.position, |o| o.position);
        let wanted = self.governor.update(car.forward_speed, self.cruise, 4.0, 0.5, self.think_delta);
        RacerThink { target, speed: wanted.min(self.cruise) }
    }
}

/// The speed of a stand-in after `dt` seconds (`docs/specs/ai-simulation-lod.md` section 3.2): it brakes at 30 m/s²
/// down to `wanted`, or speeds up with the car's acceleration `accel`, half of it for the weakest skill, up to
/// `wanted`; never above `lerp(0.75, 1, skill)` of its top speed.
pub fn stand_in_speed(current: f32, wanted: f32, accel: f32, top_speed: f32, skill: f32, dt: f32) -> f32 {
    let skill = skill.clamp(0.0, 1.0);
    let speed = match current > wanted {
        true => (current - STAND_IN_BRAKING * dt).max(wanted),
        false => (current + accel * dt * (0.5 + 0.5 * skill)).min(wanted),
    };
    speed.clamp(0.0, top_speed * (0.75 + 0.25 * skill))
}

#[cfg(test)]
mod tests;
