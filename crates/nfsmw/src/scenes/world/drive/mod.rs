//! Driving a car through the streamed city: the car, its physics on a fixed 60 Hz step, the chase
//! camera and the scripted driver. The world scene owns the streaming and decides when and where
//! the car appears; this module owns everything about the car itself.

mod clock;
mod ground;
mod input;
mod rig;
mod script;
mod sim;

use blackbox_collision::CollisionWorld;
use glam::Vec3;
use nfsmw_data::car::physics::{CarPhysics, SurfaceTable};

use super::road::Spawn;
use super::space;
use crate::input::ActionState;
use crate::viewer::camera::{ChaseCamera, Followed};
use clock::FixedClock;
use ground::WorldGround;
pub use input::DriveInput;
pub use rig::{CarPose, CarRig};
pub use script::DriveScript;
use sim::{CarSim, Telemetry};

/// Physics steps run per scene update while a script drives a screenshot run (which has no frame time).
const SCRIPT_STEPS_PER_UPDATE: u32 = 6;

/// A scripted driver and how far into it the car is.
struct ScriptRun {
    script: DriveScript,
    time: f32,
}

/// Where the car should appear once the road around it has loaded.
#[derive(Debug, Clone, Copy)]
pub struct SpawnRequest {
    /// Map position (x, y) to look for a road near.
    pub near: [f32; 2],
    /// Heading in radians from +X to prefer, when the car should keep facing the same way.
    pub heading: Option<f32>,
}

pub struct Drive {
    pub car_name: String,
    rig: CarRig,
    physics: CarPhysics,
    sim: Option<CarSim>,
    /// Set while the car waits for a road: the physics does not run.
    request: Option<SpawnRequest>,
    clock: FixedClock,
    previous: CarPose,
    current: CarPose,
    telemetry: Telemetry,
    chase: ChaseCamera,
    script: Option<ScriptRun>,
    /// A screenshot run drove the script in batches: once it ends the car stays where it stopped.
    batch_run: bool,
    /// Physics steps run since the car was last placed, for the once-a-second trace of a scripted run.
    steps: u32,
}

impl Drive {
    pub fn new(
        car_name: String,
        rig: CarRig,
        physics: CarPhysics,
        request: SpawnRequest,
        script: Option<DriveScript>,
    ) -> Self {
        let pose = CarPose {
            position: Vec3::ZERO,
            rotation: glam::Quat::IDENTITY,
            wheels: [nfsmw_data::car::WheelPose::default(); 4],
        };
        Self {
            car_name,
            rig,
            physics,
            sim: None,
            request: Some(request),
            clock: FixedClock::default(),
            previous: pose,
            current: pose,
            telemetry: Telemetry::default(),
            chase: ChaseCamera::default(),
            script: script.map(|script| ScriptRun { script, time: 0.0 }),
            batch_run: false,
            steps: 0,
        }
    }

    /// The map position residency should follow, if the car is not on the road yet.
    pub fn waiting_for_road(&self) -> Option<SpawnRequest> {
        self.request
    }

    /// Ask for the car to be put on the road nearest `near`, keeping `heading` if given.
    pub fn respawn_near(&mut self, near: [f32; 2], heading: Option<f32>) {
        self.request = Some(SpawnRequest { near, heading });
    }

    /// Give up waiting for a road (none was found).
    pub fn cancel_request(&mut self) {
        self.request = None;
    }

    /// Put the car on `spawn`, standing still. False when `ground` has no road there.
    pub fn spawn(&mut self, spawn: Spawn, collision: &CollisionWorld, surfaces: &SurfaceTable) -> bool {
        let ground = WorldGround { collision, surfaces };
        let spawn = match self.request.take().and_then(|r| r.heading) {
            // Keep facing the way the car did: the road runs both ways.
            Some(h) if (spawn.heading - h).cos() < 0.0 => {
                Spawn { heading: spawn.heading + std::f32::consts::PI, ..spawn }
            }
            _ => spawn,
        };
        let rest = self.rig.rest_heights();
        let sim = self.sim.get_or_insert_with(|| CarSim::new(self.physics.clone(), rest));
        if !sim.place(&ground, spawn) {
            return false;
        }
        let pose = sim.pose();
        (self.previous, self.current) = (pose, pose);
        self.telemetry = sim.telemetry();
        self.clock.reset();
        self.chase.snap();
        self.steps = 0;
        true
    }

    /// Swap the car model; the car is put back on the road where it stands.
    pub fn set_car(
        &mut self,
        renderer: &mut blackbox_render::Renderer,
        name: String,
        rig: CarRig,
        physics: CarPhysics,
    ) {
        std::mem::replace(&mut self.rig, rig).release(renderer);
        self.physics = physics;
        self.car_name = name;
        if self.sim.take().is_some() {
            let at = self.current.position;
            self.request = Some(SpawnRequest { near: [at.x, at.y], heading: Some(self.heading()) });
        }
    }

    /// Heading of the car, radians from +X.
    pub fn heading(&self) -> f32 {
        let forward = self.current.rotation * Vec3::X;
        forward.y.atan2(forward.x)
    }

    pub fn position(&self) -> Vec3 {
        self.current.position
    }

    /// Whether the script (if any) has run out.
    pub fn script_finished(&self) -> bool {
        self.script.as_ref().is_none_or(|s| s.script.finished(s.time))
    }

    /// Run the physics for `dt` seconds of frame time (`dt == 0` with a script means a screenshot
    /// run: it steps a fixed batch instead, and stops when the script ends).
    /// A batch run also waits until the map around the car (`loaded`) has streamed in, so the car
    /// never outruns its road.
    pub fn step(
        &mut self,
        collision: &CollisionWorld,
        surfaces: &SurfaceTable,
        actions: &ActionState,
        dt: f32,
        loaded: bool,
    ) {
        if self.request.is_some() {
            return;
        }
        if self.sim.is_none() {
            return;
        }
        let batch = dt == 0.0 && self.script.as_ref().is_some_and(|s| !s.script.finished(s.time));
        self.batch_run |= batch;
        if self.batch_run && (self.script_finished() || !loaded) {
            return;
        }
        let steps = if batch { SCRIPT_STEPS_PER_UPDATE } else { self.clock.advance(dt) };
        let Some(sim) = self.sim.as_mut() else { return };
        let player = DriveInput::from_actions(actions);
        let ground = WorldGround { collision, surfaces };
        for n in 0..steps {
            let mut input = player;
            if let Some(run) = self.script.as_mut() {
                input = run.script.input_at(run.time);
                if !run.script.finished(run.time) || !batch {
                    run.time += clock::STEP;
                }
            } else if n > 0 {
                input = input.held();
            }
            sim.step(&input, &ground);
            self.previous = self.current;
            self.current = sim.pose();
            self.telemetry = sim.telemetry();
            self.steps += 1;
            if self.script.is_some() && self.steps.is_multiple_of(60) {
                let (p, t) = (self.current.position, &self.telemetry);
                log::info!(
                    "t={:>5.1}s  {:>6.1} km/h  {:>5.0} rpm  gear {}  at ({:.1}, {:.1}, {:.2})  {} on ground  throttle {:.1} brake {:.1} steer {:+.2}",
                    self.steps as f32 / 60.0,
                    t.speed_mps * 3.6,
                    t.rpm,
                    t.gear,
                    p.x,
                    p.y,
                    p.z,
                    t.wheels_on_ground,
                    input.throttle,
                    input.brake,
                    input.steer
                );
            }
        }
        if !sim.is_finite() {
            log::error!("the car's state is not finite; putting it back on the road");
            self.request =
                Some(SpawnRequest { near: [self.previous.position.x, self.previous.position.y], heading: None });
        }
    }

    /// The pose to draw: between the last two physics steps.
    pub fn pose(&self) -> CarPose {
        self.previous.lerp(&self.current, self.clock.alpha())
    }

    /// Move the chase camera.
    pub fn follow(&mut self, collision: &CollisionWorld, dt: f32) {
        if dt == 0.0 {
            // A screenshot run has no frame time: put the camera where it belongs at once.
            self.chase.snap();
        }
        let pose = self.pose();
        let forward = pose.rotation * Vec3::X;
        let followed =
            Followed { position: pose.position, heading: forward.y.atan2(forward.x), speed: self.telemetry.speed_mps };
        self.chase.update(followed, dt, |from, to| blocked(collision, from, to));
    }

    pub fn camera(&self) -> &ChaseCamera {
        &self.chase
    }

    /// Append the car's instances.
    pub fn instances(&self, out: &mut Vec<blackbox_render::Instance>) {
        if self.sim.is_some() {
            self.rig.instances(&self.pose(), out);
        }
    }

    /// The readout lines.
    pub fn hud(&self) -> String {
        if self.sim.is_none() || self.request.is_some() {
            return format!("{}: looking for a road...", self.car_name);
        }
        let t = &self.telemetry;
        let gear = match t.gear {
            -1 => "R".to_owned(),
            0 => "N".to_owned(),
            g => g.to_string(),
        };
        let nos = if t.nos > 0.0 { format!("  nos {:.0}%", t.nos * 100.0) } else { String::new() };
        let p = self.current.position;
        let script = self
            .script
            .as_ref()
            .map_or(String::new(), |s| format!("  script {:.1}/{:.1} s", s.time, s.script.duration()));
        format!(
            "{:>4.0} km/h  {:>5.0} rpm  gear {gear}{nos}\n{} at ({:.0}, {:.0}, {:.1}){script}",
            t.speed_mps * 3.6,
            t.rpm,
            self.car_name,
            p.x,
            p.y,
            p.z
        )
    }
}

/// How far (0..1) along `from`-`to` (render space) the first wall or surface is.
fn blocked(collision: &CollisionWorld, from: Vec3, to: Vec3) -> Option<f32> {
    let hit = collision.ray_cast(
        space::to_physics(from),
        space::to_physics(to),
        &blackbox_collision::RayOptions::default(),
    )?;
    Some(hit.t)
}
