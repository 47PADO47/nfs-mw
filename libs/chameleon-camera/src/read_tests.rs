//! The reader on a synthetic container: the Porsche 911's numbers written into a block at the offsets of the real
//! layout (`docs/formats/chameleon-camera.md`). No game data.

use crate::genesys::Object;
use crate::params::from_data_axes;
use crate::read::parse_container;

/// `(type name, offset)` of the structures of the container object, as the import table lists them.
const STRUCTS: &[(&str, usize)] = &[
    ("Genesys.Gen.VehicleCameraContainer", 0x0),
    ("Genesys.Gen.CameraGameplayBonnet", 0x20),
    ("Genesys.Gen.CameraGameplayBumper", 0xA0),
    ("Genesys.Gen.CameraGameplayExternal", 0x120),
    ("Genesys.Gen.CameraGameplayExternal.Speed_based_movement", 0x12C),
    ("Genesys.Gen.CameraGameplayExternal.Speed_based_movement.High_Speed_Shake", 0x138),
    ("Genesys.Gen.CameraGameplayExternal.Acceleration_movement", 0x174),
    ("Genesys.Gen.CameraGameplayExternal.Camera_Roll", 0x1B0),
    ("Genesys.Gen.CameraGameplayExternal.Yaw_spring_modification", 0x1DC),
    ("Genesys.Gen.CameraGameplayExternal.Deceleration_Pitch_Change", 0x208),
    ("Genesys.Gen.CameraGameplayExternal.Speed_based_height_change", 0x22C),
    ("Genesys.Gen.CameraGameplayExternalGlobals", 0x2A8),
    ("Genesys.Gen.CameraGameplayExternalGlobals.Impact_shake", 0x2B4),
    ("Genesys.Gen.CameraGameplayExternalGlobals.LensProperties", 0x314),
    ("Genesys.Gen.CameraGameplayGradientAdjustments", 0x46C),
    ("Genesys.Gen.CameraGameplayGradientAdjustments.Params", 0x478),
    ("Genesys.Gen.CameraGameplayGradientAdjustments.Params", 0x490),
    ("Genesys.Gen.CameraRearView", 0x4B0),
    ("Genesys.Gen.CameraRearViewGlobals", 0x4D8),
];

const SIZE: usize = 0x6D0;

fn put(block: &mut [u8], at: usize, values: &[f32]) {
    for (n, value) in values.iter().enumerate() {
        block[at + 4 * n..at + 4 * n + 4].copy_from_slice(&value.to_le_bytes());
    }
}

fn put_u32(block: &mut [u8], at: usize, value: u32) {
    block[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

/// The mounted camera structure at `base` with the bonnet's or the bumper's numbers.
fn put_mounted(block: &mut [u8], base: usize, eye: [f32; 3], target: [f32; 3], fov: [f32; 2], id: u32) {
    put(block, base + 0xC, &[0.5, 0.7, 1.0]);
    put(block, base + 0x18, &eye);
    put(block, base + 0x24, &target);
    put_u32(block, base + 0x30, id);
    put(block, base + 0x34, &[0.3, 0.05, 0.25, 0.15, 0.05, -1.0, 5.0, 0.1, 0.0, 15.0, 15.0, 0.0]);
    put(block, base + 0x64, &[fov[0], fov[1], 3.0]);
}

/// A container with the Porsche 911's values.
pub(crate) fn porsche_block() -> Vec<u8> {
    let mut b = vec![0u8; SIZE];
    put_u32(&mut b, 0xC, 0x108E64);
    put_mounted(&mut b, 0x20, [0.0, 1.31785, 0.207693], [0.0, 1.05, 0.8], [105.0, 95.0], 0x1E041);
    put_mounted(&mut b, 0xA0, [0.0, 0.6, -1.5], [0.0, 0.425, 1.5], [120.0, 95.0], 0x191997);
    let e = 0x120;
    put(&mut b, e + 0x124, &[0.55, 0.09, 1.0]);
    put(&mut b, e + 0x130, &[0.0, 1.2, 0.75]);
    put(&mut b, e + 0x13C, &[0.0, 0.0, 0.75]);
    put(&mut b, e + 0x148, &[0.0, 1.6, -4.6]);
    put(&mut b, e + 0x160, &[95.0, 70.0, 10.0, 4.0, 0.055]);
    put(&mut b, 0x12C + 0x34, &[-1.0, 1.5, 1.7, 0.0]);
    put(&mut b, 0x138 + 0x10, &[1.2, 1.0, 180.0, 0.0]);
    put(&mut b, 0x174 + 0x10, &[-2.0, 0.5, 2.5, 0.0, 50.0, 1.5, -1.2, 2.0, 0.0, 1.75]);
    put(&mut b, 0x1B0 + 0x10, &[2.5, 2.5, 30.0, 4.7, 0.2, 10.0]);
    put(&mut b, 0x1DC + 0x10, &[0.01, 10.0, 100.0, 0.05, 10.0, 0.0]);
    put(&mut b, 0x208 + 0x10, &[12.0, 4.5, 2.0, 2.25]);
    put(&mut b, 0x22C + 0x10, &[1.59]);
    put(&mut b, 0x2B4 + 0xC, &[10.0, 10.0, 50.0, 1.0, 1.0, 1.0, 0.5, 0.5, 0.5]);
    put(&mut b, 0x2B4 + 0x34, &[0.75, 0.165, 1.0, 60.0, 2.0, 30.0, 4.0, 5.0, 1.0]);
    put(&mut b, 0x314 + 0x10, &[90.0, 1.4]);
    put(&mut b, 0x478 + 0x10, &[0.5]);
    put(&mut b, 0x490 + 0x10, &[0.5]);
    put(&mut b, 0x4B0 + 0xC, &[0.0, 0.9, 0.0]);
    put(&mut b, 0x4D8 + 0x10, &[3.33333, 400.0, 60.0]);
    b
}

fn object_of(block: Vec<u8>) -> Object {
    Object::new(block, STRUCTS.iter().map(|(name, at)| ((*name).to_owned(), *at)).collect())
}

/// The Porsche 911's cameras, decoded from the synthetic container (for the model tests).
pub(crate) fn porsche_cameras() -> crate::params::CarCameras {
    parse_container(&object_of(porsche_block())).unwrap()
}

#[test]
fn axes_map_from_the_data_to_the_host_space() {
    // data: x left, y up, z forward -> host: x forward, y left, z up
    assert_eq!(from_data_axes([1.0, 0.0, 0.0]), [0.0, 1.0, 0.0], "data left is host y");
    assert_eq!(from_data_axes([0.0, 1.0, 0.0]), [0.0, 0.0, 1.0], "data up is host z");
    assert_eq!(from_data_axes([0.0, 0.0, 1.0]), [1.0, 0.0, 0.0], "data forward is host x");
}

#[test]
fn reads_the_chase_camera_in_host_axes() {
    let cameras = parse_container(&object_of(porsche_block())).unwrap();
    let ext = &cameras.external;
    assert_eq!(ext.position, [-4.6, 0.0, 1.6], "4.6 m behind and 1.6 m up");
    assert_eq!(ext.focus, [0.75, 0.0, 1.2]);
    assert_eq!(ext.ground_pivot, [0.75, 0.0, 0.0]);
    assert_eq!(ext.response, [1.0, 0.55, 0.09]);
    assert_eq!(ext.fov_deg, [95.0, 70.0]);
    assert_eq!(ext.fov_range_deg(), [70.0, 95.0]);
    assert_eq!((ext.param_168, ext.param_16c, ext.lag_s), (10.0, 4.0, 0.055));
    assert_eq!(ext.height_at_speed, 1.59);
    assert_eq!(cameras.object_key, 0x108E64);
}

#[test]
fn reads_the_chase_sub_structures() {
    let ext = parse_container(&object_of(porsche_block())).unwrap().external;
    let speed = &ext.speed_movement;
    assert_eq!((speed.back_at_speed, speed.height_a, speed.height_b, speed.param_40), (-1.0, 1.5, 1.7, 0.0));
    assert_eq!(
        (speed.shake.amplitude, speed.shake.rate, speed.shake.full_speed_mph, speed.shake.start_speed_mph),
        (1.2, 1.0, 180.0, 0.0)
    );
    let accel = &ext.accel_movement;
    assert_eq!((accel.pitch_deg, accel.smoothing_s, accel.full_accel), (-2.0, 0.5, 50.0));
    assert_eq!((accel.back_m, accel.forward_m, accel.param_34), (1.5, -1.2, 1.75));
    let roll = &ext.roll;
    assert_eq!((roll.full_speed_mph, roll.max_deg, roll.smoothing_s), (30.0, 4.7, 0.2));
    let yaw = &ext.yaw_spring;
    assert_eq!(
        (yaw.speed_min, yaw.rate_low, yaw.speed_max, yaw.lead_s, yaw.rate_high),
        (0.01, 10.0, 100.0, 0.05, 10.0)
    );
    let decel = &ext.decel_pitch;
    assert_eq!((decel.rate_deg_s, decel.max_deg, decel.response), (12.0, 4.5, 2.25));
}

#[test]
fn reads_the_mounted_cameras_and_the_globals() {
    let cameras = parse_container(&object_of(porsche_block())).unwrap();
    assert_eq!(cameras.bonnet.eye, [0.207693, 0.0, 1.31785]);
    assert_eq!(cameras.bonnet.target, [0.8, 0.0, 1.05]);
    assert_eq!(cameras.bonnet.fov_range_deg(), [95.0, 105.0]);
    assert_eq!(cameras.bonnet.look_limit_deg, [15.0, 15.0]);
    assert_eq!((cameras.bonnet.smoothing_s, cameras.bonnet.sway_m, cameras.bonnet.wind_shake), (0.3, 0.05, 0.25));
    assert_eq!(cameras.bonnet.object_key, 0x1E041);
    assert_eq!(cameras.bumper.eye, [-1.5, 0.0, 0.6]);
    assert_eq!(cameras.bumper.target, [1.5, 0.0, 0.425]);
    assert_eq!(cameras.bumper.fov_range_deg(), [95.0, 120.0]);
    let impact = &cameras.globals.impact_shake;
    assert_eq!(impact.frequency, [10.0, 10.0, 50.0], "per rotation axis, in the file's order");
    assert_eq!(impact.amplitude_deg, [1.0; 3]);
    assert_eq!(impact.decay_s, [0.5; 3]);
    assert_eq!(impact.scalars[0], 0.75);
    assert_eq!((cameras.globals.lens.fov_deg, cameras.globals.lens.scale), (90.0, 1.4));
    assert_eq!(cameras.gradient.pitch_follow, [0.5, 0.5]);
    assert_eq!(cameras.rear_view.offset, [0.0, 0.0, 0.9]);
    assert_eq!((cameras.rear_view.width_px, cameras.rear_view.fov_deg), (400.0, 60.0));
    assert!((cameras.rear_view.aspect - 3.3333).abs() < 1e-3);
}

#[test]
fn a_container_missing_a_structure_is_refused() {
    let without_roll: Vec<(String, usize)> = STRUCTS
        .iter()
        .filter(|(name, _)| !name.ends_with("Camera_Roll"))
        .map(|(name, at)| ((*name).to_owned(), *at))
        .collect();
    let error = parse_container(&Object::new(porsche_block(), without_roll)).unwrap_err();
    assert!(error.to_string().contains("Camera_Roll"), "{error}");
}

#[test]
fn a_truncated_object_is_refused_not_panicked_on() {
    let mut block = porsche_block();
    block.truncate(0x300);
    assert!(parse_container(&object_of(block)).is_err());
}

#[test]
fn implausible_numbers_are_refused() {
    let mut block = porsche_block();
    put(&mut block, 0x120 + 0x148, &[0.0, 1.6, 4.6]); // the camera in front of the car
    let error = parse_container(&object_of(block)).unwrap_err();
    assert!(error.to_string().contains("implausible"), "{error}");
    let mut block = porsche_block();
    put(&mut block, 0x120 + 0x160, &[f32::NAN, 70.0]);
    assert!(parse_container(&object_of(block)).is_err(), "a NaN field of view");
}

#[test]
fn only_two_gradient_params_are_accepted() {
    let one: Vec<(String, usize)> =
        STRUCTS.iter().filter(|(_, at)| *at != 0x490).map(|(name, at)| ((*name).to_owned(), *at)).collect();
    assert!(parse_container(&Object::new(porsche_block(), one)).is_err());
}

#[test]
fn moving_the_origin_forward_shortens_the_chase_and_bonnet_offsets_only() {
    let before = porsche_cameras();
    let mut after = before.clone();
    after.move_origin_forward(0.2);
    let moved = |a: [f32; 3], b: [f32; 3]| (a[0] - b[0] - 0.2).abs() < 1e-6 && a[1..] == b[1..];
    assert!(moved(before.external.position, after.external.position));
    assert!(moved(before.external.focus, after.external.focus));
    assert!(moved(before.external.ground_pivot, after.external.ground_pivot));
    assert!(moved(before.bonnet.eye, after.bonnet.eye));
    assert!(moved(before.bonnet.target, after.bonnet.target));
    assert_eq!(before.bumper, after.bumper, "the bumper camera is anchored by the host");
    assert_eq!(before.rear_view, after.rear_view);
}
