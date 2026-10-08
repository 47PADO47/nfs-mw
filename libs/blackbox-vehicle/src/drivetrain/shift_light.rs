//! The shift light: the gearbox's shift-up wish as a gauge shows it. Spec: `docs/specs/vehicle-engine-drivetrain.md`
//! (shift points).

use super::gearbox::ShiftPotential;
use super::powertrain::Powertrain;
use crate::math::rad_to_rpm;

impl Powertrain {
    /// The box wants the next gear: the engine speed matched to the wheels is at or past the gear's shift-up
    /// point, with the clutch engaged (the wish is cleared during a shift) and a higher gear to go to. This is
    /// what lights the shift light of the tachometer; an automatic car shifts as soon as it holds.
    pub fn shift_up_wish(&self) -> bool {
        if !self.clutch.is_engaged() {
            return false;
        }
        let rpm = rad_to_rpm(self.omega_trans);
        self.shift_points.potential(&self.engine, &self.trans, self.gear, rpm, self.throttle) == ShiftPotential::Up
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{engine, trans};
    use crate::drivetrain::GEAR_FIRST;
    use crate::induction::InductionSpec;
    use crate::math::rpm_to_rad;
    use crate::nos::NosSpec;

    use super::*;

    #[test]
    fn the_light_comes_on_at_the_shift_up_point_and_not_in_the_top_gear() {
        let mut p = Powertrain::new(engine(), trans(), InductionSpec::default(), NosSpec::default(), 0.33);
        p.gear = GEAR_FIRST;
        let up = p.shift_points().up[GEAR_FIRST];
        p.omega_trans = rpm_to_rad(up - 100.0);
        assert!(!p.shift_up_wish());
        p.omega_trans = rpm_to_rad(up + 10.0);
        assert!(p.shift_up_wish());
        p.clutch.disengage();
        assert!(!p.shift_up_wish(), "a gear change in progress clears it");
        p.clutch.engage(0.0);
        p.gear = p.top_gear();
        p.omega_trans = rpm_to_rad(engine().red_line);
        assert!(!p.shift_up_wish(), "there is no higher gear");
    }
}
