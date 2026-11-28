use glam::Vec3;

use super::*;
use crate::viewer::camera::direction;

fn lens_at(eye: Vec3, heading: f32, pitch: f32, fov: f32) -> Lens {
    let forward = direction(heading.to_radians(), pitch.to_radians());
    Lens::look_at(eye, eye + forward, fov, 0.1, 16.0 / 9.0)
}

fn words(line: &str) -> Vec<&str> {
    line.split_whitespace().collect()
}

#[test]
fn a_lens_reads_as_the_cam_command() {
    let pose = Pose::from_lens(&lens_at(Vec3::new(1214.0, 547.0, 98.06), 45.0, -15.0, 70.0));
    assert_eq!(pose.line(), "cam 1214.0 547.0 98.1 45.0 -15.0 70.0");
}

#[test]
fn headings_read_from_0_to_360_degrees() {
    let pose = Pose::from_lens(&lens_at(Vec3::ZERO, -90.0, 0.0, 60.0));
    assert_eq!(pose.line(), "cam 0.0 0.0 0.0 270.0 0.0 60.0");
    let west = Pose::from_lens(&lens_at(Vec3::new(-0.01, 2.0, -3.0), 180.0, 10.0, 55.5));
    assert_eq!(west.line(), "cam 0.0 2.0 -3.0 180.0 10.0 55.5");
}

#[test]
fn the_line_round_trips_through_the_command() {
    for (eye, heading, pitch, fov) in [
        (Vec3::new(1214.0, 547.0, 98.1), 45.0, -15.0, 70.0),
        (Vec3::new(-3021.4, 1888.8, 12.3), 300.5, 32.2, 48.0),
        (Vec3::new(0.0, -0.5, 250.0), 0.0, -88.0, 90.0),
    ] {
        let line = Pose::from_lens(&lens_at(eye, heading, pitch, fov)).line();
        let parsed = Pose::parse(&words(&line)).expect("the line parses");
        assert!(parsed.eye.distance(eye) < 0.08, "{line}: {:?}", parsed.eye);
        assert!((parsed.heading - heading).abs() < 0.06, "{line}");
        assert!((parsed.pitch - pitch).abs() < 0.06, "{line}");
        assert_eq!(parsed.fov, Some(fov));
        assert_eq!(parsed.line(), line, "printing the parsed pose gives the same line");
    }
}

#[test]
fn the_field_of_view_is_optional_and_commas_are_allowed() {
    let pose = Pose::parse(&["10,", "20,", "30,", "90,", "-5"]).expect("five numbers");
    assert_eq!(pose, Pose { eye: Vec3::new(10.0, 20.0, 30.0), heading: 90.0, pitch: -5.0, fov: None });
    assert_eq!(pose.line(), "cam 10.0 20.0 30.0 90.0 -5.0");
    let (yaw, pitch) = pose.yaw_pitch_radians();
    assert!((yaw - std::f32::consts::FRAC_PI_2).abs() < 1e-6 && (pitch + 5f32.to_radians()).abs() < 1e-6);
}

#[test]
fn bad_arguments_are_refused_with_the_usage() {
    let refused: [&[&str]; 5] = [
        &[],
        &["1", "2", "3", "4"],
        &["1", "2", "3", "4", "5", "6", "7"],
        &["1", "2", "x", "4", "5"],
        &["1", "2", "3", "4", "NaN"],
    ];
    for args in refused {
        let error = Pose::parse(args).expect_err("refused");
        assert!(error.contains("usage") || error.contains("finite"), "{args:?}: {error}");
    }
    assert!(Pose::parse(&["1", "2", "3", "4", "89.5"]).is_err(), "past the free camera's pitch limit");
    assert!(Pose::parse(&["1", "2", "3", "4", "5", "1"]).is_err(), "too narrow a field of view");
    assert!(Pose::parse(&["1", "2", "3", "4", "5", "179"]).is_err(), "too wide a field of view");
    assert!(Pose::parse(&["1e9", "2", "3", "4", "5"]).is_err(), "out of the world");
}
