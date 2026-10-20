use super::*;

#[test]
fn retained_heat_changes_patrol_detection_and_the_next_chase() {
    let mut police = Police { carried_heat: 4.9, ..Default::default() };
    assert_eq!(police.patrol_heat(), 4);
    let sight = crate::scenes::world::ai::traffic_events::Sighting {
        heat: police.patrol_heat(),
        player_speed: 2.0,
        seen: true,
        recent_hit: false,
        has_911: false,
    };
    assert!(crate::scenes::world::ai::traffic_events::starts_pursuit(&Default::default(), &sight));
    police.begin(1);
    assert_eq!(police.vehicle_heat(), 4.9);
}

#[test]
fn heat_grows_during_sight_grace_but_stops_in_cooldown() {
    let mut police = Police::default();
    police.begin(1);
    police.update(1.0, true, &view(10.0), sensed(true));
    let heat = police.vehicle_heat();
    police.update(2.0, true, &view(10.0), sensed(false));
    assert!(police.vehicle_heat() > heat);
    for _ in 0..120 {
        police.update(0.05, true, &view(10.0), sensed(false));
    }
    assert_eq!(police.cooldown_status(), blackbox_pursuit::CooldownStatus::CoolDown);
    let heat = police.vehicle_heat();
    police.update(1.0, true, &view(10.0), sensed(false));
    assert_eq!(police.vehicle_heat(), heat);
    for _ in 0..80 {
        police.update(0.05, true, &view(10.0), sensed(true));
    }
    assert_eq!(police.cooldown_status(), blackbox_pursuit::CooldownStatus::InitialChase);
    assert!(police.vehicle_heat() > heat);
}

#[test]
fn helicopter_visual_contact_prevents_cooldown_without_a_ground_sighting() {
    let mut police = Police::default();
    police.begin(4);
    police.heli = Some(Helicopter::spawn(Vec3::new(0.0, 15.0, 0.0), 0.0, Default::default()));
    assert!(!police.update(0.25, true, &view(5.0), sensed(false)));
    assert!(police.has_contact());
    assert!(police.escape_progress() <= 0.0);
    assert_eq!(police.evade_level(), 0.0);
}

#[test]
fn another_automatic_pursuit_waits_for_the_arrest_and_ticket_to_finish() {
    let mut police = Police::default();
    assert!(police.can_begin());
    police.start_bust(0.0);
    assert!(!police.can_begin());
    police.tick_bust(3.1);
    assert!(!police.can_begin());
    police.tick_bust(3.1);
    assert!(!police.can_begin());
    police.bust.pay(busted::Payment::Cash, 0).unwrap();
    assert!(police.can_begin());
}

#[test]
fn the_cooldown_bar_opens_empty_although_the_meter_holds_the_grace() {
    let mut police = Police::default();
    police.begin(1);
    let mut shown_at = None;
    for frame in 0..600 {
        police.update(0.05, true, &view(10.0), sensed(false));
        if police.cooldown_status() != blackbox_pursuit::CooldownStatus::CoolDown {
            continue;
        }
        shown_at = Some(frame);
        break;
    }
    assert!(shown_at.is_some(), "the cool-down opens once the cops lose the player");
    police.update(0.05, true, &view(10.0), sensed(false));
    assert!(police.evade_level() > 0.2, "the meter already holds the grace: {}", police.evade_level());
    assert!(police.cooldown_bar() < 0.1, "the board's bar opens empty: {}", police.cooldown_bar());
}
