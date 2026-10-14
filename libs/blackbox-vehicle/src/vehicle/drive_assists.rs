//! Axle torque, launch differential bias and traction control, chassis spec §4.2.

use glam::Vec3;

use super::Vehicle;
use crate::drivetrain::calc_split;
use crate::math::ramp;

impl Vehicle {
    pub(super) fn distribute_drive(&mut self, torque: f32, av: [f32; 4], grounded: [bool; 4]) {
        if torque.abs() <= f32::EPSILON {
            return;
        }
        let trans = &self.spec.transmission;
        let share = calc_split(
            trans.torque_split,
            trans.differential[2],
            false,
            [grounded[0] || grounded[1], grounded[2] || grounded[3]],
            [av[0] + av[1], av[2] + av[3]],
        );
        let velocity = self.body.rotation().transpose() * self.body.linear_velocity;
        for (axle, share) in share.into_iter().enumerate() {
            let axle_torque = torque * share;
            if axle_torque.abs() <= f32::EPSILON {
                continue;
            }
            let a = axle * 2;
            let pair = [self.tires[a], self.tires[a + 1]];
            let slip = (pair[0].slip + pair[1].slip) * share * 0.5;
            let lateral = (pair[0].lateral_speed + pair[1].lateral_speed) * share * 0.5;
            let (mut control, mut grip) =
                traction_control(axle_torque, slip, lateral, velocity, self.controls.steering);
            let mut bias = 0.5;
            let locked = self.burnout.state & 3 != 0;
            match self.burnout.state & 3 {
                1 | 3 => {
                    control = [1.0; 2];
                    grip = [1.0, self.burnout.traction];
                    bias = self.burnout.traction * 0.5;
                }
                2 => {
                    control = [1.0; 2];
                    grip = [self.burnout.traction, 1.0];
                    bias = 1.0 - self.burnout.traction * 0.5;
                }
                _ => {}
            }
            let split =
                calc_split(bias, trans.differential[axle], locked, [grounded[a], grounded[a + 1]], [av[a], av[a + 1]]);
            for i in 0..2 {
                if !grounded[a + i] {
                    continue;
                }
                self.tires[a + i].traction_boost *= grip[i];
                if self.tire_integration == crate::tires::TireIntegration::Reference && self.tires[a + i].brake_locked {
                    continue;
                }
                self.tires[a + i].drive_torque = axle_torque * split[i] * control[i];
            }
        }
    }
}

fn traction_control(torque: f32, slip: f32, lateral: f32, velocity: Vec3, steer: f32) -> ([f32; 2], [f32; 2]) {
    let delta_lat = lateral - velocity.x;
    if delta_lat * steer <= 0.0 || torque * slip <= 0.0 {
        return ([1.0; 2], [1.0; 2]);
    }
    let delta_fwd = slip - velocity.z;
    let angle = (steer * delta_lat.atan2(delta_fwd.abs()).to_degrees()).abs();
    let limit = ramp(delta_fwd, 1.0, 20.0) * ramp(angle, 1.0, 16.0);
    let mut control = [1.0 - limit * 0.5; 2];
    let mut grip = [1.0; 2];
    let slipping_side = usize::from(delta_lat > 0.0);
    control[slipping_side] = 1.0 - limit;
    grip[1 - slipping_side] = 1.0 + limit;
    (control, grip)
}
