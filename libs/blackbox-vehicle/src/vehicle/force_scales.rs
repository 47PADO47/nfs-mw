//! Caller-owned, per-instance force multipliers. Installed hardware stays unchanged.
use super::Vehicle;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ForceScales {
    pub positive_drive_torque: f32,
    pub drag: f32,
    pub downforce: f32,
    /// Per-frame tire override; absent retains the normal tire policy.
    pub tire_steering_drag: Option<f32>,
}

impl Default for ForceScales {
    fn default() -> Self {
        Self { positive_drive_torque: 1.0, drag: 1.0, downforce: 1.0, tire_steering_drag: None }
    }
}

impl Vehicle {
    pub fn set_force_scales(&mut self, scales: ForceScales) {
        self.force_scales = ForceScales {
            positive_drive_torque: valid(scales.positive_drive_torque),
            drag: valid(scales.drag),
            downforce: valid(scales.downforce),
            tire_steering_drag: scales.tire_steering_drag.map(valid),
        };
    }
}

fn valid(value: f32) -> f32 {
    if !value.is_finite() {
        return 1.0;
    }
    value.max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn force_scales_change_aerodynamic_forces_without_changing_hardware() {
        let mut base = Vehicle::new(super::super::VehicleSpec::example());
        base.place_moving(Vec3::Y, glam::Quat::IDENTITY, 60.0);
        let original = base.spec().aero;
        let mut scaled = base.clone();
        scaled.set_force_scales(ForceScales { drag: 0.625, downforce: 1.1875, ..Default::default() });
        base.apply_aero();
        scaled.apply_aero();
        assert_eq!(scaled.spec().aero.drag_coefficient, original.drag_coefficient);
        assert_eq!(scaled.spec().aero.aero_coefficient, original.aero_coefficient);
        assert!((scaled.body.pending_force().z / base.body.pending_force().z - 0.625).abs() < 1e-5);
        assert!((scaled.body.pending_force().y / base.body.pending_force().y - 1.1875).abs() < 1e-5);
        scaled.set_force_scales(ForceScales {
            positive_drive_torque: f32::NAN,
            drag: -1.0,
            downforce: f32::INFINITY,
            ..Default::default()
        });
        assert_eq!(scaled.force_scales, ForceScales { drag: 0.0, ..Default::default() });
    }

    #[test]
    fn force_scales_reach_the_running_powertrain() {
        let ground = crate::ground::FlatGround::new(0.0);
        let mut base = Vehicle::new(super::super::VehicleSpec::example());
        assert!(base.place_on_ground(&ground, 0.0, 0.0, 5.0, 0.0));
        let input = crate::InputState { throttle: 1.0, ..Default::default() };
        for _ in 0..100 {
            base.step(crate::FIXED_STEP, &input, &ground);
        }
        let mut scaled = base.clone();
        scaled.set_force_scales(ForceScales { positive_drive_torque: 1.1875, ..Default::default() });
        base.step(crate::FIXED_STEP, &input, &ground);
        scaled.step(crate::FIXED_STEP, &input, &ground);
        let torque = base.powertrain.drive_torque();
        assert!(torque > 0.0, "the fixture must exercise positive drive torque");
        assert!((scaled.powertrain.drive_torque() / torque - 1.1875).abs() < 1e-5);
    }

    #[test]
    fn per_frame_tire_override_reaches_every_tire_and_yields_to_a_maneuver() {
        let mut car = Vehicle::new(super::super::VehicleSpec::example());
        let ground = crate::ground::FlatGround::new(0.0);
        assert!(car.place_on_ground(&ground, 0.0, 0.0, 5.0, 0.0));
        car.set_force_scales(ForceScales { tire_steering_drag: Some(0.33125), ..Default::default() });
        car.step(crate::FIXED_STEP, &crate::InputState::default(), &ground);
        assert!(car.tires.iter().all(|tire| tire.drag_reduction == 0.33125));
        car.maneuver.tire_steering_drag = Some(1.0);
        car.step(crate::FIXED_STEP, &crate::InputState::default(), &ground);
        assert!(car.tires.iter().all(|tire| tire.drag_reduction == 1.0));
        car.maneuver = Default::default();
        car.set_force_scales(ForceScales::default());
        car.step(crate::FIXED_STEP, &crate::InputState::default(), &ground);
        assert!(car.tires.iter().all(|tire| tire.drag_reduction == 0.15));
    }
}
