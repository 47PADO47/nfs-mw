//! The steering wheel switches: extra controls that stay off until the player says a wheel has them.

/// Which of a wheel's optional controls are in use (docs/specs/vehicle-manual-shifting.md, section 6).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WheelOptions {
    /// The clutch pedal (the `clutch` action) is read: while it is pressed the clutch stays open.
    pub manual_clutch: bool,
    /// The gear actions come from a selector that holds a gear: no gear held is neutral, and the brake pedal never
    /// picks reverse.
    pub h_shifter: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Partial, Settings};

    #[test]
    fn both_switches_are_off_by_default_and_follow_the_settings() {
        assert_eq!(WheelOptions::default(), WheelOptions { manual_clutch: false, h_shifter: false });
        let s = Settings::from(Partial { manual_clutch: Some(true), ..Partial::default() });
        assert_eq!(s.wheel_options(), WheelOptions { manual_clutch: true, h_shifter: false });
        let s = Settings::from(Partial { h_shifter: Some(true), ..Partial::default() });
        assert_eq!(s.wheel_options(), WheelOptions { manual_clutch: false, h_shifter: true });
    }
}
