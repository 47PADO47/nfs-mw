//! One computer-driven car: the physics, the driver and the road cursor it follows.

use std::cell::Cell;
use std::rc::{Rc, Weak};

use blackbox_driver::{AiControls, ControllerKind, Driver, GearRequest};
use blackbox_roads::{Body, RandomSource, RoadNav};
use glam::{Vec2, Vec3};

use super::TrafficModel;
use super::cop::{self, Target};
use super::racer;
use super::stop;
use super::traffic::{self, Cruise};
use super::trailer::TrailerCar;
use crate::scenes::world::drive::{CarPose, CarRig, CarSim, STEP, WorldGround};
use crate::scenes::world::road::Spawn;

mod accessors;
mod accident;
mod avoid;
mod damage_effects;
mod glass;
mod goal_drive;
mod hits;
mod lighting;
mod net;
mod original_camera;
mod patrol;
mod racing;
mod rain;
mod setup;
mod sight;
mod sound;
mod speech;
mod stand_in;
mod state_hash;
mod stepping;
pub(super) use stepping::AiStep;
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

/// The AI goal of a roadblock car, hashed once at compile time rather than per car per tick.
const ROADBLOCK_GOAL: u32 = blackbox_attrib::vlt_hash("AIGoalStaticRoadBlock");

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
    step_delta: f32,
    elapsed: f32,
    think_elapsed: f32,
    think_delta: f32,
    has_thought: bool,
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
    sight: sight::Sight,
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
    /// A racer off the loaded world moves by its stand-in (`car/stand_in.rs`).
    stand_in: stand_in::StandIn,
}

impl AiCar {
    #[cfg(test)]
    pub(in crate::scenes::world::ai) fn navigation(&self) -> &RoadNav {
        &self.nav
    }

    #[cfg(test)]
    pub(in crate::scenes::world::ai) fn drive_request(&self) -> (Vec3, f32) {
        (self.target, self.speed)
    }

    #[cfg(test)]
    pub(in crate::scenes::world::ai) fn telemetry_debug(&self) -> String {
        let action = match &self.role {
            Role::Cop(cop) => cop.last_diagnostic.as_str(),
            _ => "action=non-cop",
        };
        format!("{:?} {action}", self.sim.telemetry())
    }

    #[cfg(test)]
    pub(in crate::scenes::world::ai) fn nitrous_burning(&self) -> bool {
        self.sim.telemetry().nos_burning
    }

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
        sim.set_tunings(model.tunings);
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
            step_delta: STEP,
            elapsed: 0.0,
            think_elapsed: 0.0,
            think_delta: traffic::THINK_PERIOD,
            has_thought: false,
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
            sight: sight::Sight::default(),
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
            stand_in: stand_in::StandIn::default(),
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

    pub(in crate::scenes::world::ai) fn set_step_delta(&mut self, dt: f32) {
        self.step_delta = dt;
    }

    /// Advance the light bars by one tick, at the car's frame.
    pub fn step_lights(&mut self) {
        let active = self.emergency_lights();
        if let Some(lights) = &mut self.cop_lights {
            lights.step(self.step_delta, self.current.transform(), active);
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
    fn think<R: RandomSource>(&mut self, ctx: &mut Ctx<'_, R>, collision: &blackbox_collision::CollisionWorld) {
        self.think_delta = match self.has_thought {
            true => self.think_elapsed,
            false => {
                (match self.role {
                    Role::Traffic => traffic::THINK_STEPS,
                    Role::Cop(_) => cop::THINK_STEPS,
                    Role::Racer(_) => racer::RACER_THINK_STEPS,
                }) as f32
                    * self.step_delta
            }
        };
        self.think_elapsed = 0.0;
        self.has_thought = true;
        self.avoid.set_think_delta(self.think_delta);
        match &mut self.role {
            Role::Cop(cop) => cop.set_think_delta(self.think_delta),
            Role::Racer(racer) => racer.set_think_delta(self.think_delta),
            Role::Traffic => {}
        }
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
                let think = traffic::think_with_delta(
                    self.think_delta,
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
                // A stand-in is not driven, so it cannot be stuck.
                let stuck = !racer.held
                    && !self.stand_in.active
                    && racer.stuck.update(self.think_delta, pressing, reversing, false, state.body.position);
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
                let others = bodies.iter().enumerate().filter(move |&(i, _)| i != me).map(|(_, b)| b.0);
                let pressing = self.controls.gas >= 0.5;
                let stuck = cop.stuck.update(
                    self.think_delta,
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
                        self.nav.preserve_offset_at(ctx.net, at);
                        self.nav.enable_trail(ctx.net);
                    }
                    shift(&mut self.sim, Some(gear));
                }
                let aim = support_drive::aim(self.support, &player, ctx.aims.get(ctx.me).copied().flatten());
                let nos = self.sim.nitrous_spec();
                let think = cop.think(
                    ctx,
                    &mut self.nav,
                    &state,
                    &Target { body: player, aim },
                    others,
                    cop::ActionInputs {
                        collision,
                        grade: view.forward.y,
                        ram: self.goal.get().command().is_some(),
                        nitrous: cop::RoadNitrous {
                            boost: if nos.present() { nos.torque_boost } else { 0.0 },
                            bottle_seconds: nos.nos_capacity,
                            capacity: self.sim.telemetry().nos,
                            was_on: self.controls.nos,
                        },
                    },
                );
                let ram_allowed = cop.ram_allowed;
                self.target = think.target;
                self.think_goal(state.body.position, think.speed, ram_allowed);
                self.support_drive(&player, ram_allowed);
            }
        }
    }
}
