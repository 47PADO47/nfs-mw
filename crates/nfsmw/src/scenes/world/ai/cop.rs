//! A cop's think: path to the player, steer point past traffic, speed from the car's limits.
//! Spec: `docs/specs/ai-driver-speed-skill.md` (§1, §3 to §5), `docs/specs/ai-driver-control.md` (§7.1).

use blackbox_driver::{CarLimits, SpeedGovernor, StuckDetector, potential_acceleration, potential_speed};
use blackbox_driver::{PursuitContext, ramp};
use blackbox_roads::{Body, PathState, PathType, RandomSource, RoadNav};
use blackbox_roads::{trail_curvature, update_occluded_position};
use blackbox_vehicle::performance::Performance;
use glam::{Vec2, Vec3};
use nfsmw_data::pursuit::AiVehicle;

use super::car::Ctx;
use super::traffic::CarState;

/// Cops think every eighth physics step.
pub const THINK_STEPS: u32 = 8;
pub const THINK_PERIOD: f32 = THINK_STEPS as f32 / 60.0;
/// The path to the player is searched again this often, seconds.
const REPATH_PERIOD: f32 = 1.0;
/// A cop has the skill of its constructor value.
const SKILL: f32 = 0.5;
/// The look-ahead follows the requested speed: 30 m at rest to 100 m at 100 m/s.
fn look_ahead(speed_limit: f32) -> f32 {
    30.0 + 70.0 * ramp(speed_limit, 0.0, 100.0)
}

/// What a cop knows of the car it drives.
#[derive(Debug, Clone, Copy)]
pub struct CopCar {
    pub performance: Performance,
    pub ai: AiVehicle,
}

/// The limits of the car after matching it to the player's car (`bias = 0`: the weaker of the two).
pub fn matched_limits(cop: &CopCar, player: &Performance) -> CarLimits {
    CarLimits {
        top_speed: cop.performance.top_speed.min(player.top_speed),
        start_grip: cop.performance.start_grip.min(player.start_grip),
        end_grip: cop.performance.end_grip.min(player.end_grip),
        top_speed_multiplier: cop.ai.top_speed_multiplier,
        acceleration_multiplier: cop.ai.acceleration_multiplier,
    }
}

/// One cop's pursuit state.
#[derive(Debug, Clone)]
pub struct CopState {
    pub car: CopCar,
    limits: CarLimits,
    player: Performance,
    governor: SpeedGovernor,
    pub stuck: StuckDetector,
    repath: f32,
    last_find: Vec3,
    last_speed: f32,
    path_state: Option<PathState>,
}

/// What the player looks like to the cop.
#[derive(Debug, Clone, Copy)]
pub struct Target {
    pub body: Body,
}

/// What a think decided.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CopThink {
    pub target: Vec3,
    pub speed: f32,
}

impl CopState {
    pub fn new(car: CopCar, player: Performance, speed: f32) -> Self {
        Self {
            limits: matched_limits(&car, &player),
            car,
            player,
            governor: SpeedGovernor::begin(speed),
            stuck: StuckDetector::default(),
            repath: 0.0,
            last_find: Vec3::ZERO,
            last_speed: speed,
            path_state: None,
        }
    }

    /// The think of the race action in pursuit mode.
    pub fn think<R: RandomSource>(
        &mut self,
        ctx: &mut Ctx<'_, R>,
        nav: &mut RoadNav,
        car: &CarState,
        target: &Target,
        traffic: &[Body],
    ) -> CopThink {
        let (net, index, rng) = (ctx.net, ctx.index, &mut *ctx.rng);
        let goal = target.body.position;
        self.repath -= THINK_PERIOD;
        let to_target = Vec3::new(goal.x - car.body.position.x, 0.0, goal.z - car.body.position.z);
        if self.repath <= 0.0 || nav.dead_end {
            self.repath = REPATH_PERIOD;
            self.last_find = goal;
            let state = nav.find_path_to(net, index, goal, None, PathType::Cop);
            self.path_state = Some(state);
            if state == PathState::NoWay {
                nav.dead_end = false;
            }
        }
        // Keep the cursor a look-ahead's distance ahead of the car, steering towards the target.
        let look = look_ahead(self.governor.speed_limit);
        let gap = nav.position.distance(car.body.position);
        let toward = to_target.try_normalize().unwrap_or(Vec3::ZERO);
        if gap < look {
            nav.advance_with_lookahead(net, look - gap, toward, look, rng);
        } else if gap > 2.5 * look {
            // The cursor is far behind or off to the side: start again from the car.
            if nav.init_at_point(
                net,
                index,
                car.body.position,
                Vec3::new(car.body.forward.x, 0.0, car.body.forward.y),
                false,
            ) {
                nav.enable_trail(net);
                self.repath = 0.0;
            }
        }
        // Everything but the player is something to steer round.
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
        let offset = Vec2::new(-to_target.x, -to_target.z);
        let seek = Vec2::new(self.last_find.x - goal.x, self.last_find.z - goal.z)
            .try_normalize()
            .unwrap_or(target.body.forward);
        let context = PursuitContext {
            distant: self.car.ai.max_speed_kmh / 3.6,
            offset,
            seek_dir: seek,
            target_speed: target.body.velocity.length(),
            forward: car.body.forward,
            steer_dir: car.body.forward,
            target_steer_dir: target.body.forward,
            race_running: false,
            jerk: false,
        };
        let potential = potential_speed(&self.limits, curvature, SKILL, Some(&context));
        let table = self
            .car
            .performance
            .acceleration_at(speed.abs().max(self.governor.speed_limit))
            .min(self.player.acceleration_at(speed.abs().max(self.governor.speed_limit)));
        let accel = potential_acceleration(table, &self.limits, SKILL, None, 0.0);
        let wanted = self.governor.update(speed, potential, accel, SKILL, THINK_PERIOD);
        self.last_speed = speed;
        log::trace!(
            "cop think: speed {speed:.1} curvature {curvature:.4} potential {potential:.1} accel {accel:.1} (table {table:.1}) wanted {wanted:.1} cap {:.1} path {:?}",
            context.max_cop_speed(),
            self.path_state
        );
        CopThink { target: occlusion.map_or(nav.position, |o| o.position), speed: wanted }
    }
}
