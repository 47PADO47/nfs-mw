//! When the tail pipes flame (docs/specs/exhaust-flames.md, sections 2 and 8.3): the blow-off after a gear change
//! of an engine that allows it, and the backfire at a sputter pop while off the throttle. The nitrous does not
//! flame: its effect in the original differs and is not built yet.

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
    pub fn note_gear(&mut self, gear: i32) {
        let Some(previous) = self.gear.replace(gear) else { return };
        if gear > previous {
            self.shifting = 1.0;
        }
        if gear < previous {
            self.shifting = -1.0;
        }
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

/// The flame a sputter pop leaves: it runs a short time and a new pop restarts it.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Backfire {
    left: f32,
}

impl Backfire {
    /// Note the pops of this step: they start the flame when the driver is off the throttle in a forward gear.
    pub fn pops(&mut self, pops: u32, throttle: f32, gear: i32) {
        if pops == 0 || throttle >= LIFT_OFF_THROTTLE || !is_forward(gear) {
            return;
        }
        self.start();
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
pub fn flame_intensity(blowoff: bool, shift: &ShiftEvent, backfire: &Backfire) -> f32 {
    if blowoff && shift.active() {
        return 1.0;
    }
    match backfire.active() {
        true => BACKFIRE_INTENSITY,
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
}
