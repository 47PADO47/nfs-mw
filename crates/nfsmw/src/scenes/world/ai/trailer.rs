//! The trailer a semi tractor pulls: its own car physics, held to the tractor by a ball joint at the 5th
//! wheel until the joint lets go.
//! Spec: `docs/specs/ai-traffic.md` (§2, the 5th wheel decision).

use std::rc::Rc;

use blackbox_collision::CollisionWorld;
use blackbox_vehicle::rigid_body::BallJoint;
use glam::Vec3;
use nfsmw_data::car::physics::SurfaceTable;

use super::TrafficModel;
use super::hitch::{Hitch, Posture, Release};
use crate::scenes::world::drive::{CarPose, CarRig, CarSim, DriveInput, STEP, WorldGround};
use crate::scenes::world::props::PropWorld;
use crate::scenes::world::road::Spawn;

/// Height of the 5th wheel and the kingpin above the bottom of each body's box, metres. The install does not
/// say where the joint is (`docs/specs/ai-traffic.md`, 5th wheel decision); a real 5th wheel plate sits about
/// four feet above the road, and the same height on both bodies keeps the trailer level.
pub const FIFTH_WHEEL_HEIGHT: f32 = 1.2;
/// Solver passes of the joint per physics step.
const SOLVER_PASSES: usize = 4;
/// Below this speed (m/s) a body counts as standing still.
const IDLE_SPEED: f32 = 0.5;

/// The joint of a tractor and its trailer, anchors in each body's own frame (physics axes, from the centre of
/// its box): the tractor's hitch is at the back of its box, the trailer's kingpin at the front of its box,
/// both `FIFTH_WHEEL_HEIGHT` above the bottom.
pub fn fifth_wheel(tractor_half: Vec3, trailer_half: Vec3) -> BallJoint {
    BallJoint {
        anchor_a: Vec3::new(0.0, FIFTH_WHEEL_HEIGHT - tractor_half.y, -tractor_half.z),
        anchor_b: Vec3::new(0.0, FIFTH_WHEEL_HEIGHT - trailer_half.y, trailer_half.z),
    }
}

/// What the tractor's driver asks of the trailer's brakes.
#[derive(Debug, Clone, Copy, Default)]
pub struct Brakes {
    pub brake: f32,
    pub handbrake: bool,
}

/// The 5th wheel: the joint that holds a tractor and its trailer together and the conditions that release it.
pub struct Coupling {
    joint: BallJoint,
    hitch: Hitch,
}

impl Coupling {
    pub fn new(tractor: &CarSim, trailer: &CarSim) -> Self {
        Self { joint: fifth_wheel(tractor.half_dimensions(), trailer.half_dimensions()), hitch: Hitch::default() }
    }

    pub fn is_hitched(&self) -> bool {
        self.hitch.is_hitched()
    }

    /// Why the joint let go, if it did.
    pub fn released(&self) -> Option<Release> {
        self.hitch.released()
    }

    /// Slides the trailer over the road so its kingpin is under the tractor's hitch. The height is left alone:
    /// on a slope the wheels are on the road where they are, and the joint closes the rest within a moment
    /// (moving the trailer down would put its wheels under the road).
    pub fn align(&self, tractor: &CarSim, trailer: &mut CarSim) {
        let (on_tractor, on_trailer) = self.joint.world_anchors(tractor.body(), trailer.body());
        let offset = on_tractor - on_trailer;
        trailer.shift_by(Vec3::new(offset.x, 0.0, offset.z));
    }

    /// Holds the trailer to the tractor for one physics step and checks whether the joint lets go. Returns
    /// whether the two are still hitched.
    pub fn hold(&mut self, tractor: &mut CarSim, trailer: &mut CarSim) -> bool {
        if !self.hitch.is_hitched() {
            return false;
        }
        self.joint.solve(tractor.body_mut(), trailer.body_mut(), STEP, SOLVER_PASSES);
        self.hitch.update(STEP, posture(tractor), posture(trailer))
    }

    /// Distance between the hitch and the kingpin, metres.
    pub fn gap(&self, tractor: &CarSim, trailer: &CarSim) -> f32 {
        self.joint.gap(tractor.body(), trailer.body())
    }
}

pub struct TrailerCar {
    rig: Rc<CarRig>,
    sim: CarSim,
    coupling: Coupling,
    previous: CarPose,
    current: CarPose,
    /// Seconds with no wheel on the ground.
    airborne: f32,
    /// Seconds standing still.
    idle: f32,
    /// Seconds out of the player's view.
    offscreen: f32,
}

impl TrailerCar {
    /// Puts a trailer of `model` behind `tractor`, which stands (or rolls at `speed`) on `spawn`, with its
    /// kingpin on the tractor's hitch. `None` when the ground is missing.
    pub fn place(
        model: &TrafficModel,
        tractor: &CarSim,
        spawn: Spawn,
        speed: f32,
        ground: &WorldGround<'_>,
    ) -> Option<Self> {
        let mut sim = CarSim::for_rig(model.physics.clone(), &model.rig);
        sim.configure_trailer();
        let coupling = Coupling::new(tractor, &sim);
        // Centre to centre along the road: the tractor's box length to the hitch, the trailer's to its centre.
        let back = tractor.half_dimensions().z + sim.half_dimensions().z;
        let (sin, cos) = spawn.heading.sin_cos();
        let position = spawn.position - Vec3::new(cos * back, sin * back, 0.0);
        if !sim.place_moving(ground, Spawn { position, heading: spawn.heading }, speed) {
            return None;
        }
        // The two roads under them may differ a little: start exactly hitched.
        coupling.align(tractor, &mut sim);
        let pose = sim.pose();
        Some(Self {
            rig: model.rig.clone(),
            sim,
            coupling,
            previous: pose,
            current: pose,
            airborne: 0.0,
            idle: 0.0,
            offscreen: 0.0,
        })
    }

    /// One physics step: roll (braking as the tractor does while hitched, and fully once it is on its own, as
    /// the spring brakes of a trailer do when the air line is gone), meet the world, then hold to the tractor.
    pub fn step(
        &mut self,
        tractor: &mut CarSim,
        brakes: Brakes,
        collision: &CollisionWorld,
        props: &mut PropWorld,
        surfaces: &SurfaceTable,
    ) {
        let was_hitched = self.coupling.is_hitched();
        let input = match was_hitched {
            true => DriveInput { brake: brakes.brake, handbrake: brakes.handbrake, ..DriveInput::default() },
            false => DriveInput { brake: 1.0, ..DriveInput::default() },
        };
        let ground = WorldGround { collision, surfaces };
        let impact = self.sim.step(&input, &ground, Some((collision, &*props)));
        for &(id, _) in &impact.knocked {
            props.knock(id);
        }
        if was_hitched && !self.coupling.hold(tractor, &mut self.sim) {
            let gap = self.coupling.gap(tractor, &self.sim);
            let (a, b) = (posture(tractor), posture(&self.sim));
            log::info!(
                "traffic: a trailer came loose from its tractor ({:?}, hitch gap {gap:.2} m; tractor {} wheels up.y {:.2}, trailer {} wheels up.y {:.2}, up dot {:.2})",
                self.coupling.released(),
                a.wheels_on_ground,
                a.up.y,
                b.wheels_on_ground,
                b.up.y,
                a.up.dot(b.up)
            );
        }
        self.previous = self.current;
        self.current = self.sim.pose();
        let telemetry = self.sim.telemetry();
        self.airborne = match telemetry.wheels_on_ground == 0 {
            true => self.airborne + STEP,
            false => 0.0,
        };
        self.idle = match telemetry.speed_mps.abs() < IDLE_SPEED {
            true => self.idle + STEP,
            false => 0.0,
        };
    }

    pub fn is_hitched(&self) -> bool {
        self.coupling.is_hitched()
    }

    pub fn rig(&self) -> &CarRig {
        &self.rig
    }

    pub fn pose(&self, alpha: f32) -> CarPose {
        self.previous.lerp(&self.current, alpha)
    }

    /// The trailer's position in render space.
    pub fn position(&self) -> Vec3 {
        self.current.position
    }

    /// The centre of the trailer's box in physics space.
    pub fn physics_position(&self) -> Vec3 {
        self.sim.body().position
    }

    pub fn sim_mut(&mut self) -> &mut CarSim {
        &mut self.sim
    }

    pub fn is_finite(&self) -> bool {
        self.sim.is_finite()
    }

    /// Half the trailer's size: width, height, length.
    pub fn half_dimensions(&self) -> Vec3 {
        self.sim.half_dimensions()
    }

    pub fn velocity(&self) -> Vec3 {
        self.sim.velocity()
    }

    pub fn airborne_time(&self) -> f32 {
        self.airborne
    }

    pub fn idle_time(&self) -> f32 {
        self.idle
    }

    pub fn offscreen_time(&self) -> f32 {
        self.offscreen
    }

    /// Adds `dt` to the time the trailer has been out of view, or clears it when it is in view.
    pub fn note_view(&mut self, in_view: bool, dt: f32) {
        self.offscreen = match in_view {
            true => 0.0,
            false => self.offscreen + dt,
        };
    }

    /// Where the trailer faces in the ground plane, as the unit (x, z) vector of physics space.
    pub fn forward(&self) -> glam::Vec2 {
        let forward = self.sim.body().rotation().z_axis;
        glam::Vec2::new(forward.x, forward.z).try_normalize().unwrap_or(glam::Vec2::Y)
    }
}

/// What the joint looks at of a car.
fn posture(sim: &CarSim) -> Posture {
    Posture { up: sim.up(), wheels_on_ground: sim.telemetry().wheels_on_ground }
}

#[cfg(test)]
mod tests;
