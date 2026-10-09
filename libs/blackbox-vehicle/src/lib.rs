//! Deterministic, fixed-step vehicle physics.
//!
//! A [`Vehicle`] is a rigid body with four spring/damper corners, a tire model, an engine and drivetrain,
//! steering, aerodynamics and driver-input shaping. Everything is `f32`, pure and free of I/O: the caller
//! fills a [`VehicleSpec`] (plain structs, in the units of the game's attribute data) and calls
//! [`Vehicle::step`] with a fixed `dt` (1/60 s, see [`FIXED_STEP`]), an [`InputState`] and a [`Ground`].
//!
//! Physics space is x right, y up, z forward, metres, kilograms, seconds, radians. Wheel order is
//! 0 front left, 1 front right, 2 rear left, 3 rear right.
//!
//! Modules, one domain each: [`math`], [`rigid_body`] (integration, inertia, drag, sleep, contact
//! impulses), [`engine`] (torque curve, clutch), [`drivetrain`] (gearbox, differentials, the torque
//! loop), [`induction`] and [`nos`], [`brakes`], [`tires`] (slip, load sensitivity, friction ellipse),
//! [`suspension`] (springs, dampers, anti-roll, travel), [`steering`] (speed-sensitive limits,
//! Ackermann), [`aero`], [`input`] (pedals and stick to controls), [`ground`] (the ray-cast trait) and
//! [`vehicle`] (the assembly).
//!
//! ```
//! use blackbox_vehicle::{FIXED_STEP, FlatGround, InputState, Vehicle, VehicleSpec};
//! use glam::Vec3;
//!
//! let ground = FlatGround::new(0.0);
//! let mut car = Vehicle::new(VehicleSpec::example());
//! car.place_on_ground(&ground, 0.0, 0.0, 5.0, 0.0);
//! let input = InputState { throttle: 1.0, ..Default::default() };
//! for _ in 0..600 {
//!     car.step(FIXED_STEP, &input, &ground);
//! }
//! assert!(car.forward_speed() > 15.0);
//! assert!(car.position().distance(Vec3::ZERO) > 20.0);
//! ```

pub mod aero;
pub mod brakes;
pub mod drivetrain;
pub mod engine;
pub mod ground;
pub mod induction;
pub mod input;
pub mod math;
pub mod nos;
pub mod performance;
pub mod rigid_body;
pub mod steering;
pub mod suspension;
pub mod tires;
pub mod vehicle;

/// The fixed simulation step, in seconds.
pub const FIXED_STEP: f32 = 1.0 / 60.0;

pub use ground::{FlatGround, Ground, GroundHit, NoGround, SurfaceGrip};
pub use input::{ControlConfig, InputState};
pub use vehicle::{SKID_RANGE, SMOKE_RANGE, Tunings, Vehicle, VehicleSpec, WheelState};
