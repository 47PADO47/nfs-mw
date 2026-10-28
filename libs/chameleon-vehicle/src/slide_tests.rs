//! The sliding numbers: a hand-built handling object, and the real values of the 2012 install (set `CHAMELEON_GAME_DIR` to
//! the install folder and run with `--ignored`).

use std::path::Path;

use chameleon_bundle::Bundle;

use super::*;
use crate::genesys::Object;

/// An object with the three structures at 0x00, 0x40 and 0x80, each filled from the given floats at its fields.
fn object(fields: &[(usize, usize, f32)]) -> Object {
    let mut main = vec![0_u8; 0x100];
    for &(base, offset, value) in fields {
        main[base + offset..base + offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    let structs = vec![
        ("Genesys.Gen.DriftBehaviour.Other".to_owned(), 0x00),
        ("Genesys.Gen.DriftBehaviour.Yaw_torque".to_owned(), 0x40),
        ("Genesys.Gen.HandbrakeAbility".to_owned(), 0x80),
    ];
    Object { main, structs }
}

fn fixture() -> Object {
    object(&[
        (0x00, 0x18, 40.0),
        (0x40, 0x24, 42_500.0),
        (0x40, 0x30, 18_125.0),
        (0x40, 0x34, 10_625.0),
        (0x40, 0x38, 1_875.0),
        (0x80, 0x10, 1_200.0),
        (0x80, 0x14, 350.0),
    ])
}

#[test]
fn the_fields_are_read_from_their_structures() {
    let handling = handling_of(&fixture(), DriftPhysics::TYPICAL).unwrap();
    assert_eq!(handling.max_angle, 40.0);
    assert_eq!(handling.yaw_torque_nm, 42_500.0);
    assert_eq!(handling.yaw_torque_steps, [18_125.0, 10_625.0, 1_875.0]);
    assert_eq!((handling.handbrake_torque_nm, handling.handbrake_second_nm), (1_200.0, 350.0));
    assert_eq!(handling.physics, DriftPhysics::TYPICAL);
}

#[test]
fn numbers_that_cannot_be_real_are_refused() {
    let negative = object(&[(0x40, 0x24, -5.0)]);
    assert!(handling_of(&negative, DriftPhysics::TYPICAL).is_err());
    let wide = object(&[(0x00, 0x18, 120.0)]);
    assert!(handling_of(&wide, DriftPhysics::TYPICAL).is_err());
    let not_a_number = object(&[(0x80, 0x10, f32::NAN)]);
    assert!(handling_of(&not_a_number, DriftPhysics::TYPICAL).is_err());
}

fn install() -> Option<std::path::PathBuf> {
    std::env::var_os("CHAMELEON_GAME_DIR").map(Into::into)
}

fn read(dir: &Path, file: &str) -> DriftHandling {
    let data = std::fs::read(dir.join("VEHICLES").join(file)).unwrap();
    read_drift_handling(&Bundle::parse(&data).unwrap()).unwrap_or_else(|error| panic!("{file}: {error}"))
}

#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
fn the_porsches_have_their_known_numbers() {
    let Some(dir) = install() else { return };
    let carrera = read(&dir, "VEH_1085007_HI.BNDL");
    assert_eq!((carrera.max_angle, carrera.yaw_torque_nm), (40.0, 42_500.0));
    assert_eq!((carrera.handbrake_torque_nm, carrera.handbrake_second_nm), (1_200.0, 350.0));
    assert_eq!(carrera.yaw_torque_steps, [18_125.0, 10_625.0, 1_875.0]);
    let spyder = read(&dir, "VEH_1085830_HI.BNDL");
    assert_eq!((spyder.max_angle, spyder.yaw_torque_nm), (45.0, 37_500.0));
    assert_eq!((spyder.handbrake_torque_nm, spyder.handbrake_second_nm), (1_400.0, 650.0));
    assert_eq!(spyder.physics.trigger_angle, 8.5);
}

#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
fn every_car_reads_inside_the_ranges_the_doc_gives() {
    let Some(dir) = install() else { return };
    let mut cars = 0;
    for entry in std::fs::read_dir(dir.join("VEHICLES")).unwrap().flatten() {
        let file = entry.file_name().to_string_lossy().to_uppercase();
        if !(file.starts_with("VEH_") && file.ends_with("_HI.BNDL")) {
            continue;
        }
        let handling = read(&dir, &file);
        assert!((0.0..=55.0).contains(&handling.max_angle), "{file}: {handling:?}");
        assert!((6_000.0..=60_000.0).contains(&handling.yaw_torque_nm), "{file}: {handling:?}");
        assert!((300.0..=1_800.0).contains(&handling.handbrake_torque_nm), "{file}: {handling:?}");
        assert!((200.0..=750.0).contains(&handling.handbrake_second_nm), "{file}: {handling:?}");
        cars += 1;
    }
    assert_eq!(cars, 93);
}
