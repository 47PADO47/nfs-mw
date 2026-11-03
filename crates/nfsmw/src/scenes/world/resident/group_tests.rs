use std::collections::HashMap;

use super::{hidden_group, night_building};

#[test]
fn inspected_downtown_towers_use_night_building_response() {
    for name in ["XB_CITYMODERN24_1A_00", "XB_CITYMODERN15_1A_00", "XB_CITYTOWERB_1A_00", "XB_HAPARTMENT1_1C_JG_00"] {
        assert!(night_building(name), "{name}");
    }
    for name in ["XB_CITYMODERNSHOP_1B_00", "XB_CITYMODERNBASE12_1B_00", "XB_WALKWAY_1B_00"] {
        assert!(!night_building(name), "{name}");
    }
}

fn groups() -> HashMap<u16, u16> {
    HashMap::from([(41, 624), (13601, 389)])
}

#[test]
fn hidden_instance_takes_its_group() {
    assert_eq!(hidden_group(true, Some(41), &groups()), 624);
    assert_eq!(hidden_group(true, Some(13601), &groups()), 389);
}

#[test]
fn visible_instance_is_never_tagged() {
    assert_eq!(hidden_group(false, Some(41), &groups()), 0);
}

#[test]
fn hidden_instance_outside_every_group_is_untagged() {
    assert_eq!(hidden_group(true, None, &groups()), 0);
    assert_eq!(hidden_group(true, Some(7), &groups()), 0);
}

#[test]
fn override_index_too_big_for_a_u16_is_untagged() {
    assert_eq!(hidden_group(true, Some(0x1_0000 + 41), &groups()), 0);
}
