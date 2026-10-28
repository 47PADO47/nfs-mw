//! The unit conversions of the physics importer: the units of the RIDE 6 assets to the SI units of the `[bike]` table.
//!
//! The simulation of RIDE 6 runs in Unreal units, and its assets are written in a mix of them and of SI
//! (`docs/formats/ride6-physics-vehicle.md`, "Unit convention of the simulation"). Every number the importer writes passes
//! through one of these functions or is copied unchanged because the asset is already in the manifest's unit.

use std::f64::consts::PI;

/// Chassis geometry (centre of gravity, wheelbase, trail, rider offsets) and the tyre deformation limit are centimetres.
pub fn metres(centimetres: f64) -> f64 {
    centimetres / 100.0
}

/// Speeds in tuning ranges are km/h (the binary turns cm/s into km/h with 0.036), the manifest wants m/s.
pub fn metres_per_second(km_per_hour: f64) -> f64 {
    km_per_hour / 3.6
}

/// Angles of the controllers are degrees, the manifest wants radians.
pub fn radians(degrees: f64) -> f64 {
    degrees * (PI / 180.0)
}

/// The slipper clutch's torque range is in Unreal torque units (1 N m is 10000 of them).
pub fn newton_metres(unreal_torque: f64) -> f64 {
    unreal_torque / 10_000.0
}

/// A point or vector of RIDE 6 (x forward, y right, z up) as a point of the physics space of the manifest (x right, y up,
/// z forward).
pub fn to_physics_space(ride6: [f64; 3]) -> [f64; 3] {
    [ride6[1], ride6[2], ride6[0]]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_conversions_are_the_exact_factors() {
        assert_eq!(metres(145.0), 1.45);
        assert!((metres_per_second(36.0) - 10.0).abs() < 1e-12);
        assert!((radians(180.0) - PI).abs() < 1e-12);
        assert_eq!(newton_metres(250_000.0), 25.0);
        assert_eq!(to_physics_space([1.0, 2.0, 3.0]), [2.0, 3.0, 1.0]);
    }
}
