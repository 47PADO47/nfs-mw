//! The burnout: a bike held by its front brake with the throttle open spins its rear wheel on the spot (`docs/specs/ride6-burnout.md`).
//!
//! The reference's input handler raises one flag, `IsDoingBurnout`, every step, from what the rider holds and the speed
//! (`0x1450d7fb0`): the throttle and the front brake are pressed, the rear brake lever (the "handbrake") and the clutch lever are not, the
//! gear is a forward one and the bike is slower than `BurnoutMinSpeed`. While it is set the handler puts the factor
//! `BurnoutGripMultiplier` on both grip scales of the rear wheel and `BurnoutSlipCorrectionMultiplier` on its slip correction
//! (`0x1450d7ed0`), the launch clutch is not worked (`0x1450d7cd0`: the clutch stays where the rider's lever leaves it, engaged), the traction control
//! gives no cut (`0x14512bd20`) and the automatic gearbox does not change up for the engine's speed. The front brake, which holds the bike, is the
//! only thing that keeps it from leaving.

use super::reverse::PRESSED;
use super::{Bike, BikeInput};

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
}

impl Default for BurnoutSpec {
    /// The road bikes' handler: the grip multiplier of 436 of the 445 handler objects, the constructor's slip correction of 0 and speed limit.
    fn default() -> Self {
        Self { grip: 0.05, slip_correction: 0.0, speed_limit: 5.0 / 3.6 }
    }
}

impl BurnoutSpec {
    /// Whether the numbers can run a burnout: finite, the grip in 0 to 1 and a speed limit above 0.
    pub fn is_valid(&self) -> bool {
        let finite = [self.grip, self.slip_correction, self.speed_limit].iter().all(|v| v.is_finite());
        finite && (0.0..=1.0).contains(&self.grip) && self.slip_correction >= 0.0 && self.speed_limit > 0.0
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

impl Bike {
    /// The burnout's part of one sub-step: raise or drop the flag from what the rider holds, and release the rear brake while it is set (the
    /// brake pedal that pulled both levers would hold the rear wheel the burnout lets go). `levers` are the front and rear levers so far.
    pub(super) fn step_burnout(&mut self, input: &BikeInput, speed: f32, levers: (f32, f32)) -> (f32, f32) {
        let spec = self.spec.burnout;
        let request =
            Request { throttle: input.throttle, front_brake: input.front_brake, rear_brake: input.rear_brake, speed };
        let on_the_bike = !self.fallen && !self.departure.off && spec.is_valid();
        self.burnout = on_the_bike && request.is_burnout(&spec);
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

    /// The factors a burnout puts on the rear tyre: its grip (both directions) and its slip correction; 1 and 1 out of a burnout.
    pub(super) fn burnout_factors(&self) -> (f32, f32) {
        match self.burnout {
            true => (self.spec.burnout.grip, self.spec.burnout.slip_correction),
            false => (1.0, 1.0),
        }
    }
}

#[cfg(test)]
#[path = "burnout_tests.rs"]
mod tests;
