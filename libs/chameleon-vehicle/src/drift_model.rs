//! The 2012 drift rule as a small state machine: a slide that is fast and crooked enough becomes a drift, a drift that lasts
//! pays nitrous, and straightening out, slowing down, leaving the ground or hitting something ends it. Spec:
//! `docs/formats/chameleon-drift.md`.
//!
//! The model owns no clock and no car: the caller reads the car once a step and passes what it read in a [`DriftInput`]. It
//! answers with the nitrous earned this step as a fraction of a full tank and with the facts a HUD needs. Units are metres,
//! seconds and radians; the numbers the game's data holds in other units are converted in [`DriftParams::new`].

use crate::drift::{DriftPayout, DriftPhysics};

/// A hit at least this strong (0 to 1, a full hit is 1) ends a drift. This is the rewrite's own choice; the 2012 data names
/// no collision rule for drifting.
pub const HIT_ENDS: f32 = 0.15;

/// The rule's numbers, in metres, seconds, radians and fractions of a full tank.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DriftParams {
    /// A slide starts a drift at this speed along the car or more, m/s.
    pub enter_speed: f32,
    /// ... and at this slip angle or more, radians.
    pub enter_angle: f32,
    /// A drift is over once the slip angle has stayed under this, radians ...
    pub exit_angle: f32,
    /// ... (or the speed under `enter_speed`, or the wheels off the ground) for this long, seconds.
    pub exit_hold: f32,
    /// A drift pays at this speed along the car or more, m/s.
    pub pay_speed: f32,
    /// A drift pays once it has lasted this long, seconds.
    pub pay_delay: f32,
    /// The part of a full tank a second of paying drift adds.
    pub tank_per_second: f32,
    /// The hit strength (0 to 1) that ends a drift at once.
    pub hit_ends: f32,
}

impl DriftParams {
    /// The rule for a car with `physics` that has the nitrous `payout`.
    pub fn new(physics: &DriftPhysics, payout: &DriftPayout) -> Self {
        Self {
            enter_speed: physics.trigger_speed,
            enter_angle: physics.trigger_angle.to_radians(),
            exit_angle: physics.exit_angle.to_radians(),
            exit_hold: physics.exit_hold,
            pay_speed: payout.min_speed(),
            pay_delay: payout.delay,
            tank_per_second: payout.tank_per_second(),
            hit_ends: HIT_ENDS,
        }
    }
}

impl Default for DriftParams {
    /// The median car of the install with the burn nitrous.
    fn default() -> Self {
        Self::new(&DriftPhysics::TYPICAL, &DriftPayout::BURN)
    }
}

/// What the car did over one step.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DriftInput {
    /// Seconds the step covers.
    pub dt: f32,
    /// Speed along the car, m/s (negative in reverse).
    pub speed: f32,
    /// Angle between where the car points and where it moves, radians, either sign.
    pub slip_angle: f32,
    /// At least some wheels touch the ground.
    pub grounded: bool,
    /// The strongest hit of the step, 0 to 1.
    pub hit: f32,
}

/// A drift that has just ended.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DriftSummary {
    /// Metres driven in it.
    pub distance: f32,
    /// Seconds it lasted, the time spent running out included.
    pub seconds: f32,
    /// The part of a full tank it earned.
    pub earned: f32,
}

/// The model's answer for one step.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DriftOutput {
    /// A drift is going on.
    pub drifting: bool,
    /// It is paying nitrous this step.
    pub paying: bool,
    /// The part of a full tank earned this step.
    pub gain: f32,
    /// Metres driven in the drift so far (0 outside one).
    pub distance: f32,
    /// Set on the step a drift ends.
    pub ended: Option<DriftSummary>,
}

/// A drift in progress.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Running {
    seconds: f32,
    /// How long the conditions have been off.
    off: f32,
    distance: f32,
    earned: f32,
}

/// The state machine: no drift, or one in progress.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DriftModel {
    params: DriftParams,
    running: Option<Running>,
}

impl DriftModel {
    pub fn new(params: DriftParams) -> Self {
        Self { params, running: None }
    }

    pub fn params(&self) -> &DriftParams {
        &self.params
    }

    /// Whether a drift is going on.
    pub fn drifting(&self) -> bool {
        self.running.is_some()
    }

    /// Forget the drift in progress without paying or reporting it (the car was replaced, the game paused).
    pub fn reset(&mut self) {
        self.running = None;
    }

    /// Advance by one step.
    pub fn step(&mut self, input: &DriftInput) -> DriftOutput {
        let valid = [input.dt, input.speed, input.slip_angle, input.hit].iter().all(|value| value.is_finite());
        if !valid || input.dt <= 0.0 {
            return self.idle_or_current();
        }
        let params = self.params;
        let Some(mut run) = self.running else {
            let slide = input.grounded
                && input.speed >= params.enter_speed
                && input.slip_angle.abs() >= params.enter_angle
                && input.hit < params.hit_ends;
            if slide {
                self.running = Some(Running { seconds: 0.0, off: 0.0, distance: 0.0, earned: 0.0 });
            }
            return DriftOutput { drifting: slide, ..DriftOutput::default() };
        };
        let off = !input.grounded || input.speed < params.enter_speed || input.slip_angle.abs() < params.exit_angle;
        run.off = match off {
            true => run.off + input.dt,
            false => 0.0,
        };
        run.seconds += input.dt;
        let hit = input.hit >= params.hit_ends;
        if hit || run.off >= params.exit_hold {
            self.running = None;
            let summary = DriftSummary { distance: run.distance, seconds: run.seconds, earned: run.earned };
            return DriftOutput { ended: Some(summary), ..DriftOutput::default() };
        }
        run.distance += input.speed.max(0.0) * input.dt;
        let paying = !off && run.seconds >= params.pay_delay && input.speed >= params.pay_speed;
        let gain = match paying {
            true => params.tank_per_second * input.dt,
            false => 0.0,
        };
        run.earned += gain;
        self.running = Some(run);
        DriftOutput { drifting: true, paying, gain, distance: run.distance, ended: None }
    }

    /// The answer for a step that cannot be used: the drift in progress is kept as it is.
    fn idle_or_current(&self) -> DriftOutput {
        match self.running {
            Some(run) => DriftOutput { drifting: true, distance: run.distance, ..DriftOutput::default() },
            None => DriftOutput::default(),
        }
    }
}
