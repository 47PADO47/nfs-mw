//! Acceleration observed by the contact camera after every shared-world response.

use glam::{Mat4, Vec3};

use super::CarSim;
use crate::scenes::world::space;
use crate::viewer::camera::original::{Avoidable, BodyInput};

impl CarSim {
    pub fn camera_acceleration(&self) -> Vec3 {
        space::to_render(((self.body().linear_velocity - self.before_velocity) / self.step_delta).to_array())
    }

    pub fn original_camera_body(&self, zoom: f32, look_back: bool) -> BodyInput {
        let body = self.vehicle.body();
        let rotation = body.rotation();
        let cg = body.cog();
        let position = body.position + rotation * Vec3::new(0.0, cg.y, cg.z);
        let grounded = self.wheels_down() > 0;
        let drift = crate::viewer::camera::original::drift(self.vehicle.slip_angle(), self.vehicle.speed(), grounded);
        BodyInput {
            position: space::to_render(position.to_array()),
            forward: space::to_render(rotation.z_axis.to_array()),
            up: space::to_render(rotation.y_axis.to_array()),
            velocity: space::to_render(body.linear_velocity.to_array()),
            // The retained native anchor producer computes scalar speed change.
            // Timeline supplies it after sampling this physical frame.
            acceleration: Vec3::ZERO,
            speed: self.vehicle.speed(),
            top_speed: self.vehicle.powertrain().max_speed(),
            gear: self.original_camera_gear(),
            grounded,
            nos_engaged: self.ride.is_none() && self.vehicle.nos_burning(),
            // Timeline owns the real-time impact timer and nearby-body contribution.
            collision_damping: 0.0,
            drift,
            up_influence: Vec3::ZERO,
            zoom,
            look_back,
        }
    }

    pub fn original_camera_overrev(&self) -> bool {
        let red_line = match &self.ride {
            Some(_) => self.max_rpm(),
            None => self.vehicle.powertrain().engine_spec().red_line,
        };
        self.engine_rpm() > red_line - 500.0
    }

    pub fn original_camera_gear(&self) -> i32 {
        match &self.ride {
            // The camera counts gears as the gearbox ids do: reverse 0, neutral 1, first 2.
            Some(ride) => ride.bike.gear() as i32 + 1,
            None => self.vehicle.gear() as i32,
        }
    }

    pub fn original_camera_braking(&self) -> bool {
        let controls = self.controls();
        controls.brake > 0.0 || controls.handbrake > 0.0
    }

    pub fn camera_avoidable(&self) -> Avoidable {
        let body = self.vehicle.body();
        let rotation = body.rotation();
        let convert = |v: Vec3| space::to_render(v.to_array());
        let half = body.dimension();
        Avoidable {
            transform: Mat4::from_cols(
                convert(rotation.z_axis).extend(0.0),
                convert(rotation.x_axis).extend(0.0),
                convert(rotation.y_axis).extend(0.0),
                convert(body.position).extend(1.0),
            ),
            half_extents: Vec3::new(half.z, half.x, half.y),
            velocity: convert(body.linear_velocity),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_vehicle::VehicleSpec;
    use nfsmw_data::car::physics::{CarBounds, CarPhysics, WallSpec};

    fn car() -> CarSim {
        let mut spec = VehicleSpec::example();
        spec.body.cg = Vec3::new(0.4, -0.2, 0.6);
        let bounds = CarBounds { half_dimensions: spec.dimension, pivot: Vec3::new(5.0, 6.0, 7.0) };
        let mut car = CarSim::new(
            CarPhysics {
                effect_source: 0,
                spec,
                bounds,
                tire_integration: Default::default(),
                jump_config: Default::default(),
                walls: WallSpec::default(),
                geometry: None,
                events: None,
                reactions: None,
                bike: None,
            },
            [0.2; 4],
        );
        car.vehicle.body_mut().place(Vec3::new(10.0, 20.0, 30.0), glam::Quat::IDENTITY);
        car.vehicle.body_mut().linear_velocity = Vec3::new(1.0, 2.0, 3.0);
        car
    }

    #[test]
    fn native_anchor_keeps_physical_origin_and_excludes_lateral_cog() {
        let car = car();
        let body = car.original_camera_body(0.6, true);
        assert!((body.position - Vec3::new(30.6, -10.0, 19.8)).length() < 1e-5);
        assert_eq!(body.forward, Vec3::X);
        assert_eq!(body.up, Vec3::Z);
        assert_eq!(body.velocity, Vec3::new(3.0, -1.0, 2.0));
        assert_eq!(body.zoom, 0.6);
        assert_eq!(body.gear, 2, "native first gear is2; HUD display uses1");
        assert!(body.look_back);
        assert_eq!(body.drift, 0.0, "airborne vehicles do not supply grounded drift");
    }

    #[test]
    fn avoidable_axes_and_extents_share_the_same_physical_center() {
        let car = car();
        let frame = car.camera_avoidable();
        assert_eq!(frame.transform.w_axis.truncate(), Vec3::new(30.0, -10.0, 20.0));
        assert_eq!(frame.transform.x_axis.truncate(), Vec3::X);
        assert_eq!(frame.transform.y_axis.truncate(), -Vec3::Y);
        assert_eq!(frame.transform.z_axis.truncate(), Vec3::Z);
        let size = car.vehicle.body().dimension();
        assert_eq!(frame.half_extents, Vec3::new(size.z, size.x, size.y));
        assert_eq!(frame.velocity, Vec3::new(3.0, -1.0, 2.0));
    }
}
