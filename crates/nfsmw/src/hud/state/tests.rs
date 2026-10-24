//! The tests of the HUD state's numbers: the speed readout, the tachometer face and the red-line masks.

use super::*;
#[test]
fn speed_converts_and_cuts_off() {
    let s = HudState { speed: 27.78, ..Default::default() };
    assert_eq!(s.display_speed(), 100);
    let m = HudState { speed: 26.9, use_mph: true, ..Default::default() };
    assert_eq!(m.display_speed(), 60);
    // 99.9 km/h reads 99, as the game's integer digits do.
    assert_eq!(HudState { speed: 27.75, ..Default::default() }.display_speed(), 99);
    assert_eq!(HudState { speed: -3.0, ..Default::default() }.display_speed(), 0);
    assert_eq!(HudState { speed: 1.0e6, ..Default::default() }.display_speed(), 999);
}

#[test]
fn the_scale_is_the_next_face_above_max_rpm() {
    for (max, face) in [(6500.0, 7000.0), (7000.0, 8000.0), (7500.0, 8000.0), (8000.0, 9000.0), (8999.0, 9000.0)] {
        assert_eq!(scale_rpm(max), face, "{max}");
    }
    assert_eq!(scale_rpm(9000.0), 10000.0);
    assert_eq!(scale_rpm(12000.0), 10000.0);
}

#[test]
fn rpm_fraction_clamps_and_follows_the_face() {
    let s = HudState { rpm: 9500.0, max_rpm: 8250.0, ..Default::default() };
    assert_eq!(s.rpm_fraction(), 1.0);
    let half = HudState { rpm: 4500.0, max_rpm: 8250.0, ..Default::default() };
    assert_eq!(half.rpm_fraction(), 0.5, "the face is the 9000 one");
}

#[test]
fn the_drag_redline_follows_the_executables_drag_table() {
    // The constants of the drag branch: 0x42260000 is 41.5, 0x42140000 is 37, 0x41fc0000 is 31.5, 0x41d80000 is 27.
    assert_eq!(redline_rotation_drag(9500.0, 9600.0), 41.5);
    assert_eq!(redline_rotation_drag(9500.0, 9200.0), 37.0);
    assert_eq!(redline_rotation_drag(10000.0, 8700.0), 31.5);
    assert_eq!(redline_rotation_drag(8250.0, 8000.0), 37.0);
    assert_eq!(redline_rotation_drag(8250.0, 8700.0), 42.0);
    assert_eq!(redline_rotation_drag(7500.0, 7100.0), 34.0);
    assert_eq!(redline_rotation_drag(6500.0, 5000.0), 19.25);
}

#[test]
fn the_redline_rotation_follows_the_table() {
    assert_eq!(redline_rotation(8250.0, 8000.0), 154.0);
    assert_eq!(redline_rotation(8250.0, 8700.0), 166.0);
    assert_eq!(redline_rotation(8250.0, 6000.0), 115.0);
    assert_eq!(redline_rotation(6800.0, 6500.0), 164.5);
    assert_eq!(redline_rotation(7500.0, 7100.0), 152.0);
    assert_eq!(redline_rotation(9500.0, 9200.0), 156.0);
    assert_eq!(redline_rotation(10000.0, 7000.0), 123.0);
}
