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
    // The arrest scene is answered as unavailable, so the bust goes on to its ticket after the hold.
    let _ = police.bust.arrest_scene_over();
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

#[test]
fn the_career_bin_caps_the_chase_heat_and_sets_how_fast_it_climbs() {
    use nfsmw_data::pursuit::HeatGrowth;
    let scale =
        [120.0, 120.0, 120.0, 120.0, 120.0, 120.0, 120.0, 120.0, 120.0, 120.0, 120.0, 180.0, 240.0, 360.0, 600.0];
    let growth = vec![Some(HeatGrowth { time_per_heat_level: 1.0, scale_per_bucket: scale })];
    let climb = |bin: Option<u8>, secs: u32| {
        let mut police = Police::default();
        police.set_heat_growth(growth.clone());
        police.set_career_bin(bin);
        police.begin(1);
        for _ in 0..secs {
            police.update(1.0, true, &view(10.0), sensed(true));
        }
        police.vehicle_heat()
    };
    assert!((climb(Some(10), 60) - 1.5).abs() < 0.01, "120 s a level from bin 10: {}", climb(Some(10), 60));
    assert!((climb(Some(15), 60) - 1.1).abs() < 0.01, "600 s a level at the start of the Blacklist");
    assert!(climb(Some(15), 5000) <= 2.0, "the start of the Blacklist never leaves heat 2");
    assert!(climb(Some(10), 5000) <= 3.0, "bin 10 stops at heat 3");
    assert!((climb(None, 5) - (1.0 + 5.0 / 600.0)).abs() < 0.0001, "sandbox stays at heat 1 during the opening");
    assert!((climb(None, 60) - 1.1).abs() < 0.01, "no-profile sandbox uses the initial career growth bucket");
    assert_eq!(climb(None, 5000), 5.0, "ordinary sandbox heat stops at 5");
}

#[test]
fn a_forced_chase_raises_the_bin_range_to_its_level() {
    let mut police = Police::default();
    police.set_career_bin(Some(15));
    police.set_forced_pursuit(true);
    police.begin(6);
    assert_eq!(police.level(), Some(6));
}
