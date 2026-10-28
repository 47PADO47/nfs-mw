//! The parameters of a two-wheeled vehicle, in SI units and in the physics space of the crate: x right, y up, z forward.
//!
//! Every field is a plain number the caller fills from its own data. `BikeSpec::example` is a generic 450 cc
//! supermoto-class bike for tests and examples; it is not any game's data.

use glam::Vec3;

use super::magic::Mf94;

/// The whole bike.
#[derive(Clone, Debug)]
pub struct BikeSpec {
    pub chassis: ChassisSpec,
    pub front: WheelSpec,
    pub rear: WheelSpec,
    pub front_suspension: SuspensionSpec,
    pub rear_suspension: SuspensionSpec,
    pub engine: EngineSpec,
    pub transmission: TransmissionSpec,
    pub brakes: BrakeSpec,
    pub steering: SteeringSpec,
    /// The unitless quadratic drag coefficient along the bike, as `RigidBodySpec::drag` takes it.
    pub drag: f32,
}

/// Frame, mass and steering geometry.
#[derive(Clone, Debug)]
pub struct ChassisSpec {
    /// The bike without the rider (kg).
    pub mass: f32,
    /// The rider's mass, carried at the rider's own height (kg); 0 for a bike without one.
    pub rider_mass: f32,
    /// Half extents of the body box (m): the inertia and the drag area come from it.
    pub half_dimensions: Vec3,
    /// Scale of the box inertia about x (pitch), y (yaw) and z (roll).
    pub tensor_scale: Vec3,
    /// Height of the centre of gravity above the ground at rest (m).
    pub cg_height: f32,
    /// Share of the weight on the front wheel at rest, 0 to 1.
    pub front_weight: f32,
    /// Distance between the wheel centres at rest (m).
    pub wheelbase: f32,
    /// Steering-head angle from the vertical (rad), the top leaning back.
    pub rake: f32,
    /// Horizontal distance from the steering axis' ground point back to the contact patch (m).
    pub trail: f32,
}

/// One wheel with its tyre.
#[derive(Clone, Debug)]
pub struct WheelSpec {
    /// Outer radius (m).
    pub radius: f32,
    /// Width of the tyre (m).
    pub width: f32,
    /// 0 for a flat tread, 1 for a round tyre: the radius of the tyre's cross-section is this share of half the width.
    pub toroid_factor: f32,
    /// Mass of the wheel and tyre (kg).
    pub mass: f32,
    /// Moment of inertia about the axle (kg m^2).
    pub inertia: f32,
    pub tyre: Mf94,
    /// Rolling resistance as a share of the load: a constant, a part per m/s and a part per (m/s)^2.
    pub rolling: [f32; 3],
    /// The largest lateral force as a multiple of the load.
    pub lateral_grip_limit: f32,
}

/// A fork or a shock with its damper.
#[derive(Clone, Debug)]
pub struct SuspensionSpec {
    /// The length the suspension can compress over from its fully extended state (m).
    pub max_travel: f32,
    /// Spring rate at the shock (N/m).
    pub spring: f32,
    /// Stiffness of the stop that holds the suspension at full extension (N/m).
    pub top_out_stiffness: f32,
    pub damper: DamperSpec,
    /// `(share of travel, ratio)` of wheel travel to shock travel; empty for a direct (fork) acting spring.
    pub leverage: Vec<(f32, f32)>,
}

/// Two-slope damper: the slow slope up to a threshold speed, the fast slope beyond it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DamperSpec {
    pub slow_bump: f32,
    pub fast_bump: f32,
    pub bump_threshold: f32,
    pub slow_rebound: f32,
    pub fast_rebound: f32,
    pub rebound_threshold: f32,
}

#[derive(Clone, Debug)]
pub struct EngineSpec {
    /// The peak torque (N m) the torque curve's factors multiply.
    pub peak_torque: f32,
    /// `(rpm, share of the peak)`, rpm rising.
    pub torque_curve: Vec<(f32, f32)>,
    /// Drag of the closed throttle at the rev limit (N m); it grows with the speed of the engine.
    pub friction_torque: f32,
    pub idle_rpm: f32,
    pub max_rpm: f32,
}

#[derive(Clone, Debug)]
pub struct TransmissionSpec {
    /// Forward gear ratios, first gear first.
    pub gears: Vec<f32>,
    /// The ratio between the gearbox output and the rear wheel, chain included.
    pub final_ratio: f32,
    /// The share of the engine torque that reaches the wheel.
    pub efficiency: f32,
    /// The engine speed, as a share of the rev limit, at which the automatic changes up under full throttle.
    pub up_shift: f32,
    /// The engine speed, as a share of the rev limit, below which it changes down.
    pub down_shift: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrakeSpec {
    /// The braking force of both brakes together at the tyre patches (N).
    pub force: f32,
    /// The share of it the front brake takes.
    pub front_share: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SteeringSpec {
    /// The handlebar's lock to each side (rad).
    pub max_angle: f32,
    /// The stiffness of the handlebar's response to the controller (1/s^2) and its damping (1/s).
    pub response: f32,
    pub damping: f32,
    /// The steepest lean the controller asks for (rad).
    pub max_lean: f32,
    /// How fast the controller pulls the lean to its target (rad/s): the natural frequency of a critically damped response.
    pub lean_response: f32,
}

impl BikeSpec {
    /// A generic 450 cc supermoto-class bike: about 110 kg, a 1.47 m wheelbase, 17 inch wheels, a 5-speed box.
    pub fn example() -> Self {
        let tyre = |peak: f32| {
            let mut a = [0.0; 15];
            a[0] = 1.2;
            a[1] = -10.0;
            a[2] = peak;
            a[3] = 1200.0;
            a[4] = 6.0;
            a[5] = 0.002;
            a[8] = -0.01;
            let mut b = [0.0; 11];
            b[0] = 1.5;
            b[1] = -10.0;
            b[2] = 1250.0;
            b[4] = 200.0;
            b[6] = -0.1;
            b[8] = 0.1;
            Mf94 { a, b, c: [0.0; 18] }
        };
        let wheel = |tyre: Mf94, mass: f32| WheelSpec {
            radius: 0.32,
            width: 0.12,
            toroid_factor: 0.7,
            mass,
            inertia: 0.65,
            tyre,
            rolling: [0.02, 0.0004, 0.000_001],
            lateral_grip_limit: 1.25,
        };
        let damper = DamperSpec {
            slow_bump: 1600.0,
            fast_bump: 1250.0,
            bump_threshold: 0.3,
            slow_rebound: 2400.0,
            fast_rebound: 1650.0,
            rebound_threshold: 0.3,
        };
        Self {
            chassis: ChassisSpec {
                mass: 110.0,
                rider_mass: 0.0,
                half_dimensions: Vec3::new(0.42, 0.64, 1.06),
                tensor_scale: Vec3::new(0.4, 0.45, 0.2),
                cg_height: 0.62,
                front_weight: 0.5,
                wheelbase: 1.47,
                rake: 26.0_f32.to_radians(),
                trail: 0.105,
            },
            front: wheel(tyre(1330.0), 17.0),
            rear: wheel(tyre(1360.0), 19.0),
            front_suspension: SuspensionSpec {
                max_travel: 0.28,
                spring: 8800.0,
                top_out_stiffness: 3800.0,
                damper,
                leverage: Vec::new(),
            },
            rear_suspension: SuspensionSpec {
                max_travel: 0.26,
                spring: 48000.0,
                top_out_stiffness: 17000.0,
                damper: DamperSpec {
                    slow_bump: 7500.0,
                    slow_rebound: 10500.0,
                    bump_threshold: 0.2,
                    rebound_threshold: 0.2,
                    ..damper
                },
                leverage: vec![(0.0, 2.8), (1.0, 1.97)],
            },
            engine: EngineSpec {
                peak_torque: 46.0,
                torque_curve: vec![(2300.0, 0.7), (5000.0, 0.85), (7500.0, 1.0), (9500.0, 0.9), (11000.0, 0.7)],
                friction_torque: 12.0,
                idle_rpm: 2300.0,
                max_rpm: 11000.0,
            },
            transmission: TransmissionSpec {
                gears: vec![2.1, 1.65, 1.35, 1.12, 0.96],
                final_ratio: 7.2,
                efficiency: 0.92,
                up_shift: 0.9,
                down_shift: 0.45,
            },
            brakes: BrakeSpec { force: 1800.0, front_share: 0.65 },
            steering: SteeringSpec {
                max_angle: 0.7,
                response: 600.0,
                damping: 40.0,
                max_lean: 55.0_f32.to_radians(),
                lean_response: 7.0,
            },
            drag: 0.15,
        }
    }

    /// The total mass the rigid body carries (kg).
    pub fn total_mass(&self) -> f32 {
        self.chassis.mass + self.chassis.rider_mass + self.front.mass + self.rear.mass
    }
}
