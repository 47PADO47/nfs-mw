//! An AI racer's think: path to the next checkpoint, steer point past traffic, speed from the car's limits and the
//! skill. Specs: `docs/specs/ai-racers.md`, `docs/specs/ai-racers-route.md`, `docs/specs/ai-driver-speed-skill.md`.

use blackbox_driver::{CarLimits, SpeedGovernor, StuckDetector, potential_acceleration, potential_speed, ramp};
use blackbox_roads::{PathState, PathType, RandomSource, RoadNav, trail_curvature, update_occluded_position};
use blackbox_vehicle::performance::Performance;
use glam::Vec3;
use nfsmw_data::pursuit::AiVehicle;

use super::car::Ctx;
use super::cop::{THINK_PERIOD, THINK_STEPS};
use super::traffic::CarState;
use blackbox_roads::Body;

/// Racers think as often as cops do.
pub const RACER_THINK_STEPS: u32 = THINK_STEPS;
/// The path to the target is searched again this often, seconds.
const REPATH_PERIOD: f32 = 1.0;
/// The share of its top speed an emulated racer averages on a route with bends and traffic **[unconfirmed]**.
const EMULATED_SHARE: f32 = 0.7;
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
    /// The racer is far from the player and is not simulated: the race moves it along the route instead
    /// (`docs/specs/ai-racers-route.md` section 6).
    pub emulated: bool,
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
            emulated: false,
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

    /// The speed an emulated racer covers its route at, m/s: a share of the limits it would drive at, by skill, and the
    /// catch-up. **[unconfirmed]** (`docs/specs/ai-racers-route.md` section 6).
    pub fn emulated_speed(&self) -> f32 {
        let skill = self.skill();
        self.limits.top_speed * EMULATED_SHARE * (0.8 + 0.2 * skill)
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
            let state = nav.find_path_to(net, index, goal, None, PathType::Racer);
            self.path_state = Some(state);
            log::debug!(
                "racer {}: path from ({:.0}, {:.0}) to ({:.0}, {:.0}): {state:?}, {} segment(s)",
                self.id,
                car.body.position.x,
                car.body.position.z,
                goal.x,
                goal.z,
                nav.path.len()
            );
            if state == PathState::NoWay {
                nav.dead_end = false;
            }
        }
        let look = look_ahead(self.governor.speed_limit);
        let gap = nav.position.distance(car.body.position);
        let toward = to_goal.try_normalize().unwrap_or(Vec3::ZERO);
        if gap < look {
            nav.advance_with_lookahead(net, look - gap, toward, look, rng);
        } else if gap > 2.5 * look
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

#[cfg(test)]
mod tests {
    use super::*;

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
