//! Data identity for a user-driven original-PC capture; not a tire-force/trajectory acceptance test.

use nfsmw_data::car::physics::{PerformanceCatalog, PerformanceCategory, PhysicsData};

#[test]
#[ignore = "requires the matching PC installation and its 911 Turbo profile"]
fn captured_911turbo_maximum_junkman_profile_matches_physical_inputs() {
    let dir = crate::install().expect("this check requires NFSMW_GAME_DIR");
    let data = PhysicsData::load(&dir).unwrap();
    let catalog = PerformanceCatalog::read(data.database(), "911turbo").unwrap();
    let mut package = catalog.maximum();
    for category in PerformanceCategory::ALL {
        if catalog.junkman_available(category, package) {
            package.junkman |= 1 << category as u8;
        }
    }
    assert_ne!(package.junkman, 0, "the captured setup includes eligible Junkman hardware");
    let physics = data.car_with_performance("911turbo", package).unwrap();
    // Captured from owner-validated body/tire state with all seven tuning sliders neutral.
    assert!((physics.spec.mass - 1590.0).abs() < 0.01, "mass {}", physics.spec.mass);
    assert!((physics.spec.tires.steering - 1.05).abs() < 1e-6, "steering {}", physics.spec.tires.steering);
    for axle in 0..2 {
        let radius = physics.spec.tires.radius(axle);
        assert!((radius - 0.3171).abs() < 1e-6, "axle {axle} radius {radius}");
    }
    println!("911turbo maximum + eligible Junkman: captured mass, steering and both tire radii match");
}
