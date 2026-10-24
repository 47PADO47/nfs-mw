//! An AI racer's think: path to the next checkpoint, steer point past traffic, speed from the car's limits and the
//! skill. Specs: `docs/specs/ai-racers.md`, `docs/specs/ai-racers-route.md`, `docs/specs/ai-driver-speed-skill.md`.

use blackbox_driver::{CarLimits, SpeedGovernor, StuckDetector, potential_acceleration, potential_speed, ramp};
use blackbox_roads::{
    PathState, PathType, RandomSource, RoadNav, RoadNetwork, SegmentIndex, trail_curvature, update_occluded_position,
};
use blackbox_vehicle::performance::Performance;
use glam::Vec3;
use nfsmw_data::pursuit::AiVehicle;

use super::car::Ctx;
use super::cop::{THINK_PERIOD, THINK_STEPS};
use super::racer_route::follow_route;
use super::traffic::CarState;
use blackbox_roads::Body;

/// Racers think as often as cops do.
pub const RACER_THINK_STEPS: u32 = THINK_STEPS;
/// The path to the target is searched again this often, seconds.
const REPATH_PERIOD: f32 = 1.0;
/// How hard a racer's stand-in brakes, m/s² (`docs/specs/ai-simulation-lod.md` section 3.2).
const STAND_IN_BRAKING: f32 = 30.0;
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
    /// Where the racer was told to cruise to after finishing: a speed (m/s).
    cruise: f32,
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
}

/// What a racer's think decided.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RacerThink {
    pub target: Vec3,
    pub speed: f32,
}

impl RacerState {
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
            id,
            limits: matched_limits(&performance, &ai, &player, bias),
            performance,
            player,
            governor: SpeedGovernor::begin(speed),
            stuck: StuckDetector::default(),
            goal: None,
            repath: 0.0,
            path_state: None,
            base_skill: base_skill.clamp(0.0, 1.0),
            glue: 0.0,
            held: true,
            cruise: 25.0,
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
        traffic: impl IntoIterator<Item = Body>,
    ) -> RacerThink {
        let (net, index, rng) = (ctx.net, ctx.index, &mut *ctx.rng);
        let Some(goal) = self.goal else {
            return self.cruise_think(ctx, nav, car, traffic);
        };
        self.repath -= THINK_PERIOD;
        let to_goal = Vec3::new(goal.x - car.body.position.x, 0.0, goal.z - car.body.position.z);
        if self.repath <= 0.0 || nav.dead_end {
            self.repath = REPATH_PERIOD;
            self.repath_to(net, index, nav, car, goal);
        }
        let look = look_ahead(self.governor.speed_limit);
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
        let occlusion = nav
            .trail()
            .and_then(|trail| update_occluded_position(trail, &car.body, nav.position, traffic, false, nav.half_width));
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
        let table = self.performance.acceleration_at(reference).min(self.player.acceleration_at(reference));
        let accel = potential_acceleration(table, &self.limits, skill, None, 0.0);
        let wanted = self.governor.update(speed, potential, accel, skill, THINK_PERIOD);
        RacerThink { target: occlusion.map_or(nav.position, |o| o.position), speed: wanted }
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
        traffic: impl IntoIterator<Item = Body>,
    ) -> RacerThink {
        let (net, rng) = (ctx.net, &mut *ctx.rng);
        let look = look_ahead(self.cruise);
        let gap = nav.position.distance(car.body.position);
        if gap < look {
            nav.advance_with_lookahead(net, look - gap, Vec3::ZERO, look, rng);
        }
        let occlusion = nav
            .trail()
            .and_then(|trail| update_occluded_position(trail, &car.body, nav.position, traffic, false, nav.half_width));
        let target = occlusion.map_or(nav.position, |o| o.position);
        let wanted = self.governor.update(car.forward_speed, self.cruise, 4.0, 0.5, THINK_PERIOD);
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
mod tests {
    use super::*;

    #[test]
    fn a_stand_in_speeds_up_by_skill_and_brakes_hard() {
        assert!((stand_in_speed(10.0, 50.0, 4.0, 80.0, 1.0, 0.5) - 12.0).abs() < 1e-5);
        assert!((stand_in_speed(10.0, 50.0, 4.0, 80.0, 0.0, 0.5) - 11.0).abs() < 1e-5, "half the pull");
        assert_eq!(stand_in_speed(49.0, 50.0, 4.0, 80.0, 1.0, 1.0), 50.0, "never past the wanted speed");
        assert_eq!(stand_in_speed(50.0, 20.0, 4.0, 80.0, 1.0, 0.5), 35.0, "30 m/s² of braking");
        assert_eq!(stand_in_speed(50.0, 49.0, 4.0, 80.0, 1.0, 0.5), 49.0);
    }

    #[test]
    fn a_stand_in_never_beats_its_skills_share_of_the_top_speed() {
        assert_eq!(stand_in_speed(70.0, 90.0, 4.0, 80.0, 0.0, 1.0), 60.0);
        assert_eq!(stand_in_speed(70.0, 90.0, 40.0, 80.0, 1.0, 1.0), 80.0);
        assert_eq!(stand_in_speed(0.0, -5.0, 4.0, 80.0, 1.0, 1.0), 0.0, "it does not reverse");
    }

    fn performance(top: f32, grip: f32) -> Performance {
        Performance {
            top_speed: top,
            acceleration: [5.0; blackbox_vehicle::performance::TABLE_SIZE],
            start_grip: grip,
            end_grip: grip,
        }
    }

    #[test]
    fn a_bias_of_zero_matches_the_racer_down_to_the_player() {
        let racer = performance(90.0, 1.4);
        let player = performance(70.0, 1.0);
        let limits = matched_limits(&racer, &AiVehicle::default(), &player, 0.0);
        assert_eq!((limits.top_speed, limits.start_grip), (70.0, 1.0));
    }

    #[test]
    fn a_bias_of_one_keeps_the_racers_own_numbers() {
        let racer = performance(90.0, 1.4);
        let player = performance(70.0, 1.0);
        let limits = matched_limits(&racer, &AiVehicle::default(), &player, 1.0);
        assert_eq!((limits.top_speed, limits.start_grip), (90.0, 1.4));
    }

    #[test]
    fn a_weaker_racer_is_never_raised() {
        let racer = performance(60.0, 0.9);
        let player = performance(70.0, 1.0);
        for bias in [0.0, 0.5, 1.0] {
            let limits = matched_limits(&racer, &AiVehicle::default(), &player, bias);
            assert_eq!((limits.top_speed, limits.start_grip), (60.0, 0.9), "bias {bias}");
        }
    }

    #[test]
    fn the_skill_adds_the_catch_up_and_stays_in_range() {
        let mut state =
            RacerState::new(1, performance(80.0, 1.2), AiVehicle::default(), performance(80.0, 1.2), 1.0, 0.8, 0.0);
        state.glue = 0.5;
        assert_eq!(state.skill(), 1.0);
        state.glue = -1.0;
        assert_eq!(state.skill(), 0.0);
    }
}
