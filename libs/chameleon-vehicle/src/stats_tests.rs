//! The stats reader: the torque spline on knots written out here, and the real values of the 2012 install (set
//! `CHAMELEON_GAME_DIR` to the install folder and run with `--ignored`).

use std::path::Path;

use chameleon_bundle::Bundle;

use crate::read_stats;
use crate::stats::{TorqueKnot, VehicleStats, WheelStats};

const WHEEL: WheelStats = WheelStats { position: [0.7, 0.34, 1.2], radius: 0.34, half_width: 0.12, mass_kg: 24.0 };

fn stats_with(curve: Vec<TorqueKnot>) -> VehicleStats {
    VehicleStats {
        body_mass_kg: 1300.0,
        centre_of_mass: [0.0, 0.45, -0.27],
        box_half_extents: [0.9, 0.55, 2.2],
        box_centre: [0.0, 0.7, 0.0],
        axle_z: [1.2, -1.2],
        wheels: [WHEEL; 4],
        torque_curve: curve,
        reverse_ratio: 2.7,
        gear_ratios: vec![3.9, 2.3, 1.5],
        final_drive: 3.4,
        differentials: [0.0, 0.0, 0.0, 0.75],
        brake_torque_nm: [1300.0, 1350.0],
        tire_grip: [1.5, 1.4],
        top_speed_mph: 189.0,
        boosted_top_speed_mph: 229.0,
        nitrous_strength: 1.9,
    }
}

fn porsche_curve() -> Vec<TorqueKnot> {
    vec![
        TorqueKnot { rpm: 1000.0, torque_nm: 360.0, out_slope: 150.0 / 4600.0, in_slope: 150.0 / 4600.0 },
        TorqueKnot { rpm: 5600.0, torque_nm: 440.0, out_slope: 0.0, in_slope: 0.0 },
        TorqueKnot { rpm: 7800.0, torque_nm: 390.0, out_slope: -125.0 / 2750.0, in_slope: -125.0 / 2750.0 },
    ]
}

#[test]
fn the_spline_passes_through_its_knots_and_is_flat_outside() {
    let stats = stats_with(porsche_curve());
    assert_eq!(stats.torque_at(1000.0), 360.0);
    assert!((stats.torque_at(5600.0) - 440.0).abs() < 1e-3);
    assert!((stats.torque_at(7800.0) - 390.0).abs() < 1e-3);
    assert_eq!(stats.torque_at(200.0), 360.0);
    assert_eq!(stats.torque_at(9000.0), 390.0);
    assert_eq!(stats.lowest_rpm(), 1000.0);
    assert_eq!(stats.redline_rpm(), 7800.0);
}

#[test]
fn the_spline_between_knots_follows_the_hermite_basis() {
    // Half way along the first segment: 0.5 * 360 + 0.125 * (slope * width = 150) + 0.5 * 440 + 0.
    let stats = stats_with(porsche_curve());
    assert!((stats.torque_at(3300.0) - 418.75).abs() < 1e-2, "{}", stats.torque_at(3300.0));
}

#[test]
fn a_curve_without_knots_makes_no_torque() {
    assert_eq!(stats_with(Vec::new()).torque_at(3000.0), 0.0);
}

fn install() -> Option<std::path::PathBuf> {
    std::env::var_os("CHAMELEON_GAME_DIR").map(Into::into)
}

fn read(dir: &Path, file: &str) -> VehicleStats {
    let data = std::fs::read(dir.join("VEHICLES").join(file)).unwrap();
    read_stats(&Bundle::parse(&data).unwrap()).unwrap_or_else(|error| panic!("{file}: {error}"))
}

#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
fn the_porsche_911_carrera_s_has_its_known_stats() {
    let Some(dir) = install() else { return };
    let stats = read(&dir, "VEH_1085007_HI.BNDL");
    assert_eq!(stats.body_mass_kg, 1295.0);
    assert_eq!(stats.gear_ratios, vec![3.91, 2.29, 1.55, 1.3, 1.08, 0.88, 0.71]);
    assert_eq!(stats.reverse_ratio, 2.68);
    assert_eq!(stats.final_drive, 3.44);
    let knots: Vec<(f32, f32)> = stats.torque_curve.iter().map(|knot| (knot.rpm, knot.torque_nm)).collect();
    assert_eq!(knots, vec![(1000.0, 360.0), (5600.0, 440.0), (7800.0, 390.0)]);
    assert_eq!(stats.brake_torque_nm, [1300.0, 1350.0]);
    assert_eq!(stats.differentials, [0.0, 0.0, 0.0, 0.75]);
    assert_eq!(stats.top_speed_mph, 189.0);
    assert_eq!(stats.boosted_top_speed_mph, 229.0);
    assert!((stats.nitrous_strength - 1.9).abs() < 1e-6);
    assert!((stats.axle_z[0] - 1.245).abs() < 1e-4 && (stats.axle_z[1] + 1.205).abs() < 1e-4);
    assert!((stats.centre_of_mass[1] - 0.45).abs() < 1e-4);
    assert!(stats.wheels[0].position[0] > 0.0 && stats.wheels[1].position[0] < 0.0);
    assert!(stats.wheels[0].position[2] > stats.wheels[2].position[2]);
    assert!((stats.wheels[0].radius - 0.34).abs() < 1e-4);
}

#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
fn every_car_of_the_install_has_stats() {
    let Some(dir) = install() else { return };
    let mut read_count = 0;
    for entry in std::fs::read_dir(dir.join("VEHICLES")).unwrap().flatten() {
        let file = entry.file_name().to_string_lossy().to_uppercase();
        if !(file.starts_with("VEH_") && file.ends_with("_HI.BNDL")) {
            continue;
        }
        let stats = read(&dir, &file);
        assert!(stats.redline_rpm() > stats.lowest_rpm(), "{file}");
        assert!((1.0..=6.0).contains(&(stats.axle_z[0] - stats.axle_z[1])), "{file}: wheelbase");
        assert!(stats.torque_at(0.5 * (stats.lowest_rpm() + stats.redline_rpm())) > 50.0, "{file}: torque");
        assert!(stats.top_speed_mph > 50.0 && stats.boosted_top_speed_mph > stats.top_speed_mph, "{file}: top speed");
        read_count += 1;
    }
    assert_eq!(read_count, 93);
}
