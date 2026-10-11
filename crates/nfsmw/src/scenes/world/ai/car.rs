//! One computer-driven car: the physics, the driver and the road cursor it follows.

use std::rc::Rc;

use blackbox_collision::CollisionWorld;
use blackbox_driver::{AiControls, ControllerKind, DriveFlags, DriveRequest, Driver, GearRequest};
use blackbox_roads::{Body, RandomSource, RoadNav, RoadNetwork, SegmentIndex};
use glam::{Vec2, Vec3};
use nfsmw_data::car::physics::SurfaceTable;

use super::TrafficModel;
use super::cop::{self, CopState, Target};
use super::traffic::{self, Cruise, THINK_STEPS};
use crate::scenes::world::drive::{CarPose, CarRig, CarSim, DriveInput, STEP, WorldGround};
use crate::scenes::world::props::PropWorld;
use crate::scenes::world::road::Spawn;
use crate::scenes::world::space;

/// What a car is for.
pub enum Role {
    Traffic,
    /// A cop chasing the player.
    Cop(Box<CopState>),
}

/// The world a car thinks in.
pub struct Ctx<'a, R> {
    pub net: &'a RoadNetwork,
    pub index: &'a SegmentIndex,
    pub rng: &'a mut R,
    /// Every computer-driven car, with whether it is a cop, and which of them is this car.
    pub bodies: &'a [(Body, bool)],
    pub me: usize,
    pub player: Option<Body>,
}

/// How a new car begins: when its first think comes and how fast it rolls.
#[derive(Debug, Clone, Copy)]
pub struct Start {
    /// Physics steps added to the car's clock so the thinks of different cars fall on different steps.
    pub stagger: u32,
    pub speed: f32,
    /// The speeds it cruises at.
    pub cruise: Cruise,
    /// A patrol cop: a cop car driving like traffic.
    pub patrol: bool,
}

/// How a crash with the player went for a traffic car.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Accident {
    None,
    /// No pedals and full lock, for this many more thinks.
    InProgress(u32),
    /// Braking to a halt with the wheels still turned.
    Over,
}

/// Thinks an accident lasts (the original counts thinks where it probably meant seconds).
const ACCIDENT_THINKS: u32 = 3;
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
    sim: CarSim,
    driver: Driver,
    pub role: Role,
    nav: RoadNav,
    previous: CarPose,
    current: CarPose,
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
    /// Seconds the car has been (almost) standing still.
    idle: f32,
    /// Seconds with no wheel on the ground.
    airborne: f32,
    /// 0..1: hit hard enough to lose control for a moment.
    shock: f32,
    accident: Accident,
    cruise: Cruise,
    patrol: bool,
    /// Seconds the car has been out of the player's view.
    offscreen: f32,
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
        if !sim.place_moving(ground, spawn, start.speed) {
            return None;
        }
        let pose = sim.pose();
        let radius = physics.spec.dimension.length();
        Some(Self {
            name,
            rig,
            sim,
            driver: Driver::new(
                match role {
                    Role::Traffic => ControllerKind::Simple,
                    Role::Cop(_) => ControllerKind::Pid,
                },
                false,
            ),
            role,
            target: nav.position,
            nav,
            previous: pose,
            current: pose,
            steps: 0,
            stagger: start.stagger,
            speed: start.speed,
            controls: AiControls::default(),
            radius,
            mass: physics.spec.mass,
            half: (physics.spec.dimension.x, physics.spec.dimension.z),
            blocked: false,
            idle: 0.0,
            airborne: 0.0,
            shock: 0.0,
            accident: Accident::None,
            cruise: start.cruise,
            patrol: start.patrol,
            offscreen: 0.0,
        })
    }

    /// The car's position in render space.
    pub fn position(&self) -> Vec3 {
        self.current.position
    }

    pub fn pose(&self, alpha: f32) -> CarPose {
        self.previous.lerp(&self.current, alpha)
    }

    pub fn rig(&self) -> &CarRig {
        &self.rig
    }

    pub fn is_finite(&self) -> bool {
        self.sim.is_finite()
    }

    /// Seconds the car has had no wheel on the ground (it fell off the map, or flew).
    pub fn airborne_time(&self) -> f32 {
        self.airborne
    }

    /// Seconds the car has been standing still.
    pub fn idle_time(&self) -> f32 {
        self.idle
    }

    /// Speed along the car, m/s.
    pub fn speed(&self) -> f32 {
        self.sim.telemetry().speed_mps
    }

    /// A line about what the car is doing, for the `traffic status` command.
    pub fn debug(&self) -> String {
        format!(
            "{} segment {} lane {} t {:.2}{}{}, wants {:.1} m/s",
            if self.is_cop() {
                "cop"
            } else if self.patrol {
                "patrol"
            } else {
                "traffic"
            },
            self.nav.segment,
            self.nav.lane,
            self.nav.t,
            if self.blocked { ", blocked" } else { "" },
            if self.nav.dead_end { ", dead end" } else { "" },
            self.speed
        )
    }

    pub fn controls(&self) -> AiControls {
        self.controls
    }

    /// A cop chasing the player.
    pub fn is_cop(&self) -> bool {
        matches!(self.role, Role::Cop(_))
    }

    /// A cop car cruising like traffic.
    pub fn is_patrol(&self) -> bool {
        self.patrol
    }

    /// Adds `dt` to the time the car has been out of view, or clears it when it is in view.
    pub fn note_view(&mut self, in_view: bool, dt: f32) {
        self.offscreen = match in_view {
            true => 0.0,
            false => self.offscreen + dt,
        };
    }

    pub fn offscreen_time(&self) -> f32 {
        self.offscreen
    }

    /// One physics step: think when it is this car's turn, then drive.
    pub fn step<R: RandomSource>(
        &mut self,
        ctx: &mut Ctx<'_, R>,
        collision: &CollisionWorld,
        props: &mut PropWorld,
        surfaces: &SurfaceTable,
    ) {
        let think_steps = match self.role {
            Role::Traffic => THINK_STEPS,
            Role::Cop(_) => cop::THINK_STEPS,
        };
        if (self.steps + self.stagger).is_multiple_of(think_steps) {
            self.think(ctx);
        }
        let mut view = self.sim.driver_view();
        view.in_shock = self.shock > 0.0;
        self.shock = (self.shock - STEP / SHOCK_TIME).max(0.0);
        let reversing = self.driver.reverse_override_left() > 0.0;
        let flags = match (&self.role, reversing) {
            (Role::Cop(_), false) => DriveFlags::FULL,
            _ => DriveFlags::SIMPLE,
        };
        let wanted = if reversing { RECOVERY_SPEED } else { self.speed };
        let request = DriveRequest { target: self.target, speed: wanted, flags };
        let clock = self.steps as f32 * STEP;
        self.controls = self.driver.step(&view, &request, STEP, clock);
        match self.accident {
            Accident::None => {}
            Accident::InProgress(_) => {
                self.controls = AiControls { steer: 1.0, ..AiControls::default() };
            }
            Accident::Over => {
                self.controls = AiControls { brake: 1.0, steer: 1.0, ..AiControls::default() };
            }
        }
        shift(&mut self.sim, self.controls.gear);
        let input = DriveInput {
            throttle: self.controls.gas,
            brake: self.controls.brake,
            steer: self.controls.steer,
            handbrake: self.controls.handbrake > 0.5,
            nos: self.controls.nos,
            ..DriveInput::default()
        };
        let impact = self.sim.step(&input, &WorldGround { collision, surfaces }, Some((collision, &*props)));
        for &(id, _) in &impact.knocked {
            props.knock(id);
        }
        self.previous = self.current;
        self.current = self.sim.pose();
        self.steps += 1;
        self.airborne = match self.sim.telemetry().wheels_on_ground == 0 {
            true => self.airborne + STEP,
            false => 0.0,
        };
        self.idle = match self.sim.telemetry().speed_mps.abs() < IDLE_SPEED {
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
        let state = traffic::CarState {
            body: self.body(),
            forward_speed: view.forward_speed,
            radius: self.radius,
            mass: self.mass,
        };
        match &mut self.role {
            Role::Traffic => {
                if let Accident::InProgress(left) = self.accident {
                    self.accident = match left {
                        0 | 1 => Accident::Over,
                        n => Accident::InProgress(n - 1),
                    };
                }
                let mut others: Vec<Body> =
                    ctx.bodies.iter().enumerate().filter(|&(i, _)| i != ctx.me).map(|(_, b)| b.0).collect();
                others.extend(ctx.player);
                let think =
                    traffic::think(ctx.net, &mut self.nav, &state, &others, self.blocked, ctx.rng, &self.cruise);
                self.target = think.target;
                self.speed = think.speed;
                self.blocked = think.blocked;
            }
            Role::Cop(cop) => {
                let Some(player) = ctx.player else {
                    self.speed = 0.0;
                    return;
                };
                let traffic: Vec<Body> = ctx
                    .bodies
                    .iter()
                    .enumerate()
                    .filter(|&(i, (_, is_cop))| i != ctx.me && !is_cop)
                    .map(|(_, b)| b.0)
                    .collect();
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
                let think = cop.think(ctx, &mut self.nav, &state, &Target { body: player }, &traffic);
                self.target = think.target;
                self.speed = think.speed;
            }
        }
    }

    /// The car's physics, for hits between cars.
    pub(super) fn sim_mut(&mut self) -> &mut CarSim {
        &mut self.sim
    }

    /// A hit with impulse `impulse` (N s). A hard one shocks the car; one with the player's car also starts an
    /// accident (the car stops thinking about the road).
    pub(super) fn on_hit(&mut self, impulse: f32, by_player: bool) {
        let speed_change = impulse / self.mass.max(1.0);
        if speed_change > SHOCK_MIN_SPEED_CHANGE {
            self.shock = self.shock.max((speed_change / SHOCK_FORCE).min(1.0));
        }
        if by_player {
            log::info!("traffic: {} was hit by the player (impulse {impulse:.0} N s)", self.name);
        }
        if by_player && self.accident == Accident::None {
            self.accident = Accident::InProgress(ACCIDENT_THINKS);
        }
    }

    /// Where the car is in physics space and which way it faces, for the spawner's spacing test.
    pub fn physics_position(&self) -> Vec3 {
        Vec3::from(space::to_physics(self.current.position))
    }
}
