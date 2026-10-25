//! A cop's think: path to the player, steer point past traffic, speed from the car's limits.
//! Spec: `docs/specs/ai-driver-speed-skill.md` (§1, §3 to §5), `docs/specs/ai-driver-control.md` (§7.1).

use blackbox_collision::CollisionWorld;
use blackbox_driver::{CarLimits, SpeedGovernor, StuckDetector, potential_acceleration, potential_speed};
use blackbox_driver::{PursuitContext, ramp};
use blackbox_roads::{Body, PathState, PathType, RandomSource, RoadNav};
use blackbox_roads::{trail_curvature, update_occluded_position};
use blackbox_vehicle::performance::Performance;
use glam::{Vec2, Vec3};
use nfsmw_data::pursuit::AiVehicle;

use super::car::Ctx;
use super::traffic::CarState;

mod close;
mod history;
mod motion;
#[cfg(test)]
mod reinforcement_tests;
mod road_nos;
#[cfg(test)]
mod road_nos_tests;
mod roads;
pub(super) mod separation;
#[cfg(test)]
mod shared_target_tests;
mod state_hash;

pub(in crate::scenes::world::ai) use road_nos::Input as RoadNitrous;
pub(super) use roads::TargetRoad;

/// Cops think every eighth physics step.
pub const THINK_STEPS: u32 = 8;
pub const THINK_PERIOD: f32 = THINK_STEPS as f32 / 60.0;
/// The path to the player is searched again this often, seconds.
const REPATH_PERIOD: f32 = 1.0;
/// A cop has the skill of its constructor value.
const SKILL: f32 = 1.0;
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
    road_nos: road_nos::State,
    pub stuck: StuckDetector,
    repath: f32,
    last_find: Vec3,
    last_speed: f32,
    path_state: Option<PathState>,
    close: close::Close,
    environment: Environment,
    pub(super) ram_allowed: bool,
    /// The other cars of the think in progress; kept so a think does not allocate.
    traffic_scratch: Vec<Body>,
    #[cfg(test)]
    pub(super) last_diagnostic: String,
}

/// Live pursuit values shared by the driving actions.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Environment {
    pub reaction: Option<f32>,
    pub level: u32,
    pub nitrous: bool,
    pub nitrous_input: bool,
    pub nitrous_boost: f32,
    pub jerk: bool,
    pub racing: bool,
    pub head_on_support: bool,
    /// The primary uses the shared tracker even when it cannot publish a road.
    pub shared_target_road: bool,
    pub target_road: Option<TargetRoad>,
}

/// What the player looks like to the cop.
#[derive(Debug, Clone, Copy)]
pub struct Target {
    pub body: Body,
    /// Where this cop drives instead of the player, when a tactic (a block or a pit) sets one.
    pub aim: Option<Vec3>,
}

/// What a think decided.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CopThink {
    pub target: Vec3,
    pub speed: f32,
}

/// Physical context needed to select and run a pursuit driving action.
pub(super) struct ActionInputs<'a> {
    pub collision: &'a CollisionWorld,
    pub grade: f32,
    pub ram: bool,
    pub nitrous: RoadNitrous,
}

impl CopState {
    pub fn new(car: CopCar, player: Performance, speed: f32) -> Self {
        Self {
            limits: matched_limits(&car, &player),
            car,
            player,
            governor: SpeedGovernor::begin(speed),
            road_nos: road_nos::State::default(),
            stuck: StuckDetector::default(),
            repath: 0.0,
            last_find: Vec3::ZERO,
            last_speed: speed,
            path_state: None,
            close: close::Close::default(),
            environment: Environment::default(),
            ram_allowed: false,
            traffic_scratch: Vec::new(),
            #[cfg(test)]
            last_diagnostic: String::new(),
        }
    }

    /// A return from lane search starts a fresh pursuit path and speed governor at the existing car's speed.
    pub fn resume_chase(&mut self, speed: f32) {
        let environment = self.environment;
        *self = Self::new(self.car, self.player, speed);
        self.environment = environment;
    }

    pub(super) fn environment(&mut self, environment: Environment) {
        self.environment = environment;
    }

    pub(super) fn set_head_on_support(&mut self, enabled: bool) {
        if self.environment.head_on_support == enabled {
            return;
        }
        self.environment.head_on_support = enabled;
        self.repath = 0.0;
        self.road_nos.leave();
        self.close.leave();
    }

    pub(super) fn nitrous(&self) -> bool {
        match self.close.active() {
            true => self.close.nos,
            false => self.road_nos.requested(),
        }
    }

    pub(super) fn close_active(&self) -> bool {
        self.close.active()
    }

    #[cfg(test)]
    pub(super) fn target_road_observation(&self) -> (bool, Option<TargetRoad>) {
        (self.environment.shared_target_road, self.environment.target_road)
    }

    /// The think of the race action in pursuit mode.
    pub(super) fn think<R: RandomSource>(
        &mut self,
        ctx: &mut Ctx<'_, R>,
        nav: &mut RoadNav,
        car: &CarState,
        target: &Target,
        traffic: impl IntoIterator<Item = Body>,
        inputs: ActionInputs<'_>,
    ) -> CopThink {
        let mut others = std::mem::take(&mut self.traffic_scratch);
        others.clear();
        others.extend(traffic);
        let think = self.think_among(ctx, nav, car, target, &others, inputs);
        self.traffic_scratch = others;
        think
    }

    fn think_among<R: RandomSource>(
        &mut self,
        ctx: &mut Ctx<'_, R>,
        nav: &mut RoadNav,
        car: &CarState,
        target: &Target,
        traffic: &[Body],
        inputs: ActionInputs<'_>,
    ) -> CopThink {
        let ActionInputs { collision, grade, ram, nitrous } = inputs;
        #[cfg(test)]
        let mut diagnostic_traffic = traffic.to_vec();
        let road = match self.environment.shared_target_road {
            true => self.environment.target_road,
            false => roads::target(ctx, target, self.environment.level, self.environment.jerk),
        };
        let clear_target = roads::clear(collision, car.body.position, target.body.position);
        let clear_nav = roads::clear(collision, car.body.position + car.body.velocity * 0.65, nav.position);
        self.ram_allowed = clear_target && clear_nav;
        if ram && self.ram_allowed {
            #[cfg(test)]
            {
                self.last_diagnostic = "action=ram".into();
            }
            self.road_nos.leave();
            self.close.leave();
            self.repath = 0.0;
            return CopThink { target: target.aim.unwrap_or(target.body.position), speed: car.forward_speed.abs() };
        }
        let road_gap = road.as_ref().map_or(0.0, |road| road.gap);
        if close::eligible(&car.body, &target.body, road_gap, clear_target, clear_nav) {
            self.road_nos.leave();
            // Drivability to the cop's road is a separate test from target visibility.
            if !self.close.active()
                && let Some(own) = roads::at_car(ctx, &car.body, self.environment.level)
            {
                *nav = own;
                nav.enable_trail(ctx.net);
            }
            let avoidance = roads::wall(collision, &car.body);
            let reaction = self.environment.reaction.unwrap_or(
                [1.75, 1.5, 0.75, 0.5, 0.1, 0.1, 0.5, 0.0, 0.0, 0.0]
                    [(self.environment.level as usize).clamp(1, 10) - 1],
            );
            let direction = road.as_ref().map_or(target.body.forward, |r| r.direction);
            let aim = target.aim.unwrap_or(target.body.position);
            let aim = road.as_ref().map_or(aim, |road| road.centre + aim - target.body.position);
            let think = self.close.think(
                close::Input {
                    car: &self.car,
                    me: &car.body,
                    target: &target.body,
                    aim,
                    road_direction: road.as_ref().map(|r| r.direction),
                    separation: separation::steer(&car.body, traffic.iter().copied(), 2.7, 5.3),
                    avoidance,
                    performance: &self.player,
                    forward_speed: car.forward_speed,
                    grade,
                    reaction,
                    target_nos: self.environment.nitrous,
                    target_nos_input: self.environment.nitrous_input,
                    jerk: self.environment.jerk,
                },
                |point| roads::snap(ctx, nav, &car.body, point, direction, traffic, self.environment.level),
            );
            #[cfg(test)]
            {
                self.last_diagnostic = format!(
                    "{} road_gap={road_gap:.3} clear_target={clear_target} clear_nav={clear_nav}",
                    self.close.diagnostic()
                );
            }
            return think;
        }
        if !self.road_nos.active() {
            if self.close.active()
                && let Some(own) = roads::at_car(ctx, &car.body, self.environment.level)
            {
                *nav = own;
                nav.enable_trail(ctx.net);
            }
            self.repath = 0.0;
            self.governor = SpeedGovernor::begin(car.forward_speed);
            self.last_speed = car.forward_speed;
            self.road_nos.begin();
        }
        self.close.leave();
        let (net, index, rng) = (ctx.net, ctx.index, &mut *ctx.rng);
        // The road chase intercepts the player's future road. Formation slots
        // belong to the close/ram actions and must not turn a distant cop back.
        let goal = match self.environment.head_on_support {
            true => target.body.position,
            false => road.as_ref().map_or(target.body.position, |r| r.seek),
        };
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
        // Keep the cursor on the forward tangent's look-ahead circle. Its
        // distance alone cannot tell whether the car has already passed it.
        let look = look_ahead(self.governor.speed_limit);
        let gap = nav.position.distance(car.body.position);
        let toward = to_target.try_normalize().unwrap_or(Vec3::ZERO);
        if gap > 2.5 * look {
            // The cursor is far behind or off to the side: start again from the car.
            if nav.init_at_point(
                net,
                index,
                car.body.position,
                Vec3::new(car.body.forward.x, 0.0, car.body.forward.y),
                false,
            ) {
                nav.preserve_offset_at(net, car.body.position);
                nav.enable_trail(net);
                self.repath = 0.0;
            }
        }
        let advance = motion::nav_advance(nav.position - car.body.position, nav.forward, look);
        if advance > 0.0 {
            nav.advance_with_lookahead(net, advance, toward, look, rng);
        }
        // Everything but the player is something to steer round.
        let occlusion = nav.trail().and_then(|trail| {
            update_occluded_position(trail, &car.body, nav.position, traffic.iter().copied(), false, nav.half_width)
        });
        let curvature = match (&occlusion, nav.trail()) {
            (Some(o), Some(trail)) => {
                trail_curvature(trail.cookies(), o.current_index, car.body.position, o, nav.position, car.forward_speed)
            }
            _ => nav.curvature,
        };

        let speed = car.forward_speed;
        let (offset, seek) = motion::speed_vectors(&car.body, &target.body, self.last_find);
        let context = PursuitContext {
            distant: self.car.ai.max_speed_kmh / 3.6
                * match self.environment.jerk {
                    true => 1.1,
                    false => 1.0,
                },
            offset,
            seek_dir: seek,
            target_speed: target.body.velocity.length(),
            forward: car.body.forward,
            steer_dir: Vec2::new(nav.position.x - car.body.position.x, nav.position.z - car.body.position.z)
                .normalize_or_zero(),
            target_steer_dir: road.as_ref().map_or(target.body.forward, |r| r.direction),
            race_running: self.environment.racing,
            jerk: self.environment.jerk,
        };
        let potential = potential_speed(&self.limits, curvature, SKILL, Some(&context));
        #[cfg(test)]
        let table_speed = speed.abs().max(self.governor.speed_limit);
        let table = self
            .car
            .performance
            .acceleration_at(speed.abs().max(self.governor.speed_limit))
            .min(self.player.acceleration_at(speed.abs().max(self.governor.speed_limit)));
        let scale = match (self.environment.racing, self.environment.jerk) {
            (true, _) => 2.0,
            (_, true) => 1.5,
            _ => 1.0,
        };
        let boost = nitrous.acceleration_boost(self.environment.nitrous_boost);
        let accel = potential_acceleration(table * scale, &self.limits, SKILL, boost, grade);
        let actual_acceleration = (speed - self.last_speed) / THINK_PERIOD;
        let wanted = self.governor.update(speed, potential, accel, SKILL, THINK_PERIOD);
        self.last_speed = speed;
        log::trace!(
            "cop think: speed {speed:.1} curvature {curvature:.4} potential {potential:.1} accel {accel:.1} (table {table:.1}) wanted {wanted:.1} cap {:.1} path {:?}",
            context.max_cop_speed(),
            self.path_state
        );
        let destination = occlusion.as_ref().map_or(nav.position, |o| o.position);
        let turning = motion::needs_turnaround(&car.body, destination);
        self.road_nos.update(
            nitrous,
            self.environment.nitrous_boost,
            road_nos::Decision {
                speed,
                requested_speed: wanted,
                potential_speed: potential,
                potential_acceleration: accel,
                actual_acceleration,
                occluded: occlusion.as_ref().is_some_and(|o| o.occluded()),
                stopping: turning,
                skill: SKILL,
            },
            THINK_PERIOD,
        );
        #[cfg(test)]
        {
            diagnostic_traffic.sort_by(|a, b| {
                a.position
                    .distance_squared(car.body.position)
                    .total_cmp(&b.position.distance_squared(car.body.position))
            });
            diagnostic_traffic.truncate(4);
            let forward = close::lift(car.body.forward, 0.0);
            let own_acceleration = self.car.performance.acceleration_at(table_speed);
            let player_acceleration = self.player.acceleration_at(table_speed);
            self.last_diagnostic = format!(
                "action=road position={:?} raw_nav={:?} raw_nav_ahead={:.2} destination={destination:?} destination_ahead={:.2} last_find={:?} target_position={:?} path={:?} curvature={curvature:.6} potential={potential:.3} maxcap={:.3} table_speed={table_speed:.3} own_accel={own_acceleration:.3} player_accel={player_acceleration:.3} table_min={table:.3} accel={accel:.3} actual_accel={actual_acceleration:.3} wanted={wanted:.3} NOSboost={boost:?} own_top={:.3} player_top={:.3} matched_top={:.3} player_performance={:?} occlusion={occlusion:?} nearest={diagnostic_traffic:?}",
                car.body.position,
                nav.position,
                (nav.position - car.body.position).dot(forward),
                (destination - car.body.position).dot(forward),
                self.last_find,
                target.body.position,
                self.path_state,
                context.max_cop_speed(),
                self.car.performance.top_speed,
                self.player.top_speed,
                self.limits.top_speed,
                self.player,
            );
        }
        if turning {
            let own_road = roads::at_car_on(net, index, &car.body, self.environment.level);
            let mut future = own_road.unwrap_or_else(|| nav.clone());
            let heading = close::lift(car.body.forward, 0.0);
            future.advance(net, car.body.velocity.length().max(2.0), heading, rng);
            return motion::brake_on_future_road(future.position);
        }
        CopThink { target: destination, speed: wanted }
    }
}
