//! One computer-driven car: the physics, the driver and the road cursor it follows.

use std::rc::Rc;

use blackbox_collision::CollisionWorld;
use blackbox_driver::{AiControls, ControllerKind, DriveFlags, DriveRequest, Driver, GearRequest};
use blackbox_roads::{RandomSource, RoadNav, RoadNetwork};
use glam::Vec3;
use nfsmw_data::car::physics::SurfaceTable;

use super::TrafficModel;
use super::traffic::{self, THINK_PERIOD, THINK_STEPS};
use crate::scenes::world::drive::{CarPose, CarRig, CarSim, DriveInput, STEP, WorldGround};
use crate::scenes::world::props::PropWorld;
use crate::scenes::world::road::Spawn;
use crate::scenes::world::space;

/// How a new car begins: when its first think comes and how fast it rolls.
#[derive(Debug, Clone, Copy)]
pub struct Start {
    /// Physics steps added to the car's clock so the thinks of different cars fall on different steps.
    pub stagger: u32,
    pub speed: f32,
}

pub struct AiCar {
    pub name: String,
    rig: Rc<CarRig>,
    sim: CarSim,
    driver: Driver,
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
}

impl AiCar {
    /// Puts a car of `model` on `spawn` (render space), rolling at `start.speed`, with a cursor in its lane.
    /// `None` when the ground is missing.
    pub fn place(
        model: &TrafficModel,
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
            driver: Driver::new(ControllerKind::Simple, false),
            target: nav.position,
            nav,
            previous: pose,
            current: pose,
            steps: 0,
            stagger: start.stagger,
            speed: start.speed,
            controls: AiControls::default(),
            radius,
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

    /// Speed along the car, m/s.
    pub fn speed(&self) -> f32 {
        self.sim.telemetry().speed_mps
    }

    pub fn controls(&self) -> AiControls {
        self.controls
    }

    /// One physics step: think when it is this car's turn, then drive.
    pub fn step(
        &mut self,
        net: &RoadNetwork,
        rng: &mut impl RandomSource,
        collision: &CollisionWorld,
        props: &mut PropWorld,
        surfaces: &SurfaceTable,
    ) {
        if (self.steps + self.stagger).is_multiple_of(THINK_STEPS) {
            self.think(net, rng);
        }
        let view = self.sim.driver_view();
        let request = DriveRequest { target: self.target, speed: self.speed, flags: DriveFlags::SIMPLE };
        let clock = self.steps as f32 * STEP;
        self.controls = self.driver.step(&view, &request, STEP, clock);
        match self.controls.gear {
            Some(GearRequest::Reverse) => self.sim.shift_reverse(),
            Some(GearRequest::First) => self.sim.shift_first(),
            Some(GearRequest::Neutral) => self.sim.shift_neutral(),
            None => {}
        }
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
    }

    /// The traffic think: keep the cursor ahead of the car and pick a speed.
    fn think(&mut self, net: &RoadNetwork, rng: &mut impl RandomSource) {
        let view = self.sim.driver_view();
        let speed = view.forward_speed.abs();
        let look = traffic::look_ahead(speed, THINK_PERIOD, self.radius);
        let ahead = traffic::cursor_ahead(&self.nav, view.position);
        if !self.nav.dead_end && ahead < look {
            self.nav.advance(net, look - ahead, Vec3::ZERO, rng);
        }
        self.target = self.nav.position;
        self.speed = traffic::wanted_speed(net, &self.nav, view.forward_speed, THINK_PERIOD);
    }

    /// Where the car is in physics space and which way it faces, for the spawner's spacing test.
    pub fn physics_position(&self) -> Vec3 {
        Vec3::from(space::to_physics(self.current.position))
    }
}
