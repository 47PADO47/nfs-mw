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
//! Modules: [`math`], [`rigid_body`], [`engine`], [`drivetrain`], [`induction`], [`nos`], [`brakes`],
//! [`tires`], [`suspension`], [`steering`], [`aero`], [`input`], [`ground`], [`vehicle`].

pub mod aero;
pub mod brakes;
pub mod drivetrain;
pub mod engine;
pub mod ground;
pub mod induction;
pub mod input;
pub mod math;
pub mod nos;
pub mod rigid_body;
pub mod steering;
pub mod suspension;
pub mod tires;
pub mod vehicle;

/// The fixed simulation step, in seconds.
pub const FIXED_STEP: f32 = 1.0 / 60.0;

pub use ground::{FlatGround, Ground, GroundHit, NoGround, SurfaceGrip};
pub use input::{ControlConfig, InputState};
pub use vehicle::{Tunings, Vehicle, VehicleSpec, WheelState};
