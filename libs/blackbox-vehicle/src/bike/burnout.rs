//! The burnout: a bike held by its front brake with the throttle open spins its rear wheel on the spot (`docs/specs/ride6-burnout.md`).
//!
//! The reference's input handler raises one flag, `IsDoingBurnout`, every step, from what the rider holds and the speed
//! (`0x1450d7fb0`): the throttle and the front brake are pressed, the rear brake lever (the "handbrake") and the clutch lever are not, the
//! gear is a forward one and the bike is slower than `BurnoutMinSpeed`. While it is set the handler puts the factor
//! `BurnoutGripMultiplier` on both grip scales of the rear wheel and `BurnoutSlipCorrectionMultiplier` on its slip correction
//! (`0x1450d7ed0`), the launch clutch is not worked (`0x1450d7cd0`: the clutch stays where the rider's lever leaves it, engaged), the traction control
//! gives no cut (`0x14512bd20`) and the automatic gearbox does not change up for the engine's speed. The front brake, which holds the bike, is the
//! only thing that keeps it from leaving.
//!
//! Three things of the same handler close the model (`docs/specs/ride6-burnout.md`, sections 3 and 6): the lean limit is `BurnoutLeanLimit`
//! while the flag stands (`0x14510b180`), the traction control keeps its cut unless `bTCSLowSpeedModulationForBurnout` is false (it is
//! true by default: `0x1450fe630`), and the **burnout help force** (`0x1450d5050`, `0x1450bfa00`) is a yaw-rate controller that swings the rear
//! round the held front wheel when the stick is over: a force at the rear contact, along the bike's side, that is the torque the yaw rate
//! asks for divided by the lever arm.

use glam::Vec3;

use super::reverse::PRESSED;
use super::{Bike, BikeInput, REAR};

/// The numbers of a bike's burnout (`[bike.burnout]` of the manifest).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BurnoutSpec {
    /// `BurnoutGripMultiplier`: the factor on the rear tyre's longitudinal and lateral grip while the flag is set. The input handlers of the
    /// install give 0.05, the supermoto handler 0.15 (436 and 9 of the 445 handler objects of the 319 vehicles dumped).
    pub grip: f32,
    /// `BurnoutSlipCorrectionMultiplier`: the factor on the slip correction of the rear tyre's combined slip while the flag is set (the wheel
    /// holds 1 otherwise). The constructor's default is 0 and only the supermoto handler sets it (0.9), so the road bikes' burnout has no
    /// slip correction.
    pub slip_correction: f32,
    /// `BurnoutMinSpeed` (m/s), a limit despite its name: the flag needs the bike to be slower than this. The constructor's 5 km/h; no
    /// handler of the install sets it.
    pub speed_limit: f32,
    /// `BurnoutLeanLimit` (rad): the lean limit while the flag stands, in place of every other term of the limit (`0x14510b180`; 18 degrees
    /// on the road handlers, 10 on the supermoto one: 436 and 9 of the 445 handler objects). 0 leaves the limit alone.
    pub lean_limit: f32,
    /// `bTCSLowSpeedModulationForBurnout` of the traction control element: with it set the control's cut keeps working in a burnout (shaped
    /// by its speed window, which is 0 to 5 km/h in the data); clear, the cut is 0. The constructor sets it (`0x1450fe630`, the byte at
    /// `+0x48`) and no vehicle of the install overrides it.
    pub tcs_modulation: bool,
    /// The help force.
    pub help: HelpSpec,
}

/// The burnout help force's numbers (`EnableBurnoutHelpForce` and the six keys after it in the input handler).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HelpSpec {
    /// `EnableBurnoutHelpForce`: the constructor's true; the supermoto handler turns it off (9 of the 445 handler objects).
    pub enabled: bool,
    /// `BurnoutHelpForceFactor`, the constructor's 1: the share of the force the controller asks for.
    pub factor: f32,
    /// `BurnoutDesiredAngularSpeed` (rad/s; the data is in degrees per second): the yaw rate of the full stick, the constructor's 45.
    pub yaw_rate: f32,
    /// `BurnoutSteerFilterTime` (s), the constructor's 1: the stick's filter takes this long per unit of square root of the gap.
    pub steer_filter: f32,
    /// `BurnoutHelpForceRPMPercentageRange`, the constructor's 0.75 to 0.95: the force comes in over this share of the rev limit.
    pub rpm_range: [f32; 2],
    /// `BurnoutHelpReduction`, `BurnoutHelpReductionDuration` (s) and `BurnoutHelpRaiseRate` (1/s), the constructor's 1, 0.1 and 10: a
    /// collision cuts the force by `reduction`, holds that for `reduction_duration` and gives it back at `raise_rate` (`0x1450dd9c0`,
    /// `0x1450e0480`).
    pub reduction: f32,
    pub reduction_duration: f32,
    pub raise_rate: f32,
}

impl Default for HelpSpec {
    /// The input handler's constructor values (`0x1450bbf80`), which no handler asset of the install overrides except the supermoto's `enabled`.
    fn default() -> Self {
        Self {
            enabled: true,
            factor: 1.0,
            yaw_rate: 45.0_f32.to_radians(),
            steer_filter: 1.0,
            rpm_range: [0.75, 0.95],
            reduction: 1.0,
            reduction_duration: 0.1,
            raise_rate: 10.0,
        }
    }
}

impl Default for BurnoutSpec {
    /// The road bikes' handler: the grip multiplier and lean limit of 436 of the 445 handler objects, the constructor's slip correction of 0,
    /// speed limit, traction control flag and help force.
    fn default() -> Self {
        Self {
            grip: 0.05,
            slip_correction: 0.0,
            speed_limit: 5.0 / 3.6,
            lean_limit: 18.0_f32.to_radians(),
            tcs_modulation: true,
            help: HelpSpec::default(),
        }
    }
}

impl BurnoutSpec {
    /// Whether the numbers can run a burnout: finite, the grip in 0 to 1, a speed limit above 0, a lean limit under a right angle, and a help
    /// force with a share, a yaw rate and a filter that are not negative, a range of rpm shares and a reduction in 0 to 1.
    pub fn is_valid(&self) -> bool {
        let help = &self.help;
        let finite = [
            self.grip,
            self.slip_correction,
            self.speed_limit,
            self.lean_limit,
            help.factor,
            help.yaw_rate,
            help.steer_filter,
            help.rpm_range[0],
            help.rpm_range[1],
            help.reduction,
            help.reduction_duration,
            help.raise_rate,
        ]
        .iter()
        .all(|v| v.is_finite());
        let help_ok = help.factor >= 0.0
            && help.yaw_rate >= 0.0
            && help.steer_filter >= 0.0
            && (0.0..=1.0).contains(&help.reduction)
            && help.reduction_duration >= 0.0
            && help.raise_rate >= 0.0;
        finite
            && help_ok
            && (0.0..=1.0).contains(&self.grip)
            && self.slip_correction >= 0.0
            && self.speed_limit > 0.0
            && (0.0..=std::f32::consts::FRAC_PI_2).contains(&self.lean_limit)
    }
}

/// How far (as a share of the lever) the rear lever may be from the front one and still be the brake pedal's own: a host with one brake axis
/// pulls both brakes with it, and the pedal is not the reference's "handbrake" (the rear lever alone).
pub const JOINED_TOLERANCE: f32 = 0.02;

/// What the rider holds and the bike does that the flag depends on.
#[derive(Clone, Copy, Debug)]
pub struct Request {
    pub throttle: f32,
    pub front_brake: f32,
    pub rear_brake: f32,
    /// The speed along the bike (m/s).
    pub speed: f32,
}

/// Whether the rear lever is the handbrake: pressed, and not just the pedal pulling both levers by the same amount (a rear lever above the
/// front one is the hand's, whatever the pedal does). With separate axes (a bike with its own front and rear levers) any pressed rear lever
/// counts, as in the reference; the pedal that pulls both is the one case that has to be told apart.
pub fn handbrake(front: f32, rear: f32) -> bool {
    rear > PRESSED && (rear - front).abs() > JOINED_TOLERANCE
}

impl Request {
    /// The reference's `IsDoingBurnout` (`0x1450d7fb0`): throttle and front brake pressed, no handbrake, slower than the limit. (The clutch
    /// lever is never held here, the gearbox is automatic, and the gear is never reverse or neutral in this model.)
    pub fn is_burnout(&self, spec: &BurnoutSpec) -> bool {
        self.throttle > PRESSED
            && self.front_brake > PRESSED
            && !handbrake(self.front_brake, self.rear_brake)
            && self.speed.abs() < spec.speed_limit
    }
}

/// The nearest the rear contact may be to the axis through the centre of gravity (m) for the help force to act: the reference's 0.1 cm
/// (`0x148fbaccc`, read in centimetres), under which the lever arm of the force is nothing.
const MIN_ARM: f32 = 0.001;
/// The least step (s) the help divides the yaw rate gap by (`0x148f96ec8`).
const MIN_STEP: f32 = 0.001;

/// What the help force keeps from step to step.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct HelpState {
    /// The stick through the help's filter (the handler's `+0xd68`), -1 to 1.
    filtered: f32,
    /// The yaw rate asked of the bike in the last sub-step (rad/s, positive to the right), 0 out of a burnout.
    wanted: f32,
    /// The throttle the engine got in this sub-step, after the traction control and the rider help (the reference reads `Actions.Throttle`).
    throttle: f32,
    /// The collision's cut of the force, 0 to 1 (`+0xd74`), and how long it still holds (`+0xd78`, s).
    reduction: f32,
    hold: f32,
}

/// The stick through the help's filter (`0x1450bfa00`): the gap between the filtered stick and the stick closes in the domain of its square
/// root, `time x dt` per step, so a stick thrown over from rest takes `time` seconds to arrive, quickly at first and slowly at the end.
pub fn filter_stick(filtered: f32, stick: f32, time: f32, dt: f32) -> f32 {
    let gap = filtered - stick;
    let left = gap.abs().sqrt() - dt * time;
    match left > 0.0 {
        true => stick + gap.signum() * left * left,
        false => stick,
    }
}

/// How far the engine is into the range of the help force, 0 to 1: `(rpm - low) / (high - low)` of the rev limit's shares (`0x1450ba6a0`); 1
/// when the range is unset (a negative end), a step at `high` when its ends meet.
pub fn rpm_share(range: [f32; 2], rpm: f32, max_rpm: f32) -> f32 {
    if range[0] < 0.0 || range[1] < 0.0 {
        return 1.0;
    }
    let (low, high) = (range[0] * max_rpm, range[1] * max_rpm);
    if (high - low).abs() > 1e-8 {
        return ((rpm - low) / (high - low)).clamp(0.0, 1.0);
    }
    f32::from(rpm >= high)
}

impl HelpState {
    /// The collision's cut holds for its duration, then falls at the raise rate (`0x1450e0480`).
    fn run_down(&mut self, help: &HelpSpec, dt: f32) {
        if self.hold > 0.0 {
            self.hold = (self.hold - dt).max(0.0);
            return;
        }
        if self.reduction > 0.0 {
            self.reduction = (self.reduction - dt * help.raise_rate).max(0.0);
        }
    }
}

impl Bike {
    /// The burnout's part of one sub-step: raise or drop the flag from what the rider holds, and release the rear brake while it is set (the
    /// brake pedal that pulled both levers would hold the rear wheel the burnout lets go). `levers` are the front and rear levers so far.
    /// The help force's collision cut runs down here too.
    pub(super) fn step_burnout(&mut self, dt: f32, input: &BikeInput, speed: f32, levers: (f32, f32)) -> (f32, f32) {
        let spec = self.spec.burnout;
        let request =
            Request { throttle: input.throttle, front_brake: input.front_brake, rear_brake: input.rear_brake, speed };
        let on_the_bike = !self.fallen && !self.departure.off && spec.is_valid();
        self.burnout = on_the_bike && request.is_burnout(&spec);
        self.burnout_help.run_down(&spec.help, dt);
        match self.burnout {
            true => (levers.0, 0.0),
            false => levers,
        }
    }

    /// Whether the bike is in a burnout: the throttle and the front brake held with the bike standing or crawling (the reference's
    /// `IsDoingBurnout`). A host's smoke and skid marks read it with the rear wheel's slip.
    pub fn burnout(&self) -> bool {
        self.burnout
    }

    /// The yaw rate (rad/s, positive to the right) the burnout help force asks of the bike now: the filtered stick times the data's yaw rate;
    /// 0 out of a burnout, with the help off, or with the rear wheel off the road.
    pub fn burnout_yaw_target(&self) -> f32 {
        self.burnout_help.wanted
    }

    /// The factors a burnout puts on the rear tyre: its grip (both directions) and its slip correction; 1 and 1 out of a burnout.
    pub(super) fn burnout_factors(&self) -> (f32, f32) {
        match self.burnout {
            true => (self.spec.burnout.grip, self.spec.burnout.slip_correction),
            false => (1.0, 1.0),
        }
    }

    /// The lean limit of the burnout (rad) while the flag stands and the data has one (`0x14510b180`); it replaces the limit's other terms.
    pub(super) fn burnout_lean_limit(&self) -> Option<f32> {
        let limit = self.spec.burnout.lean_limit;
        (self.burnout && limit > 0.0).then_some(limit)
    }

    /// The traction control's cut in this sub-step: out of a burnout it is the control's; in one it is too unless the element clears
    /// `bTCSLowSpeedModulationForBurnout` (`0x14513f3b0`), when the cut is 0.
    pub(super) fn burnout_traction_cut(&self, cut: f32) -> f32 {
        match self.burnout && !self.spec.burnout.tcs_modulation {
            true => 0.0,
            false => cut,
        }
    }

    /// The throttle the engine gets after every cut (`Actions.Throttle` of the reference), which the help force is scaled by.
    pub(super) fn note_drive_throttle(&mut self, throttle: f32) {
        self.burnout_help.throttle = throttle;
    }

    /// A collision cuts the help force for a moment (`BurnoutHelpReduction`, held `BurnoutHelpReductionDuration`, then given back at
    /// `BurnoutHelpRaiseRate`: `0x1450dd9c0`).
    pub(super) fn burnout_collision(&mut self) {
        let help = self.spec.burnout.help;
        self.burnout_help.reduction = help.reduction;
        self.burnout_help.hold = help.reduction_duration;
    }

    /// The unit up of the road under the bike: the two contact normals added, the world's up with no wheel down (the reference blends the
    /// normals by `TerrainNormalBlendFactor`, whose value the data does not give; this is the same sum the lean limit's bank uses).
    fn road_up(&self) -> Vec3 {
        let down = |index: usize| self.wheels[index].on_ground.then_some(self.wheels[index].normal);
        let sum = down(super::FRONT).unwrap_or(Vec3::ZERO) + down(REAR).unwrap_or(Vec3::ZERO);
        sum.try_normalize().unwrap_or(Vec3::Y)
    }

    /// The burnout help force (`0x1450d5050`, `0x1450bfa00`): a yaw-rate controller that swings the rear round the held front wheel.
    ///
    /// The stick goes through [`filter_stick`]; the yaw rate asked is `yaw_rate x` that. The torque about the road's up that closes the gap in
    /// one step is the bike's moment of inertia about that axis times the gap over the step. A force along the bike's side (the right axis
    /// without its part along the road's up), at the rear contact seen from the centre of gravity, makes that torque: the torque over the lever arm
    /// (the contact's distance from the up axis), scaled by the throttle the engine got, `factor` and the engine's share of the force's rpm
    /// range. The tyre's own side force is taken out of it, so with every scale at 1 the side force on the rear is the controller's alone,
    /// and the collision's cut (`1 - reduction`) scales what is added. It acts only in a burnout, with the rear wheel down, when `enabled`.
    /// **[M]** the reference measures the arm and the point from the vehicle's origin; the model takes its centre of gravity, about which its
    /// forces act, so that the torque is the one asked for.
    pub(super) fn apply_burnout_help(&mut self, dt: f32, stick: f32) {
        let help = self.spec.burnout.help;
        let rear = &self.wheels[REAR];
        let active = self.burnout && help.enabled && rear.on_ground && !self.fallen && !self.departure.off;
        if !active {
            self.burnout_help.filtered = 0.0;
            self.burnout_help.wanted = 0.0;
            return;
        }
        let (contact, tyre_force) = (rear.contact, rear.force);
        let axis = self.road_up();
        let rotation = self.body.rotation();
        let cog = self.body.world_cog();
        let offset = contact - cog;
        let flat = offset - axis * offset.dot(axis);
        let arm = flat.length();
        let right = rotation * Vec3::X;
        let Some(side) = (right - axis * right.dot(axis)).try_normalize() else { return };
        if arm < MIN_ARM {
            return;
        }
        let filtered = filter_stick(self.burnout_help.filtered, stick.clamp(-1.0, 1.0), help.steer_filter, dt);
        let wanted = help.yaw_rate * filtered;
        self.burnout_help.filtered = filtered;
        self.burnout_help.wanted = wanted;
        let yaw_rate = self.body.angular_velocity.dot(axis);
        let local = rotation.transpose() * axis;
        let inertia = self.body.inertia();
        let about_axis = inertia.x * local.x * local.x + inertia.y * local.y * local.y + inertia.z * local.z * local.z;
        let torque = about_axis * (wanted - yaw_rate) / dt.max(MIN_STEP);
        let share = rpm_share(help.rpm_range, self.powertrain.rpm(), self.spec.engine.max_rpm);
        // The reference's `Actions.Clutch - 1` is -1 with the lever released: the force is minus the torque over the arm.
        let wanted_force = -torque / arm * self.burnout_help.throttle * help.factor;
        let added = (wanted_force - tyre_force.dot(side)) * share * (1.0 - self.burnout_help.reduction);
        self.body.apply_force_at(side * added, cog + flat);
    }
}

#[cfg(test)]
#[path = "burnout_tests.rs"]
mod tests;
