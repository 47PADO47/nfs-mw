use super::*;

/// A box trigger 2 m thick (along +z), 40 m wide and 40 m high, centred on the origin.
fn gate() -> BoxTrigger {
    BoxTrigger {
        key: 1,
        name: "race_bin_16/drive_to_safe_house/gpstarget".to_owned(),
        position: Vec3::ZERO,
        heading_deg: 0.0,
        dimensions: [2.0, 40.0, 40.0],
        one_shot: false,
        fire_on_exit: false,
        particle_effect: None,
        flare_spacing: None,
        time_bonus: 0.0,
        resets_player: false,
    }
}

#[test]
fn arrival_is_the_gps_target_box_when_the_data_has_one() {
    let zone = gate();
    assert!(arrived(Vec3::ZERO, Some(&zone), Some(Vec3::new(500.0, 0.0, 0.0))), "inside the box wins over the marker");
    assert!(!arrived(Vec3::new(50.0, 0.0, 0.0), Some(&zone), Some(Vec3::new(50.0, 0.0, 0.0))));
}

#[test]
fn without_the_box_arrival_is_a_circle_round_the_respawn_marker() {
    let marker = Some(Vec3::new(100.0, 0.0, 100.0));
    assert!(arrived(Vec3::new(100.0, 0.0, 129.0), None, marker));
    assert!(!arrived(Vec3::new(100.0, 0.0, 131.0), None, marker));
    assert!(!arrived(Vec3::ZERO, None, None), "no target at all never arrives");
}

#[test]
fn the_drive_targets_the_first_safe_house_zone() {
    assert_eq!(SAFE_HOUSE_ZONE, "menu_gate_safehouse/zone_safehouse_1");
    assert!(GPS_TARGET_SUFFIX.ends_with("gpstarget"));
}
