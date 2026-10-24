use super::*;

fn close(actual: [f32; 3], expected: [f32; 3]) {
    for (a, b) in actual.into_iter().zip(expected) {
        assert!((a - b).abs() < 1e-6, "{actual:?} != {expected:?}");
    }
}

#[test]
fn native_weights_preserve_stock_and_isolate_brakes_from_power_axes() {
    let endpoints = RatingEndpoints { stock: [0.2; 3], maximum: [0.8; 3], max_levels: [4; 7] };
    close(endpoints.rating(PerformancePackage::default()), endpoints.stock);
    close(endpoints.rating(PerformancePackage { levels: [4; 7], junkman: 0 }), endpoints.maximum);
    let mut brakes = PerformancePackage::default();
    brakes.levels[1] = 4;
    close(endpoints.rating(brakes), [0.2, 0.3, 0.2]);
    let mut tires = PerformancePackage::default();
    tires.levels[0] = 4;
    close(endpoints.rating(tires), [0.26, 0.5, 0.2 + 0.6 * 0.25 / 4.95]);
}

#[test]
fn junkman_adjusts_the_endpoint_and_the_blend_position() {
    let endpoints = RatingEndpoints { stock: [0.2; 3], maximum: [0.8; 3], max_levels: [4; 7] };
    // All seven Junkman bits: upper endpoint becomes one; base becomes .464.
    close(endpoints.rating(PerformancePackage { levels: [0; 7], junkman: 0x7f }), [0.464; 3]);
    close(endpoints.rating(PerformancePackage { levels: [2; 7], junkman: 0x7f }), [0.82044; 3]);
    close(endpoints.rating(PerformancePackage { levels: [4; 7], junkman: 0x7f }), [1.0; 3]);
    let no_upgrades = RatingEndpoints { max_levels: [0; 7], ..endpoints };
    close(no_upgrades.rating(PerformancePackage { levels: [4; 7], junkman: 0 }), endpoints.stock);
}

#[test]
fn population_normalization_includes_maximum_builds_and_inverts_acceleration() {
    let entry = |key, speed, grip, time, upgraded| Entry {
        key,
        stock: Raw { top_speed: speed, handling: grip, time_to_100: time },
        maximum: upgraded,
        max_levels: [4; 7],
    };
    let entries = [
        entry(1, 40.0, 20.0, 30.0, Some(Raw { top_speed: 80.0, handling: 100.0, time_to_100: 10.0 })),
        entry(2, 60.0, 60.0, 20.0, None),
    ];
    let book = RatingBook::from_entries(&entries);
    close(book.cars[&1].stock, [0.0; 3]);
    close(book.cars[&1].maximum, [1.0; 3]);
    close(book.cars[&2].stock, [0.5; 3]);
    close(book.cars[&2].maximum, [0.5; 3]);
    let one = RatingBook::from_entries(&entries[1..]);
    close(one.cars[&2].stock, [0.0, 0.0, 1.0]);
}

#[test]
#[ignore = "requires an installed game"]
fn installed_garage_rating_endpoints_cover_the_original_eligible_population() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let db = shared::attributes(&dir).unwrap();
    let book = RatingBook::load(&dir).unwrap();
    let mut rated = 0;
    for vehicle in db.collections_of("pvehicle") {
        let Some(name) = vehicle.name() else { continue };
        let Some(endpoints) = book.for_car(name) else { continue };
        let maximum = PerformanceCatalog::read(&db, name).unwrap().maximum();
        let bounds = CarBounds { half_dimensions: Vec3::ONE, pivot: Vec3::ZERO };
        let mut physics = car_physics(&db, name, bounds).unwrap();
        assert!(estimate::estimate(&physics.spec).is_some(), "{name}: stock estimate");
        super::super::performance::apply(&db, name, maximum, &mut physics).unwrap();
        assert!(estimate::estimate(&physics.spec).is_some(), "{name}: maximum estimate");
        assert_eq!(endpoints.max_levels, maximum.levels, "{name}");
        for values in [
            endpoints.stock,
            endpoints.maximum,
            endpoints.rating(maximum),
            endpoints.rating(PerformancePackage { junkman: 0x7f, ..maximum }),
        ] {
            assert!(values.into_iter().all(|v| v.is_finite() && (0.0..=1.0).contains(&v)), "{name}: {values:?}");
        }
        close(endpoints.rating(PerformancePackage::default()), endpoints.stock);
        rated += 1;
    }
    assert_eq!(rated, book.len());
    assert!(rated >= 40, "only {rated} eligible rating records");
    assert!(book.for_car("cobaltss").is_some());
    assert!(book.for_car("corvettec6r").is_some());
    for car in ["cobaltss", "bmwm3gtr", "corvettec6r"] {
        println!("{car}: {:?}", book.for_car(car).unwrap());
    }
    println!("{rated} installed original rating records including stock and maximum ordinary endpoints");
}
