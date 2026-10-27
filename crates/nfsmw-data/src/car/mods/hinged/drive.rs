//! A part played out by conditions (`[[damage_parts.drive]]`, `docs/modding/car-damage-parts.md`): the part's movement runs
//! forward to its full pose once the opening tests hold, stays while they and the closing tests allow, and runs back when
//! the closing tests hold. A part with several drives (the Huayra's flaps: turning, a speed band, braking) is out as far as
//! the furthest of them.
//!
//! Assumptions (not data): a drive runs at a constant rate over the movement's length, as the executable's animation
//! behaviours do, and it goes forward while its own closing tests do not hold; a drive that is out and whose closing tests
//! do not hold stays out whatever the opening tests do.

/// The car's inputs the tests read.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Inputs {
    /// The speed, mph.
    pub mph: f32,
    pub gas: f32,
    pub brake: f32,
    /// -1 to 1, positive to the right.
    pub steering: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    Speed,
    Gas,
    Brake,
    Steering,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Eq,
    Ne,
    Gt,
    Lt,
    Ge,
    Le,
}

/// One test: `input op value`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Test {
    pub input: Input,
    pub op: Op,
    pub value: f32,
}

impl Test {
    fn holds(&self, inputs: &Inputs) -> bool {
        let seen = match self.input {
            Input::Speed => inputs.mph,
            Input::Gas => inputs.gas,
            Input::Brake => inputs.brake,
            Input::Steering => inputs.steering,
        };
        match self.op {
            Op::Eq => seen == self.value,
            Op::Ne => seen != self.value,
            Op::Gt => seen > self.value,
            Op::Lt => seen < self.value,
            Op::Ge => seen >= self.value,
            Op::Le => seen <= self.value,
        }
    }
}

fn holds(tests: &[Test], any: bool, inputs: &Inputs) -> bool {
    match any {
        true => tests.iter().any(|test| test.holds(inputs)),
        false => tests.iter().all(|test| test.holds(inputs)),
    }
}

/// How a part is played out.
#[derive(Debug, Clone, PartialEq)]
pub struct Drive {
    pub open: Vec<Test>,
    pub open_any: bool,
    /// Seconds the opening tests have to hold before the movement starts.
    pub open_after: f32,
    pub close: Vec<Test>,
    pub close_any: bool,
}

/// Most drives of a part (`blackbox_modding::content::MAX_DRIVES`).
pub const MAX_DRIVES: usize = 4;

/// Where one drive is.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DriveState {
    pub amount: f32,
    up: bool,
    held: f32,
}

impl DriveState {
    /// Advance by `dt` seconds with the movement taking `seconds` for the whole way.
    pub fn step(&mut self, drive: &Drive, seconds: f32, inputs: &Inputs, dt: f32) {
        match self.up {
            false => match holds(&drive.open, drive.open_any, inputs) {
                true => {
                    self.held += dt;
                    if self.held >= drive.open_after {
                        self.up = true;
                    }
                }
                false => self.held = 0.0,
            },
            true => {
                if holds(&drive.close, drive.close_any, inputs) {
                    self.up = false;
                    self.held = 0.0;
                }
            }
        }
        let rate = dt.max(0.0) / seconds.max(0.01);
        self.amount = match self.up {
            true => (self.amount + rate).min(1.0),
            false => (self.amount - rate).max(0.0),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    fn test(input: Input, op: Op, value: f32) -> Test {
        Test { input, op, value }
    }

    /// The Huayra's front right flap turning: out above 50 mph below 137 mph with the wheel over 0.3 to the right, back
    /// when the speed leaves the band or the wheel comes back to 0.3.
    fn turning() -> Drive {
        Drive {
            open: vec![
                test(Input::Speed, Op::Gt, 50.0),
                test(Input::Speed, Op::Lt, 137.0),
                test(Input::Steering, Op::Gt, 0.3),
            ],
            open_any: false,
            open_after: 0.0,
            close: vec![
                test(Input::Speed, Op::Lt, 50.0),
                test(Input::Speed, Op::Gt, 137.0),
                test(Input::Steering, Op::Le, 0.3),
            ],
            close_any: true,
        }
    }

    fn run(state: &mut DriveState, drive: &Drive, inputs: Inputs, seconds: f32) {
        for _ in 0..(seconds / DT).round() as usize {
            state.step(drive, 0.6, &inputs, DT);
        }
    }

    fn at(mph: f32, steering: f32, brake: f32) -> Inputs {
        Inputs { mph, gas: 0.0, brake, steering }
    }

    #[test]
    fn a_turning_flap_goes_out_with_the_wheel_over_and_back_when_it_returns() {
        let (drive, mut state) = (turning(), DriveState::default());
        run(&mut state, &drive, at(100.0, 0.0, 0.0), 1.0);
        assert_eq!(state.amount, 0.0, "straight ahead it stays in");
        run(&mut state, &drive, at(100.0, 0.5, 0.0), 0.3);
        assert!((state.amount - 0.5).abs() < 0.05, "half way in half the time: {}", state.amount);
        run(&mut state, &drive, at(100.0, 0.5, 0.0), 1.0);
        assert_eq!(state.amount, 1.0);
        run(&mut state, &drive, at(100.0, 0.31, 0.0), 1.0);
        assert_eq!(state.amount, 1.0, "still over 0.3: it stays out");
        run(&mut state, &drive, at(100.0, 0.2, 0.0), 1.0);
        assert_eq!(state.amount, 0.0, "the wheel is back: it goes in");
    }

    #[test]
    fn leaving_the_speed_band_brings_the_flap_in_and_other_speeds_keep_it_in() {
        let (drive, mut state) = (turning(), DriveState::default());
        run(&mut state, &drive, at(140.0, 0.8, 0.0), 1.0);
        assert_eq!(state.amount, 0.0, "above the band it does not go out");
        run(&mut state, &drive, at(100.0, 0.8, 0.0), 1.0);
        assert_eq!(state.amount, 1.0);
        run(&mut state, &drive, at(40.0, 0.8, 0.0), 1.0);
        assert_eq!(state.amount, 0.0, "below the band it comes in although the wheel is still over");
    }

    #[test]
    fn a_drive_waits_for_its_opening_tests_to_hold_long_enough() {
        let brake = Drive {
            open: vec![test(Input::Speed, Op::Gt, 155.0), test(Input::Brake, Op::Ne, 0.0)],
            open_any: false,
            open_after: 0.4,
            close: vec![test(Input::Brake, Op::Eq, 0.0), test(Input::Speed, Op::Lt, 10.0)],
            close_any: true,
        };
        let mut state = DriveState::default();
        run(&mut state, &brake, at(170.0, 0.0, 1.0), 0.3);
        assert_eq!(state.amount, 0.0, "not held 0.4 s yet");
        run(&mut state, &brake, at(170.0, 0.0, 0.0), 0.1);
        run(&mut state, &brake, at(170.0, 0.0, 1.0), 0.3);
        assert_eq!(state.amount, 0.0, "the pause started the count again");
        run(&mut state, &brake, at(170.0, 0.0, 1.0), 0.5);
        assert!(state.amount > 0.0, "{}", state.amount);
        run(&mut state, &brake, at(170.0, 0.0, 1.0), 1.0);
        assert_eq!(state.amount, 1.0);
        run(&mut state, &brake, at(170.0, 0.0, 0.0), 1.0);
        assert_eq!(state.amount, 0.0, "the brake is released");
    }
}
