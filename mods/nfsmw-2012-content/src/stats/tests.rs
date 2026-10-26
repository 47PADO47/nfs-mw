use blackbox_modding::content::{CarManifest, ContentError, Drive, parse_car};
use chameleon_vehicle::{TorqueKnot, VehicleStats, WheelStats};

use super::*;

/// The Porsche 911 Carrera S (2012) with the values its bundle holds (`VEH_1085007_HI.BNDL`).
fn porsche() -> VehicleStats {
    let wheel = |x: f32, z: f32, radius: f32, half_width: f32, mass_kg: f32| WheelStats {
        position: [x, if z > 0.0 { 0.338 } else { 0.34 }, z],
        radius,
        half_width,
        mass_kg,
    };
    let knot = |rpm, torque_nm, slope: f32| TorqueKnot { rpm, torque_nm, in_slope: slope, out_slope: slope };
    VehicleStats {
        body_mass_kg: 1295.0,
        centre_of_mass: [0.0, 0.45, -0.27],
        box_half_extents: [0.9115, 0.5473, 2.2115],
        box_centre: [0.0019, 0.7187, -0.0228],
        axle_z: [1.245, -1.205],
        wheels: [
            wheel(0.769, 1.24, 0.34, 0.123, 24.0),
            wheel(-0.769, 1.24, 0.34, 0.123, 24.0),
            wheel(0.758, -1.21, 0.343, 0.148, 27.0),
            wheel(-0.758, -1.21, 0.343, 0.148, 27.0),
        ],
        torque_curve: vec![
            knot(1000.0, 360.0, 150.0 / 4600.0),
            knot(5600.0, 440.0, 0.0),
            knot(7800.0, 390.0, -125.0 / 2750.0),
        ],
        reverse_ratio: 2.68,
        gear_ratios: vec![3.91, 2.29, 1.55, 1.3, 1.08, 0.88, 0.71],
        final_drive: 3.44,
        differentials: [0.0, 0.0, 0.0, 0.75],
        brake_torque_nm: [1300.0, 1350.0],
        tire_grip: [1.5, 1.4],
        top_speed_mph: 189.0,
        boosted_top_speed_mph: 229.0,
        nitrous_strength: 1.9,
    }
}

/// Parse a table the way the game does. A manifest still names a clone while the clone key exists.
fn parse(table: &str) -> Result<CarManifest, ContentError> {
    match parse_car("test", table) {
        Err(ContentError::Parse(message)) if message.contains("missing field `clone`") => {
            parse_car("test", &format!("clone = \"X\"\n{table}"))
        }
        other => other,
    }
}

#[test]
fn the_porsche_is_a_1397_kg_rear_drive_car_with_its_weight_behind_the_middle() {
    let stats = ImportedStats::of(&porsche());
    assert_eq!(stats.mass_kg, 1397.0, "the body and four wheels");
    assert!((stats.front_weight_pct - 38.8).abs() < 0.2, "{}", stats.front_weight_pct);
    assert!((stats.cg_height_m - 0.442).abs() < 0.002, "{}", stats.cg_height_m);
    assert!((stats.wheelbase_m - 2.45).abs() < 1e-4);
    assert!((stats.track_m[0] - 1.538).abs() < 1e-3 && (stats.track_m[1] - 1.516).abs() < 1e-3);
    assert_eq!((stats.tire_radius_m[0], stats.tire_radius_m[1]), (0.34, 0.343));
    assert!((stats.tire_width_m[0] - 0.246).abs() < 1e-4 && (stats.tire_width_m[1] - 0.296).abs() < 1e-4);
    assert!((stats.ride_height_m - 0.1714).abs() < 1e-3);
    assert_eq!((stats.drive, stats.front_torque_share, stats.diff_lock), ("rwd", 0.0, [0.0, 0.75]));
    assert!((stats.top_speed_kph - 304.2).abs() < 0.1);
    assert!((stats.nitrous_boost - 0.9).abs() < 1e-6);
}

#[test]
fn the_curve_keeps_its_knots_and_stays_inside_the_manifests_sixteen_points() {
    let stats = ImportedStats::of(&porsche());
    let curve = &stats.torque_curve;
    assert_eq!(curve.len(), MAX_POINTS);
    for knot in [[1000.0, 360.0], [5600.0, 440.0], [7800.0, 390.0]] {
        assert!(curve.contains(&knot), "{knot:?} in {curve:?}");
    }
    assert!(curve.windows(2).all(|pair| pair[1][0] > pair[0][0]));
    assert_eq!((stats.idle_rpm, stats.redline_rpm, stats.peak_torque_nm()), (1000.0, 7800.0, 440.0));
}

#[test]
fn the_drive_follows_the_share_of_torque_the_front_axle_gets() {
    assert_eq!(drive_of(0.0), ("rwd", 0.0));
    assert_eq!(drive_of(1.0), ("fwd", 1.0));
    assert_eq!(drive_of(0.3), ("awd", 0.3));
}

#[test]
fn an_electric_motor_that_starts_at_zero_rpm_idles_at_the_floor() {
    let mut car = porsche();
    car.torque_curve[0].rpm = 0.0;
    let stats = ImportedStats::of(&car);
    assert_eq!(stats.idle_rpm, IDLE_FLOOR_RPM);
    assert_eq!(stats.torque_curve[0][0], 0.0);
}

#[test]
fn the_table_is_a_manifest_the_game_accepts_with_the_porsches_numbers() {
    let manifest = parse(&ImportedStats::of(&porsche()).manifest_table()).unwrap();
    let stats = manifest.stats;
    assert_eq!(stats.mass_kg, 1397.0);
    assert_eq!(stats.gear_ratios, vec![3.91, 2.29, 1.55, 1.3, 1.08, 0.88, 0.71]);
    assert_eq!((stats.final_drive, stats.reverse_ratio, stats.drive), (3.44, 2.68, Drive::Rear));
    assert_eq!((stats.tire_grip, stats.brake_torque_nm), ([1.5, 1.4], [1300.0, 1350.0]));
    assert_eq!(stats.torque_curve.len(), MAX_POINTS);
    assert!(stats.nitrous && (stats.nitrous_boost - 0.9).abs() < 1e-6);
    assert!((stats.top_speed_kph.unwrap() - 304.2).abs() < 0.1);
}

#[test]
fn a_car_without_nitrous_writes_none() {
    let mut stats = ImportedStats::of(&porsche());
    stats.nitrous_boost = 0.0;
    let manifest = parse(&stats.manifest_table()).unwrap();
    assert!(!manifest.stats.nitrous);
}

#[test]
fn the_multipliers_scale_and_stay_inside_the_games_ranges() {
    let mut stats = ImportedStats::of(&porsche());
    let peak = stats.peak_torque_nm();
    stats.scale_torque(1.2);
    assert!((stats.peak_torque_nm() - peak * 1.2).abs() < 0.2);
    stats.scale_mass(0.9);
    assert!((stats.mass_kg - 1397.0 * 0.9).abs() < 0.1);
    stats.scale_grip(100.0);
    assert_eq!(stats.tire_grip, [GRIP.1, GRIP.1]);
    stats.scale_brakes(0.0);
    assert_eq!(stats.brake_torque_nm, [BRAKE.0, BRAKE.0]);
    assert!(parse(&stats.manifest_table()).is_ok());
}

#[test]
fn numbers_are_written_with_a_decimal_point_and_without_noise() {
    assert_eq!(number(1295.0), "1295.0");
    assert_eq!(number(0.343), "0.343");
    assert_eq!(number(304.1), "304.1");
}

/// Every car of the real install converts to a manifest the game accepts (set `CHAMELEON_GAME_DIR`).
#[test]
#[ignore = "reads the 2012 install (CHAMELEON_GAME_DIR)"]
fn every_car_of_the_install_makes_a_valid_stats_table() {
    let Some(dir) = std::env::var_os("CHAMELEON_GAME_DIR") else { return };
    let mut count = 0;
    for entry in std::fs::read_dir(std::path::Path::new(&dir).join("VEHICLES")).unwrap().flatten() {
        let file = entry.file_name().to_string_lossy().to_uppercase();
        if !(file.starts_with("VEH_") && file.ends_with("_HI.BNDL")) {
            continue;
        }
        let data = std::fs::read(entry.path()).unwrap();
        let car = chameleon_vehicle::read_stats(&chameleon_bundle::Bundle::parse(&data).unwrap()).unwrap();
        let table = ImportedStats::of(&car).manifest_table();
        let manifest = parse(&table).unwrap_or_else(|error| panic!("{file}: {error}\n{table}"));
        assert!(manifest.stats.mass_kg > 400.0 && manifest.stats.top_speed_kph.is_some(), "{file}");
        count += 1;
    }
    assert_eq!(count, 93);
}
