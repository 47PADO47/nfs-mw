//! When the tail pipes flame (docs/specs/exhaust-flames.md, sections 2 and 8.3): the blow-off after a gear change
//! of an engine that allows it, and the backfire at a sputter pop while off the throttle. Engaged nitrous
//! takes priority over these transient triggers in the caller.

/// Speed (m/s) below which a gear change does not flame.
pub const MIN_SHIFT_SPEED: f32 = 10.0;

/// `ecar` `ShiftSpeed` and `ShiftAngle`: how fast and how far the car pitches at a shift (degrees per second,
/// degrees). Their ratio sets how long the blow-off lasts; 0 for either means it never shows.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ShiftTiming {
    pub speed: f32,
    pub angle: f32,
}

impl ShiftTiming {
    /// Seconds one shift event lasts at full strength.
    pub fn duration(&self) -> f32 {
        match self.speed > 0.0 && self.angle > 0.0 {
            true => self.angle / self.speed,
            false => 0.0,
        }
    }
}

/// Whether the engine allows the shift blow-off: an upgraded engine, or one that cannot be upgraded.
pub fn blowoff_allowed(engine_level: i32, engine_upgrades: i32) -> bool {
    engine_level != 0 || engine_level == engine_upgrades
}

/// The gear the physics reports: -1 reverse, 0 neutral, 1 and up forward.
fn is_forward(gear: i32) -> bool {
    gear >= 1
}

/// The shift event and its decay: `shifting` is 0 when idle and +-1 right after a gear change.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ShiftEvent {
    shifting: f32,
    gear: Option<i32>,
}

impl ShiftEvent {
    /// Note the gear of this step: a change raises an up-shift (+1) or a down-shift (-1) event.
    pub fn note_gear(&mut self, gear: i32) -> bool {
        self.note_gear_where(gear, |_| true)
    }

    /// Like [`Self::note_gear`], but a change raises its event only if `allowed(up)` says so (`up` is true for an
    /// up-shift). The gear is noted either way; the result is whether an event was raised.
    pub fn note_gear_where(&mut self, gear: i32, allowed: impl Fn(bool) -> bool) -> bool {
        let Some(previous) = self.gear.replace(gear) else { return false };
        if gear == previous || !allowed(gear > previous) {
            return false;
        }
        if gear > previous {
            self.shifting = 1.0;
        }
        if gear < previous {
            self.shifting = -1.0;
        }
        true
    }

    /// Run the decay of one step. Without a timing, a forward gear and speed the event is dropped at once.
    pub fn advance(&mut self, dt: f32, gear: i32, speed: f32, timing: ShiftTiming) {
        if self.shifting == 0.0 {
            return;
        }
        if timing.duration() <= 0.0 || !is_forward(gear) || speed <= MIN_SHIFT_SPEED {
            self.shifting = 0.0;
            return;
        }
        let step = dt / timing.duration();
        self.shifting = match self.shifting > 0.0 {
            true => (self.shifting - step).max(0.0),
            false => (self.shifting + step).min(0.0),
        };
    }

    /// A shift event is still running.
    pub fn active(&self) -> bool {
        self.shifting != 0.0
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Seconds a flame lasts after a sputter pop, and how strongly the emitters spawn meanwhile.
pub const BACKFIRE_SECONDS: f32 = 0.06;
pub const BACKFIRE_INTENSITY: f32 = 0.5;
/// Throttle below which the driver is off the pedal, so that a pop is a lift-off backfire.
pub const LIFT_OFF_THROTTLE: f32 = 0.15;

/// How a car added by a mod flames. Its sounds decide when it pops and shifts (the host has no sputter module for it) and it
/// has no `ecar` record of its own, so the host's flames are made plainly visible: a flame that outlasts a few frames at a
/// 30 Hz capture, at full strength, for every pop and for every up-shift made under power.
pub const MOD_BACKFIRE_SECONDS: f32 = 0.15;
pub const MOD_BACKFIRE_INTENSITY: f32 = 1.0;
/// The shift blow-off timing of the game's racers (`ShiftSpeed` 8, `ShiftAngle` 2.25): the class-wide `default` `ecar` record
/// that a mod car falls back on has neither, which would turn the shift flame off for good.
pub const MOD_SHIFT_TIMING: ShiftTiming = ShiftTiming { speed: 8.0, angle: 2.25 };
/// Throttle above which an up-shift of a mod car is made under power and blows off.
pub const MOD_SHIFT_LOAD: f32 = 0.35;

/// How a bike's flames differ from a mod car's: smaller and shorter, and a down-shift flicks too (`scale.rs::compact_bike`).
/// The factor on the size, reach and pipe volume of the group's emitters: a modelling choice, not data. A motorcycle's silencer
/// outlet is about 40 to 50 mm across against 70 to 100 mm for a car's tail pipe, and a bike's pop is a flick a hand long, so
/// the car's 0.28 m flame of a pop (`enhanced/burst.rs`) becomes about 0.11 m.
pub const BIKE_FLAME_SCALE: f32 = 0.4;
/// The pitch time of a bike's shift flame: 8 degrees per second over 1 degree is 0.125 s, about half of the racers'
/// 0.28 s (`MOD_SHIFT_TIMING`). A choice: the flick of a bike is short.
pub const BIKE_SHIFT_TIMING: ShiftTiming = ShiftTiming { speed: 8.0, angle: 1.0 };

/// Whether a gear change of a bike flames: an up-shift made under power blows off like a mod car's (`MOD_SHIFT_LOAD`), and a
/// down-shift with the pedal up flicks (the pops the rider's blip leaves in the pipe). A down-shift under power does not.
pub fn bike_shift_flames(up: bool, throttle: f32) -> bool {
    match up {
        true => throttle >= MOD_SHIFT_LOAD,
        false => throttle < LIFT_OFF_THROTTLE,
    }
}

/// The flame a sputter pop leaves: it runs a short time and a new pop restarts it.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Backfire {
    left: f32,
}

impl Backfire {
    /// Note the pops of this step: they start the flame when the driver is off the throttle in a forward gear.
    #[cfg(test)]
    pub fn pops(&mut self, pops: u32, throttle: f32, gear: i32) {
        self.pops_for(pops, throttle, gear, BACKFIRE_SECONDS);
    }

    /// [`Self::pops`] with a flame that lasts `seconds`.
    pub fn pops_for(&mut self, pops: u32, throttle: f32, gear: i32, seconds: f32) {
        if pops == 0 || throttle >= LIFT_OFF_THROTTLE || !is_forward(gear) {
            return;
        }
        self.left = seconds;
    }

    /// Light the flame whatever the driver does (the console's `pop`).
    pub fn start(&mut self) {
        self.left = BACKFIRE_SECONDS;
    }

    pub fn active(&self) -> bool {
        self.left > 0.0
    }

    /// Run the flame's clock after the step it was used in.
    pub fn advance(&mut self, dt: f32) {
        self.left = (self.left - dt).max(0.0);
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// How strongly the pipes spawn this step, 0 for quiet: full for a running shift event on an engine that allows
/// it, [`BACKFIRE_INTENSITY`] for a backfire alone.
#[cfg(test)]
pub fn flame_intensity(blowoff: bool, shift: &ShiftEvent, backfire: &Backfire) -> f32 {
    flame_intensity_with(blowoff, shift, backfire, BACKFIRE_INTENSITY)
}

/// [`flame_intensity`] with a backfire of `backfire_intensity`.
pub fn flame_intensity_with(blowoff: bool, shift: &ShiftEvent, backfire: &Backfire, backfire_intensity: f32) -> f32 {
    if blowoff && shift.active() {
        return 1.0;
    }
    match backfire.active() {
        true => backfire_intensity,
        false => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STEP: f32 = 1.0 / 60.0;
    const RACER: ShiftTiming = ShiftTiming { speed: 8.0, angle: 2.25 };

    /// Steps a shift event keeps flaming after a gear change into `gear` at `speed`.
    fn frames(timing: ShiftTiming, gear: i32, speed: f32) -> usize {
        let mut shift = ShiftEvent::default();
        shift.note_gear(gear - 1);
        shift.note_gear(gear);
        for count in 0..1000 {
            shift.advance(STEP, gear, speed, timing);
            if !shift.active() {
                return count;
            }
        }
        1000
    }

    #[test]
    fn the_blow_off_lasts_the_pitch_time_of_the_car() {
        // 2.25 / 8 = 0.28 s; the event is cut after the step that takes it to zero, so one frame less than 17.
        assert_eq!(frames(RACER, 3, 40.0), 16);
        // The FXX Evo pitches 0.5 degrees: 0.0625 s, 3 frames.
        assert_eq!(frames(ShiftTiming { speed: 8.0, angle: 0.5 }, 3, 40.0), 3);
    }

    #[test]
    fn a_car_without_shift_timing_never_flames_at_a_shift() {
        assert_eq!(ShiftTiming::default().duration(), 0.0);
        assert_eq!(frames(ShiftTiming::default(), 3, 40.0), 0);
        assert_eq!(frames(ShiftTiming { speed: 8.0, angle: 0.0 }, 3, 40.0), 0);
    }

    #[test]
    fn slow_neutral_and_reverse_cut_the_event() {
        assert_eq!(frames(RACER, 2, MIN_SHIFT_SPEED), 0);
        assert_eq!(frames(RACER, 0, 40.0), 0);
        assert_eq!(frames(RACER, -1, 40.0), 0);
        assert!(frames(RACER, 2, MIN_SHIFT_SPEED + 0.1) > 0);
    }

    #[test]
    fn down_shifts_flame_as_well() {
        let mut shift = ShiftEvent::default();
        shift.note_gear(4);
        shift.note_gear(3);
        shift.advance(STEP, 3, 40.0, RACER);
        assert!(shift.active());
        assert!(shift.shifting < 0.0);
    }

    #[test]
    fn the_first_gear_seen_and_an_unchanged_gear_raise_no_event() {
        let mut shift = ShiftEvent::default();
        shift.note_gear(3);
        shift.note_gear(3);
        assert!(!shift.active());
    }

    #[test]
    fn a_new_shift_restarts_the_event() {
        let mut shift = ShiftEvent::default();
        shift.note_gear(2);
        shift.note_gear(3);
        for _ in 0..10 {
            shift.advance(STEP, 3, 40.0, RACER);
        }
        let partway = shift.shifting;
        shift.note_gear(4);
        assert!(shift.shifting > partway);
        assert_eq!(shift.shifting, 1.0);
    }

    #[test]
    fn only_an_engine_that_allows_it_blows_off() {
        // Stock engine of a car with upgrades: no. Upgraded: yes. A car with no upgrade levels: always.
        assert!(!blowoff_allowed(0, 3));
        assert!(blowoff_allowed(1, 3));
        assert!(blowoff_allowed(3, 3));
        assert!(blowoff_allowed(0, 0));
    }

    #[test]
    fn the_blow_off_needs_the_flag_and_an_event() {
        let (mut shift, back) = (ShiftEvent::default(), Backfire::default());
        assert_eq!(flame_intensity(true, &shift, &back), 0.0);
        shift.note_gear(2);
        shift.note_gear(3);
        shift.advance(STEP, 3, 40.0, RACER);
        assert_eq!(flame_intensity(true, &shift, &back), 1.0);
        assert_eq!(flame_intensity(false, &shift, &back), 0.0);
    }

    #[test]
    fn a_pop_off_the_throttle_in_a_forward_gear_backfires_for_a_short_time() {
        let mut back = Backfire::default();
        back.pops(0, 0.0, 3);
        assert!(!back.active(), "no pop, no flame");
        back.pops(1, 0.0, 3);
        let mut steps = 0;
        while back.active() {
            steps += 1;
            back.advance(STEP);
        }
        // 0.06 s at 60 Hz: the flame is used in four steps.
        assert_eq!(steps, 4);
        assert_eq!(flame_intensity(false, &ShiftEvent::default(), &Backfire { left: 0.05 }), BACKFIRE_INTENSITY);
    }

    #[test]
    fn a_pop_under_power_in_reverse_or_in_neutral_does_not_backfire() {
        for (throttle, gear) in [(LIFT_OFF_THROTTLE, 3), (1.0, 3), (0.0, 0), (0.0, -1)] {
            let mut back = Backfire::default();
            back.pops(5, throttle, gear);
            assert!(!back.active(), "throttle {throttle}, gear {gear}");
        }
    }

    #[test]
    fn a_new_pop_restarts_the_flame_and_the_stronger_triggers_win() {
        let mut back = Backfire::default();
        back.start();
        back.advance(0.05);
        back.pops(1, 0.0, 2);
        assert_eq!(back, Backfire { left: BACKFIRE_SECONDS });
        assert_eq!(flame_intensity(false, &ShiftEvent::default(), &back), BACKFIRE_INTENSITY);
        back.reset();
        assert!(!back.active());
    }

    #[test]
    fn a_gated_shift_raises_its_event_only_when_allowed_and_still_notes_the_gear() {
        let mut shift = ShiftEvent::default();
        shift.note_gear(2);
        assert!(!shift.note_gear_where(3, |_| false), "refused");
        assert!(!shift.active());
        assert!(!shift.note_gear_where(3, |_| true), "the gear was noted, so 3 to 3 is no change");
        assert!(shift.note_gear_where(4, |up| up));
        assert!(shift.active());
        assert!(!shift.note_gear_where(3, |up| up), "a down-shift is refused by an up-only gate");
        assert!(shift.shifting > 0.0, "the running up-shift is kept");
    }

    #[test]
    fn a_mod_car_pop_flame_is_longer_and_the_throttle_gate_still_applies() {
        let mut back = Backfire::default();
        back.pops_for(1, 0.0, 3, MOD_BACKFIRE_SECONDS);
        assert_eq!(back, Backfire { left: MOD_BACKFIRE_SECONDS });
        const { assert!(MOD_BACKFIRE_SECONDS > 2.0 * BACKFIRE_SECONDS) };
        assert_eq!(flame_intensity_with(false, &ShiftEvent::default(), &back, MOD_BACKFIRE_INTENSITY), 1.0);
        let mut quiet = Backfire::default();
        quiet.pops_for(1, 1.0, 3, MOD_BACKFIRE_SECONDS);
        assert!(!quiet.active());
    }

    #[test]
    fn a_bike_flicks_at_an_up_shift_under_power_and_a_down_shift_off_the_pedal() {
        assert!(bike_shift_flames(true, 1.0));
        assert!(!bike_shift_flames(true, 0.0), "an up-shift off the pedal has no blow-off");
        assert!(bike_shift_flames(false, 0.0));
        assert!(!bike_shift_flames(false, 1.0), "a down-shift under power does not flame");
        assert!(BIKE_SHIFT_TIMING.duration() < MOD_SHIFT_TIMING.duration());
        assert!(BIKE_SHIFT_TIMING.duration() > 0.1, "{}", BIKE_SHIFT_TIMING.duration());
    }

    #[test]
    fn the_fallback_shift_timing_of_a_mod_car_is_the_racers() {
        assert_eq!(MOD_SHIFT_TIMING, RACER);
        assert!(MOD_SHIFT_TIMING.duration() > 0.25);
    }
}
