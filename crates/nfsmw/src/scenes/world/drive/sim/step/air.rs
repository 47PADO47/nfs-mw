//! The player's air control of a car (`docs/specs/air-control.md`): with no wheel on the ground and the body clear of it, the stick turns the
//! body by a bounded torque. A bike's is in the library's bike (`Bike::set_air_control`); the car's body has no such step of its own, so the host
//! puts the torque on it once the driving response of the step is done (the torque then moves the body in the next step, as every force does).

use blackbox_vehicle::air_control::{AirAxes, AirControlSpec, Attitude, BodyFrame};
use blackbox_vehicle::rigid_body::BodyState;

use super::super::CarSim;

impl CarSim {
    /// Put the torque of `axes` on the car's body if it is in the air: awake, no wheel on the ground and none of its box on it either (a car on
    /// its roof or its belly is not flying). Nothing for a bike, whose own dynamics take it.
    pub(in crate::scenes::world::drive::sim) fn apply_air_control(&mut self, axes: Option<AirAxes>) {
        let Some(axes) = axes else { return };
        if self.ride.is_some() || self.vehicle.wheels_on_ground() > 0 || self.vehicle.body_on_ground() {
            return;
        }
        let body = self.vehicle.body_mut();
        if body.state != BodyState::Awake {
            return;
        }
        let frame =
            BodyFrame { rotation: body.rotation(), angular_velocity: body.angular_velocity, inertia: body.inertia() };
        let torque = AirControlSpec::CAR.torque(axes, &frame, &Attitude::default());
        if !torque.is_finite() {
            return;
        }
        body.apply_torque(torque);
    }
}
