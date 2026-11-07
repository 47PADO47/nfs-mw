//! Positive drive-force scaling leaves engine braking unchanged.
use super::Powertrain;

impl Powertrain {
    pub fn set_positive_torque_scale(&mut self, scale: f32) {
        if !scale.is_finite() {
            self.positive_torque_scale = 1.0;
            return;
        }
        self.positive_torque_scale = scale.max(0.0);
    }

    pub(super) fn scale_output(&self, torque: f32) -> f32 {
        if torque <= 0.0 {
            return torque;
        }
        torque * self.positive_torque_scale
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn force_scales_boost_drive_and_preserve_engine_braking() {
        let spec = crate::vehicle::VehicleSpec::example();
        let mut train = Powertrain::new(
            spec.engine.clone(),
            spec.transmission.clone(),
            spec.induction,
            spec.nos,
            spec.tires.average_radius(),
        );
        assert_eq!(train.scale_output(100.0), 100.0);
        train.set_positive_torque_scale(1.1875);
        assert_eq!(train.scale_output(100.0), 118.75);
        assert_eq!(train.scale_output(-100.0), -100.0);
        assert_eq!(train.scale_output(0.0), 0.0);
    }
}
