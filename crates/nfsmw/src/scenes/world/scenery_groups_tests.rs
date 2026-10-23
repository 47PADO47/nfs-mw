use super::SceneryGroups;

#[test]
fn group_zero_is_always_admitted() {
    assert!(SceneryGroups::default().admits(0));
}

#[test]
fn a_group_is_admitted_only_while_it_is_enabled() {
    let mut groups = SceneryGroups::default();
    assert!(!groups.admits(200));
    assert!(groups.set(200, true));
    assert!(groups.admits(200));
    assert!(!groups.admits(356), "another group stays off");
    assert!(groups.set(200, false));
    assert!(!groups.admits(200));
}

#[test]
fn switching_a_group_to_its_current_state_changes_nothing() {
    let mut groups = SceneryGroups::default();
    assert!(!groups.set(200, false), "off stays off");
    assert!(groups.set(200, true));
    assert!(!groups.set(200, true), "on stays on");
    assert!(!groups.is_empty());
    assert!(groups.set(200, false));
    assert!(groups.is_empty());
}

#[test]
fn a_switch_carries_the_turn_and_a_plain_set_keeps_it() {
    let mut groups = SceneryGroups::default();
    groups.switch(244, Some(true));
    assert!(groups.admits(244) && groups.turned(244));
    assert!(!groups.set(244, true), "already on");
    assert!(groups.turned(244), "a plain set does not un-turn");
    groups.switch(244, Some(false));
    assert_eq!(groups.state(244), Some(false));
    groups.switch(244, None);
    assert_eq!(groups.state(244), None);
    assert!(!groups.turned(244));
}

#[test]
fn a_group_is_found_by_its_name() {
    let door = blackbox_scenery::SceneryGroup {
        hash: blackbox_hash::bstring_hash("SCENERY_GROUP_DOOR"),
        number: 71,
        flag: 0,
        drive_through: false,
        override_indexes: vec![10687],
    };
    let groups = SceneryGroups::new(vec![door]);
    assert_eq!(groups.named("SCENERY_GROUP_DOOR").map(|g| g.number), Some(71));
    assert!(groups.named("scenery_group_door").is_none(), "the hash is case sensitive");
}

#[test]
fn every_group_but_a_drive_through_one_blocks_from_both_sides() {
    let group = |number: u16, drive_through: bool| blackbox_scenery::SceneryGroup {
        hash: u32::from(number),
        number,
        flag: 1,
        drive_through,
        override_indexes: Vec::new(),
    };
    let groups = SceneryGroups::new(vec![group(200, false), group(356, true)]);
    assert!(groups.two_sided(200));
    assert!(!groups.two_sided(356), "a drive-through group stays one-sided");
    assert!(groups.two_sided(500), "a group the table does not list is a wall");
    assert!(!groups.two_sided(0), "group 0 is no group");
}
