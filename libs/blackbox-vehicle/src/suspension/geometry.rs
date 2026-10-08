use glam::Vec3;

use super::spec::ChassisSpec;
use crate::math::inch_to_m;
use crate::tires::TireSpec;

/// Fixed geometry of the four wheels in the body frame (x right, y up, z forward).
#[derive(Clone, Copy, Debug)]
pub struct Geometry {
    /// Wheel contact points at the bottom of the body box: front left, front right, rear left, rear right.
    pub arms: [Vec3; 4],
    /// Rolling radius of each tire (m).
    pub radius: [f32; 4],
}

impl Geometry {
    /// `half_height` is the half height of the body box (`dimension.y`).
    pub fn new(chassis: &ChassisSpec, tires: &TireSpec, half_height: f32) -> Self {
        let axle_width = |axle: usize| chassis.track_width[axle] - tires.section_width[axle] * 0.001;
        let (wf, wr) = (axle_width(0) * 0.5, axle_width(1) * 0.5);
        let front_z = chassis.front_axle;
        let rear_z = chassis.front_axle - chassis.wheel_base;
        let y = -half_height;
        Self {
            arms: [
                Vec3::new(-wf, y, front_z),
                Vec3::new(wf, y, front_z),
                Vec3::new(-wr, y, rear_z),
                Vec3::new(wr, y, rear_z),
            ],
            radius: [tires.radius(0), tires.radius(0), tires.radius(1), tires.radius(1)],
        }
    }

    pub fn front_z(&self) -> f32 {
        self.arms[0].z
    }

    pub fn rear_z(&self) -> f32 {
        self.arms[2].z
    }
}

/// The centre of gravity in the body frame: along the wheelbase by the weight bias (centred when no wheel
/// touches the ground), and `roll_center` above the road at rest.
pub fn center_of_gravity(chassis: &ChassisSpec, half_height: f32, ride_extra: f32, any_wheel_on_ground: bool) -> Vec3 {
    let front_z = chassis.front_axle;
    let rear_z = chassis.front_axle - chassis.wheel_base;
    let bias = if any_wheel_on_ground { chassis.front_weight_bias * 0.01 } else { 0.5 };
    let ride = inch_to_m((chassis.ride_height[0] + ride_extra).max(chassis.ride_height[1] + ride_extra));
    Vec3::new(0.0, inch_to_m(chassis.roll_center) - (half_height + ride), (front_z - rear_z) * bias + rear_z)
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn chassis() -> ChassisSpec {
        ChassisSpec {
            spring_stiffness: [600.0, 550.0],
            spring_progression: [0.0; 2],
            shock_stiffness: [45.0; 2],
            shock_ext_stiffness: [60.0; 2],
            shock_valving: [0.0; 2],
            shock_digression: [0.0; 2],
            shock_blowout: 0.0,
            swaybar_stiffness: [80.0, 60.0],
            travel: [5.0; 2],
            ride_height: [4.0; 2],
            track_width: [1.62, 1.64],
            wheel_base: 2.6,
            front_axle: 1.4,
            front_weight_bias: 55.0,
            roll_center: 18.0,
        }
    }

    fn tires() -> TireSpec {
        TireSpec {
            rim_size: [17.0; 2],
            section_width: [225.0, 245.0],
            aspect_ratio: [45.0; 2],
            grip_scale: [1.0; 2],
            static_grip: [1.1; 2],
            dynamic_grip: [1.0; 2],
            steering: 1.0,
            yaw_control: vec![1.0],
            yaw_speed: 1.0,
        }
    }

    #[test]
    fn arms_sit_at_the_box_bottom_with_the_tire_width_removed() {
        let g = Geometry::new(&chassis(), &tires(), 0.65);
        assert!((g.arms[0].x + (1.62 - 0.225) / 2.0).abs() < 1e-5);
        assert!((g.arms[3].x - (1.64 - 0.245) / 2.0).abs() < 1e-5);
        assert_eq!(g.arms[0].y, -0.65);
        assert!((g.arms[0].z - 1.4).abs() < 1e-6 && (g.arms[2].z + 1.2).abs() < 1e-6);
        assert!(g.arms[0].x < 0.0 && g.arms[1].x > 0.0 && g.arms[2].x < 0.0 && g.arms[3].x > 0.0);
    }

    #[test]
    fn cog_follows_the_weight_bias() {
        let c = chassis();
        let cg = center_of_gravity(&c, 0.65, 0.0, true);
        // 55% front: 55% of the way from the rear axle (-1.2) to the front axle (1.4).
        assert!((cg.z - (-1.2 + 2.6 * 0.55)).abs() < 1e-5);
        assert!((cg.y - (0.4572 - (0.65 + 0.1016))).abs() < 1e-4);
        let flying = center_of_gravity(&c, 0.65, 0.0, false);
        assert!((flying.z - (-1.2 + 1.3)).abs() < 1e-5);
    }

    #[test]
    fn ride_height_tuning_lowers_the_cog_origin_offset() {
        let c = chassis();
        let a = center_of_gravity(&c, 0.65, 0.0, true);
        let b = center_of_gravity(&c, 0.65, 2.0, true);
        assert!(b.y < a.y);
    }
}
