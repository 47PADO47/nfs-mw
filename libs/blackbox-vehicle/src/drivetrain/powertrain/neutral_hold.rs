//! Holding the gearbox in neutral: a host whose engine is switched off (or cranking) puts the box in N and keeps it there.

use super::{GEAR_NEUTRAL, Powertrain};

impl Powertrain {
    /// Hold the gearbox in neutral (`true`) or let it go (`false`). Holding shifts to neutral at once, from any gear, and
    /// refuses every other gear (the shift buttons, a gear key, the automatic box, the automatic reverse) until the hold
    /// is released. Releasing leaves the box in neutral: an automatic box then takes first by itself on its next step, a
    /// manual box stays in neutral until the driver picks a gear.
    pub fn set_neutral_hold(&mut self, on: bool) {
        self.neutral_hold = on;
        if on {
            self.shift(GEAR_NEUTRAL);
        }
    }

    /// Whether the gearbox is held in neutral.
    pub fn neutral_held(&self) -> bool {
        self.neutral_hold
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::{GEAR_FIRST, GEAR_REVERSE, TickInput, tests};
    use super::*;

    fn powertrain() -> Powertrain {
        Powertrain::new(tests::engine(), tests::trans(), Default::default(), Default::default(), 0.33)
    }

    fn tick(p: &mut Powertrain, automatic: bool) {
        let mut av = [0.0; 4];
        p.tick(&mut TickInput {
            dt: 1.0 / 60.0,
            gas: 1.0,
            nos_held: false,
            can_recharge_nos: true,
            blown: false,
            automatic,
            speed: 0.0,
            wheel_av: &mut av,
            grounded: [true; 4],
            drifting: false,
            drive_slip: 0.0,
            max_wheel_slip: 0.0,
            induction_tuning: 0.0,
            nos_tuning: 0.0,
            clutch_pedal: None,
        });
    }

    #[test]
    fn a_hold_puts_the_box_in_neutral_from_any_gear() {
        let mut p = powertrain();
        assert_eq!(p.gear(), GEAR_FIRST);
        p.set_neutral_hold(true);
        assert_eq!((p.gear(), p.neutral_held()), (GEAR_NEUTRAL, true));
        let mut reverse = powertrain();
        reverse.shift(GEAR_REVERSE);
        reverse.set_neutral_hold(true);
        assert_eq!(reverse.gear(), GEAR_NEUTRAL);
    }

    #[test]
    fn a_held_box_refuses_every_gear_and_the_automatic_box_does_not_leave_neutral() {
        let mut p = powertrain();
        p.set_neutral_hold(true);
        assert!(!p.shift(GEAR_FIRST));
        assert!(!p.select_gear(GEAR_REVERSE, 0.0));
        p.request_shift(1, false);
        for _ in 0..30 {
            tick(&mut p, true);
        }
        assert_eq!(p.gear(), GEAR_NEUTRAL, "held in N through the cranking");
    }

    #[test]
    fn released_an_automatic_box_takes_first_and_a_manual_box_stays_in_neutral() {
        let mut automatic = powertrain();
        automatic.set_neutral_hold(true);
        automatic.set_neutral_hold(false);
        assert_eq!(automatic.gear(), GEAR_NEUTRAL, "releasing does not pick a gear");
        tick(&mut automatic, true);
        assert_eq!(automatic.gear(), GEAR_FIRST);
        let mut manual = powertrain();
        manual.set_neutral_hold(true);
        manual.set_neutral_hold(false);
        for _ in 0..30 {
            tick(&mut manual, false);
        }
        assert_eq!(manual.gear(), GEAR_NEUTRAL, "a manual box waits for the driver");
        manual.request_shift(1, false);
        assert_eq!(manual.gear(), GEAR_FIRST);
    }
}
