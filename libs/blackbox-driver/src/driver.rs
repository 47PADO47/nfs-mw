//! A driver: the controllers of one car and the state they share.
//! Spec: `docs/specs/ai-driver-control.md` (§3, §4), `docs/specs/ai-driver-control-pid.md`.

use glam::Vec3;

use crate::adaptive::AdaptiveSettings;
use crate::controls::{AiControls, DriveFlags, GearRequest};
use crate::simple::{heading_to, simple_pedals, simple_steering};
use crate::steering_pid::SteeringPid;
use crate::throttle_pid::{Pedals, ThrottleInput, ThrottlePid};

/// Which family of controllers a car uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControllerKind {
    /// Traffic: steer at the target, full gas or full brake.
    Simple,
    /// Racers, cops and the autopilot: adaptive steering and a velocity-form throttle PID.
    Pid,
}

/// What the driver knows about its car this tick.
#[derive(Debug, Clone, Copy)]
pub struct VehicleView {
    pub position: Vec3,
    /// Unit forward vector of the body.
    pub forward: Vec3,
    /// Signed speed along the body's forward axis, m/s.
    pub forward_speed: f32,
    /// Speed in the xz plane, m/s.
    pub planar_speed: f32,
    pub gear_is_reverse: bool,
    /// The largest wheel angle the steering input maps to, radians.
    pub max_steer: f32,
    /// Hit hard enough to lose control for a moment (traffic coasts).
    pub in_shock: bool,
    /// Waiting on a starting grid.
    pub staging: bool,
}

/// What the action asks for: where to go, how fast, and which controllers to run.
#[derive(Debug, Clone, Copy)]
pub struct DriveRequest {
    pub target: Vec3,
    pub speed: f32,
    pub flags: DriveFlags,
}

/// Reverse-gear logic switches over above this speed (m/s): no reversing while fast.
const REVERSE_MAX_SPEED: f32 = 15.0;
/// Target more than 135° behind: shift to reverse.
const REVERSE_FACING: f32 = -0.707;

#[derive(Debug, Clone)]
pub struct Driver {
    kind: ControllerKind,
    steering: SteeringPid,
    throttle: ThrottlePid,
    reversing_speed: bool,
    steering_behind: bool,
    reverse_override: f32,
    /// Whether the gearbox sits in neutral because of shock (traffic).
    shocked: bool,
}

impl Driver {
    pub fn new(kind: ControllerKind, drag_steering: bool) -> Self {
        let settings = match drag_steering {
            true => AdaptiveSettings::DRAG,
            false => AdaptiveSettings::RACING,
        };
        Self {
            kind,
            steering: SteeringPid::new(settings),
            throttle: ThrottlePid::default(),
            reversing_speed: false,
            steering_behind: false,
            reverse_override: 0.0,
            shocked: false,
        }
    }

    pub fn kind(&self) -> ControllerKind {
        self.kind
    }

    /// Forgets the controller memories (the goal changed or the car respawned).
    pub fn reset(&mut self) {
        self.steering.reset();
        self.throttle.reset();
        self.reversing_speed = false;
        self.steering_behind = false;
        self.reverse_override = 0.0;
    }

    /// Seconds left of a forced reverse manoeuvre.
    pub fn reverse_override_left(&self) -> f32 {
        self.reverse_override
    }

    /// Starts a forced reverse for `seconds`. Returns the gear to select.
    pub fn start_reverse_override(&mut self, seconds: f32, gear_is_reverse: bool) -> GearRequest {
        self.reverse_override = seconds;
        match gear_is_reverse {
            true => GearRequest::First,
            false => GearRequest::Reverse,
        }
    }

    /// Runs the reverse, steering and gas/brake controllers once.
    pub fn step(&mut self, view: &VehicleView, request: &DriveRequest, dt: f32, clock: f32) -> AiControls {
        let mut controls = AiControls::default();
        if request.flags == DriveFlags::NONE {
            return controls;
        }
        controls.gear = self.update_override(view, dt);
        if request.flags.has(DriveFlags::REVERSE) && self.reverse_override <= 0.0 {
            controls.gear = controls.gear.or(self.reverse_logic(view, request));
        }
        if request.flags.has(DriveFlags::STEER) {
            controls.steer = self.steer(view, request, dt, clock);
        }
        if request.flags.has(DriveFlags::GAS_BRAKE) {
            let (pedals, gear) = self.pedals(view, request, dt);
            controls.gas = pedals.gas;
            controls.brake = pedals.brake;
            controls.handbrake = pedals.handbrake;
            controls.gear = controls.gear.or(gear);
        }
        controls
    }

    fn update_override(&mut self, view: &VehicleView, dt: f32) -> Option<GearRequest> {
        if self.reverse_override <= 0.0 {
            return None;
        }
        self.reverse_override -= dt;
        if self.reverse_override > 0.0 {
            return None;
        }
        self.reverse_override = 0.0;
        view.gear_is_reverse.then_some(GearRequest::First)
    }

    fn reverse_logic(&mut self, view: &VehicleView, request: &DriveRequest) -> Option<GearRequest> {
        if !view.gear_is_reverse && view.planar_speed >= REVERSE_MAX_SPEED {
            self.reversing_speed = false;
            return None;
        }
        self.reversing_speed = true;
        let facing = view.forward.dot((request.target - view.position).try_normalize()?);
        match (view.gear_is_reverse, facing) {
            (true, f) if f > 0.0 => Some(GearRequest::First),
            (false, f) if f < REVERSE_FACING => Some(GearRequest::Reverse),
            _ => None,
        }
    }

    fn steer(&mut self, view: &VehicleView, request: &DriveRequest, dt: f32, clock: f32) -> f32 {
        let parked = request.speed == 0.0 && view.planar_speed < 1.0;
        if parked {
            return 0.0;
        }
        let simple = self.kind == ControllerKind::Simple || self.reverse_override > 0.0;
        if simple {
            let (steer, behind) =
                simple_steering(view.position, view.forward, request.target, view.max_steer, view.gear_is_reverse);
            self.steering_behind = behind;
            return steer;
        }
        let (angle, _) = heading_to(view.position, view.forward, request.target);
        let steer = self.steering.step(angle, view.planar_speed, view.max_steer, dt, clock);
        self.steering_behind = false;
        match view.gear_is_reverse {
            true if view.forward_speed < 0.0 => {
                if steer < 0.0 {
                    1.0
                } else {
                    -1.0
                }
            }
            true => 0.0,
            false => steer,
        }
    }

    fn pedals(&mut self, view: &VehicleView, request: &DriveRequest, dt: f32) -> (Pedals, Option<GearRequest>) {
        match self.kind {
            ControllerKind::Pid => {
                let input = ThrottleInput {
                    speed: view.forward_speed,
                    want: request.speed,
                    reversing_gear: view.gear_is_reverse,
                    staging: view.staging,
                    steering_behind: self.steering_behind,
                    reversing_speed: self.reversing_speed,
                };
                (self.throttle.step(&input, dt), None)
            }
            ControllerKind::Simple => {
                // A traffic car hit hard coasts in neutral, then goes back to its drive gear.
                let gear = match (view.in_shock, self.shocked) {
                    (true, false) => Some(GearRequest::Neutral),
                    (false, true) => Some(GearRequest::First),
                    _ => None,
                };
                self.shocked = view.in_shock;
                if view.in_shock {
                    return (Pedals::default(), gear);
                }
                let pedals = simple_pedals(
                    view.forward_speed,
                    request.speed,
                    view.gear_is_reverse,
                    self.steering_behind,
                    self.reversing_speed,
                );
                (pedals, gear)
            }
        }
    }
}
