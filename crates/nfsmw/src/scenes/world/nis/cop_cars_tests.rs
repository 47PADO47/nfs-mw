use super::*;

#[test]
fn slot_zero_is_the_player_and_slots_one_to_seven_are_civilian() {
    assert_eq!(slot_kind(0), Some(SlotKind::Player), "car1 is the player's car");
    for slot in 1..=7 {
        assert_eq!(slot_kind(slot), Some(SlotKind::Civilian), "car{} takes traffic", slot + 1);
    }
}

#[test]
fn slots_eight_to_fifteen_are_cops() {
    for slot in 8..=15 {
        assert_eq!(slot_kind(slot), Some(SlotKind::Cop), "cop{} takes a police car", slot - 7);
    }
}

#[test]
fn a_slot_past_the_last_one_has_no_kind() {
    assert_eq!(slot_kind(CAR_SLOTS), None);
    assert_eq!(slot_kind(CAR_SLOTS + 3), None);
}

#[test]
fn a_cop_channel_borrows_the_nearest_free_police_car_before_spawning() {
    let choice = choose_cop(CopSource::BorrowFirst, Some(4), || panic!("no police car is spawned while one is free"));
    assert_eq!(choice, Some(Target::Traffic(4)));
}

#[test]
fn a_cop_channel_spawns_a_police_car_when_none_is_free() {
    assert_eq!(choose_cop(CopSource::BorrowFirst, None, || Some(9)), Some(Target::Spawned(9)));
}

#[test]
fn a_cop_channel_stays_off_when_it_neither_borrows_nor_spawns() {
    assert_eq!(choose_cop(CopSource::BorrowFirst, None, || None), None);
    assert_eq!(choose_cop(CopSource::SpawnFirst, None, || None), None);
}

#[test]
fn an_arrest_spawns_its_own_police_car_even_with_a_chase_car_free() {
    assert_eq!(cop_source(NisKind::Arrest), CopSource::SpawnFirst);
    assert_eq!(choose_cop(CopSource::SpawnFirst, Some(4), || Some(9)), Some(Target::Spawned(9)));
}

#[test]
fn an_arrest_borrows_a_chase_car_only_when_it_cannot_spawn() {
    assert_eq!(choose_cop(CopSource::SpawnFirst, Some(4), || None), Some(Target::Traffic(4)));
}

#[test]
fn other_scenes_borrow_first() {
    for kind in [NisKind::Intro, NisKind::Outro, NisKind::Moment, NisKind::Ending] {
        assert_eq!(cop_source(kind), CopSource::BorrowFirst, "{kind:?}");
    }
}
