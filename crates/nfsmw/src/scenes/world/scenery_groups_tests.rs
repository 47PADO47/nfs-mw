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
