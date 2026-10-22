//! One computer-driven car: the physics, the driver and the road cursor it follows.

use std::cell::Cell;
use std::rc::{Rc, Weak};

use blackbox_collision::CollisionWorld;
use blackbox_driver::{AiControls, ControllerKind, DriveFlags, DriveRequest, Driver, GearRequest};
use blackbox_roads::{Body, RandomSource, RoadNav};
use glam::{Vec2, Vec3};
use nfsmw_data::car::physics::SurfaceTable;

use super::TrafficModel;
use super::cop::{self, Target};
use super::racer;
use super::stop;
use super::traffic::{self, Cruise, THINK_STEPS};
use super::trailer::{Brakes, TrailerCar};
use crate::scenes::world::drive::{CarPose, CarRig, CarSim, DriveInput, STEP, WorldGround};
use crate::scenes::world::props::PropWorld;
use crate::scenes::world::road::Spawn;

mod accessors;
mod accident;
mod avoid;
mod damage_effects;
mod glass;
mod goal_drive;
mod hits;
mod lighting;
mod original_camera;
mod patrol;
mod racing;
mod setup;
mod sound;
mod speech;
pub(super) mod support_drive;
mod towing;

pub use setup::{Ctx, Role, Start};

use accident::Accident;
use goal_drive::GoalDrive;
use hits::CopDamage;

/// A hit shorter than this change of speed (m/s) does not shock the car, and the level falls over this long.
const SHOCK_MIN_SPEED_CHANGE: f32 = 2.0;
const SHOCK_FORCE: f32 = 10.0;
const SHOCK_TIME: f32 = 3.0;

/// A cop stuck against something backs up for this long (s) asking for this speed (m/s).
const RECOVERY_SECONDS: f32 = 2.0;
const RECOVERY_SPEED: f32 = 15.0;
/// Below this speed (m/s) a car counts as standing still.
const IDLE_SPEED: f32 = 0.5;

/// Selects the gear a driver asked for.
fn shift(sim: &mut CarSim, gear: Option<GearRequest>) {
    match gear {
        Some(GearRequest::Reverse) => sim.shift_reverse(),
        Some(GearRequest::First) => sim.shift_first(),
        Some(GearRequest::Neutral) => sim.shift_neutral(),
        None => {}
    }
}

pub struct AiCar {
    pub name: String,
    rig: Rc<CarRig>,
    /// This physical vehicle's render identity, independent of its shared model rig. Presentation only.
    render_id: u32,
    /// The car's entity id in its world, given when it joins the world's list (`TrafficWorld::admit`); 0 before. The
    /// gameplay tie-breaks read this one, never `render_id` (`docs/specs/multiplayer/determinism.md`, D5).
    entity: u32,
    /// Persistent phase in [0,1), advancing with simulation even while lamps are off.
    light_phase: f32,
    sim: CarSim,
    /// The trailer of a semi tractor, hitched or loose.
    trailer: Option<TrailerCar>,
    driver: Driver,
    pub role: Role,
    nav: RoadNav,
    previous: CarPose,
    current: CarPose,
    /// The pose a NIS scene gives the car while it plays (`accessors.rs`); `None` draws the physics pose.
    nis_pose: Option<CarPose>,
    /// Physics steps since the car was placed.
    steps: u32,
    /// Delays the first think so the cars' thinks spread over ten steps.
    stagger: u32,
    /// What the last think decided.
    target: Vec3,
    speed: f32,
    controls: AiControls,
    radius: f32,
    mass: f32,
    /// Half width and half length of the body.
    half: (f32, f32),
    /// The last think found another car in the way.
    blocked: bool,
    /// Seconds the car has been (almost) standing still, not counting the time it waits for a light.
    idle: f32,
    /// The stop line the car last headed for (an index into the signal controller's approaches).
    stop_memory: Option<usize>,
    /// The car is held by a light that is not green.
    waiting: bool,
    /// Seconds with no wheel on the ground.
    airborne: f32,
    /// 0..1: hit hard enough to lose control for a moment.
    shock: f32,
    accident: Accident,
    tractor: bool,
    cruise: Cruise,
    patrol: bool,
    searching: bool,
    /// The cop was sent away (`StartFlee`): lights off, driving off along the traffic lanes until it is recycled.
    fleeing: bool,
    /// The cop has stood within the engagement radius of the player (`WasWithinEngagementRadius`).
    engaged: bool,
    arrested: bool,
    perp_hidden_from_me: bool,
    /// The car saw the player at its last sight test (`GetTimeSinceTargetSeen() <= 0`).
    sees_player: bool,
    /// Seconds the car has been out of the player's view.
    offscreen: f32,
    /// Whether this cop has stood where the patrol pass may not recycle it (`blackbox_pursuit::respawn`).
    respawn: blackbox_pursuit::respawn::Latch,
    breaker_stop: bool,
    /// A cop that has reached its slot holds the handbrake (the pull-over).
    arrived: bool,
    /// The goal a cop car drives by this step (`ai/goal_aims.rs`): its command and the limiter it carries. Set through a
    /// shared borrow, as the world reads the aims that way.
    goal: Cell<GoalDrive>,
    /// A roadblock car: it holds its place with the brakes on whatever the chase does (`AIGoalStaticRoadBlock`).
    roadblock: bool,
    /// The standing roadblock this car belongs to (`roadblock/state.rs`), 0 for none.
    roadblock_group: u32,
    /// The light bars of a cop car, when it has them.
    cop_lights: Option<super::cop_lights::CopLightRun>,
    /// Installed authored traffic sound class IDs.
    pub audio_profile: crate::audio::traffic::TrafficProfile,
    /// Seconds until the car may play a near miss or a horn again (`traffic_events::EventTuning::cooldown`).
    quiet: f32,
    /// A cop's damage (`hits`); `None` for traffic.
    damage: Option<CopDamage>,
    /// The request this support car belongs to and its goal (`car/support_drive.rs`); `None` for other cars.
    support: Option<support_drive::SupportCar>,
    /// The lane rules of a blocked traffic car (`car/avoid.rs`).
    avoid: avoid::Avoid,
    /// The player car's speed (m/s) at the last physics step: the crash gate of `hits.rs` reads it.
    player_speed: f32,
    pub(in crate::scenes::world::ai) damage_distance: f32,
}

impl AiCar {
    /// Puts a car of `model` on `spawn` (render space), rolling at `start.speed`, with a cursor in its lane.
    /// `None` when the ground is missing.
    pub fn place(
        model: &TrafficModel,
        role: Role,
        nav: RoadNav,
        spawn: Spawn,
        start: Start,
        ground: &WorldGround<'_>,
    ) -> Option<Self> {
        let (name, rig, physics) = (model.name.clone(), model.rig.clone(), &model.physics);
        let mut sim = CarSim::for_rig(physics.clone(), &rig);
        sim.configure_ai();
        if matches!(role, Role::Traffic) && !start.patrol {
            sim.configure_traffic();
        }
        sim.collision_role.cop = start.patrol || matches!(role, Role::Cop(_));
        if !sim.place_traffic_moving(ground, spawn, start.speed) {
            return None;
        }
        let trailer = match &model.trailer {
            Some(trailer) => Some(TrailerCar::place(trailer, &sim, spawn, start.speed, ground)?),
            None => None,
        };
        let damage = sim.collision_role.cop.then(|| CopDamage::for_model(&name, physics.events.as_deref()));
        let pose = sim.pose();
        let radius = physics.spec.dimension.length();
        let render_id = model.keys.next();
        let light_phase = (render_id.wrapping_mul(0x9e37_79b9) >> 8) as f32 / 16_777_216.0;
        Some(Self {
            name,
            rig,
            render_id,
            entity: 0,
            light_phase,
            sim,
            trailer,
            driver: Driver::new(
                match role {
                    Role::Traffic => ControllerKind::Simple,
                    Role::Cop(_) | Role::Racer(_) => ControllerKind::Pid,
                },
                false,
            ),
            role,
            target: nav.position,
            nav,
            previous: pose,
            current: pose,
            nis_pose: None,
            steps: 0,
            stagger: start.stagger,
            speed: start.speed,
            controls: AiControls::default(),
            radius,
            mass: physics.spec.mass,
            half: (physics.spec.dimension.x, physics.spec.dimension.z),
            blocked: false,
            idle: 0.0,
            stop_memory: None,
            waiting: false,
            airborne: 0.0,
            shock: 0.0,
            accident: Accident::None,
            tractor: model.tractor,
            cruise: start.cruise,
            patrol: start.patrol,
            searching: false,
            fleeing: false,
            engaged: false,
            arrested: false,
            perp_hidden_from_me: false,
            sees_player: false,
            offscreen: 0.0,
            respawn: Default::default(),
            breaker_stop: false,
            arrived: false,
            goal: Cell::new(GoalDrive::default()),
            roadblock: false,
            roadblock_group: 0,
            cop_lights: model.cop_lights.clone().map(super::cop_lights::CopLightRun::new),
            audio_profile: model.audio_profile,
            quiet: 0.0,
            damage,
            support: None,
            avoid: avoid::Avoid::default(),
            player_speed: 0.0,
            damage_distance: 0.0,
        })
    }

    /// The car's position in render space.
    pub fn position(&self) -> Vec3 {
        self.current.position
    }

    /// Where the car's nose points, in render space (x and y), for the minimap.
    pub fn heading(&self) -> [f32; 2] {
        let nose = self.current.rotation * Vec3::X;
        [nose.x, nose.y]
    }

    /// Advance the light bars by one tick, at the car's frame.
    pub fn step_lights(&mut self) {
        let active = self.emergency_lights();
        if let Some(lights) = &mut self.cop_lights {
            lights.step(STEP, self.current.transform(), active);
        }
    }

    /// The sprites of the light bars, for the effect layer.
    pub fn cop_particles(&self, camera: Vec3, forward: Vec3, out: &mut Vec<blackbox_render::TexturedEffect>) {
        if !self.emergency_lights() {
            return;
        }
        if let Some(lights) = &self.cop_lights {
            lights.textured(camera, forward, out);
        }
    }

    /// One physics step: think when it is this car's turn, then drive.
    pub fn step<R: RandomSource>(
        &mut self,
        ctx: &mut Ctx<'_, R>,
        collision: &CollisionWorld,
        props: &mut PropWorld,
        surfaces: &SurfaceTable,
    ) {
        if self.racer_emulated() {
            return;
        }
        self.sim.collision_goal = if self.roadblock && !self.is_wrecked() {
            Some(blackbox_attrib::vlt_hash("AIGoalStaticRoadBlock"))
        } else {
            self.goal_drive().collision_goal
        };
        let think_steps = match self.role {
            Role::Traffic => THINK_STEPS,
            Role::Cop(_) => cop::THINK_STEPS,
            Role::Racer(_) => super::racer::RACER_THINK_STEPS,
        };
        let human = self.human_pedals();
        if human.is_none() && !self.is_wrecked() && (self.steps + self.stagger).is_multiple_of(think_steps) {
            self.think(ctx);
        }
        let mut view = self.sim.driver_view();
        view.in_shock = self.shock > 0.0;
        self.shock = (self.shock - STEP / SHOCK_TIME).max(0.0);
        let reversing = self.driver.reverse_override_left() > 0.0;
        let flags = match (&self.role, reversing, self.searching) {
            (Role::Cop(_) | Role::Racer(_), false, false) => DriveFlags::FULL,
            _ => DriveFlags::SIMPLE,
        };
        let wanted = if reversing { RECOVERY_SPEED } else { self.speed };
        let target = self.target + self.avoid.steer_offset(&self.nav, view.position);
        let request = DriveRequest { target, speed: wanted, flags };
        let clock = self.steps as f32 * STEP;
        let held = self.controls.steer;
        if human.is_none() {
            self.controls = self.driver.step(&view, &request, STEP, clock);
        }
        if let Some(controls) = self.accident.controls(self.tractor, held) {
            self.controls = controls;
        }
        self.hold_goal_brake();
        if self.breaker_stop {
            self.controls = AiControls { brake: 1.0, ..AiControls::default() };
        }
        if self.is_wrecked() {
            self.arrived = false;
            self.controls = AiControls { brake: self.wreck_brake(), ..AiControls::default() };
        }
        if (self.arrested || self.racer_held()) && !self.is_wrecked() {
            self.controls = AiControls { brake: 1.0, handbrake: 1.0, ..AiControls::default() };
        }
        shift(&mut self.sim, self.controls.gear);
        let input = match human {
            Some(pedals) => self.human_input(pedals),
            None => DriveInput {
                throttle: self.controls.gas,
                brake: self.controls.brake,
                steer: self.controls.steer,
                handbrake: self.controls.handbrake > 0.5 || self.arrived,
                nos: self.controls.nos,
                ..DriveInput::default()
            },
        };
        let destroyed = self.is_wrecked();
        self.sim.set_destroyed_non_player(destroyed);
        let impact = self.sim.step(&input, &WorldGround { collision, surfaces }, Some((collision, props)));
        for hit in impact.damage {
            self.wreck_hit(hit, self.damage_distance);
        }
        self.avoid.advance(self.speed().abs() * STEP);
        if let Some(trailer) = self.trailer.as_mut() {
            let brakes = Brakes { brake: self.controls.brake, handbrake: self.controls.handbrake > 0.5 };
            trailer.step(&mut self.sim, brakes, collision, props, surfaces);
        }
        self.previous = self.current;
        self.current = self.sim.pose();
        self.wreck_step(STEP);
        self.steps += 1;
        self.airborne = match self.sim.telemetry().wheels_on_ground == 0 {
            true => self.airborne + STEP,
            false => 0.0,
        };
        self.idle = match self.sim.telemetry().speed_mps.abs() < IDLE_SPEED && !self.waiting {
            true => self.idle + STEP,
            false => 0.0,
        };
    }

    /// The car as the trails of the other cars see it.
    pub fn body(&self) -> Body {
        let view = self.sim.driver_view();
        let forward = Vec2::new(view.forward.x, view.forward.z).try_normalize().unwrap_or(Vec2::Y);
        Body {
            position: view.position,
            velocity: self.sim.velocity(),
            forward,
            half_width: self.half.0,
            half_length: self.half.1,
        }
    }

    /// The think of the car's role: traffic follows its lane, a cop chases the player.
    fn think<R: RandomSource>(&mut self, ctx: &mut Ctx<'_, R>) {
        let view = self.sim.driver_view();
        let mut state = traffic::CarState {
            body: self.body(),
            forward_speed: view.forward_speed,
            radius: self.radius,
            mass: self.mass,
            tractor: self.tractor,
            stop: None,
        };
        if self.searching {
            self.think_search(ctx, &state);
            return;
        }
        match &mut self.role {
            Role::Traffic => {
                self.accident.think();
                // Patrol cops obey the lights too; only a cop in a chase does not.
                state.stop =
                    ctx.lights.and_then(|l| stop::find(&l, ctx.net, &self.nav, &state.body, &mut self.stop_memory));
                self.waiting = state.stop.is_some_and(|s| s.is_holding());
                self.avoid.before_think(
                    ctx.net,
                    ctx.index,
                    &mut self.nav,
                    &state.body,
                    traffic::others(ctx.bodies, ctx.me, ctx.player),
                    self.blocked,
                );
                state.stop = self.avoid.yield_at(
                    ctx.net,
                    ctx.lights,
                    state.stop,
                    self.stop_memory,
                    traffic::others(ctx.bodies, ctx.me, ctx.player),
                );
                let think = traffic::think(
                    ctx.net,
                    &mut self.nav,
                    &state,
                    traffic::others(ctx.bodies, ctx.me, ctx.player),
                    self.blocked,
                    ctx.rng,
                    &self.cruise,
                );
                self.target = think.target;
                self.speed = think.speed;
                self.blocked = think.blocked;
            }
            Role::Racer(racer) => {
                let pressing = self.controls.gas >= 0.5;
                let reversing = self.driver.reverse_override_left() > 0.0;
                let stuck = !racer.held
                    && racer.stuck.update(cop::THINK_PERIOD, pressing, reversing, false, state.body.position);
                if stuck {
                    log::info!("race: {} is stuck, reversing", self.name);
                    let gear = self.driver.start_reverse_override(RECOVERY_SECONDS, view.gear_is_reverse);
                    let at = state.body.position;
                    let forward = Vec3::new(state.body.forward.x, 0.0, state.body.forward.y);
                    if self.nav.init_at_point(ctx.net, ctx.index, at, forward, false) {
                        self.nav.enable_trail(ctx.net);
                    }
                    shift(&mut self.sim, Some(gear));
                }
                let think = racer.think(ctx, &mut self.nav, &state, traffic::others(ctx.bodies, ctx.me, ctx.player));
                self.target = think.target;
                self.speed = think.speed;
            }
            Role::Cop(cop) => {
                let Some(player) = ctx.player else {
                    self.speed = 0.0;
                    return;
                };
                let (bodies, me) = (ctx.bodies, ctx.me);
                let others =
                    bodies.iter().enumerate().filter(move |&(i, (_, is_cop))| i != me && !is_cop).map(|(_, b)| b.0);
                let pressing = self.controls.gas >= 0.5;
                let stuck = cop.stuck.update(
                    cop::THINK_PERIOD,
                    pressing,
                    self.driver.reverse_override_left() > 0.0,
                    false,
                    state.body.position,
                );
                if stuck {
                    log::info!("cop: {} is stuck, reversing", self.name);
                    let gear = self.driver.start_reverse_override(RECOVERY_SECONDS, view.gear_is_reverse);
                    let at = state.body.position;
                    let forward = Vec3::new(state.body.forward.x, 0.0, state.body.forward.y);
                    if self.nav.init_at_point(ctx.net, ctx.index, at, forward, false) {
                        self.nav.enable_trail(ctx.net);
                    }
                    shift(&mut self.sim, Some(gear));
                }
                let aim = support_drive::aim(self.support, &player, ctx.aims.get(ctx.me).copied().flatten());
                let think = cop.think(ctx, &mut self.nav, &state, &Target { body: player, aim }, others);
                self.target = think.target;
                self.think_goal(aim, state.body.position, think.speed);
                self.support_drive(&player);
            }
        }
    }
}
