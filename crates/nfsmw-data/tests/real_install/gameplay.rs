//! The gameplay class of `GLOBAL/gameplay.bin` against docs/formats/gameplay-races.md.

use nfsmw_data::gameplay::{self, ActivityKind, RaceType};

use crate::install;

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-2
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn races_triggers_and_activity_zones_match_documented_numbers() {
    let Some(dir) = install() else { return };
    let data = gameplay::load(&dir).expect("gameplay data");

    assert_eq!(data.races.len(), 255, "races with a racestart");
    let kinds = |t: RaceType| data.races.iter().filter(|r| r.race_type == Some(t)).count();
    assert_eq!(kinds(RaceType::Sprint), 51, "p2p");
    assert_eq!(kinds(RaceType::Tollbooth), 68);
    assert_eq!(kinds(RaceType::Challenge), 36);
    assert_eq!(kinds(RaceType::Circuit), 32);
    assert_eq!(kinds(RaceType::SpeedTrap), 31);
    assert_eq!(kinds(RaceType::Knockout), 20);
    assert_eq!(kinds(RaceType::Drag), 11);
    assert!(data.races.iter().all(|r| r.race_type.is_some() || r.event_icon.is_empty()));

    // Every checkpoint and finish line refers to a box trigger that was read.
    for race in &data.races {
        for key in race.checkpoints.iter().chain(race.finish.iter()) {
            assert!(data.trigger(*key).is_some(), "{} refers to an unread trigger {key:08X}", race.name);
        }
    }

    let sprint = data.races.iter().find(|r| r.name == "race_bin_10/10_2_1_sprint").expect("10.2.1 sprint");
    assert_eq!(sprint.event_icon, "p2p");
    assert_eq!(sprint.race_type, Some(RaceType::Sprint));
    assert_eq!(sprint.laps, 1);
    assert_eq!(sprint.region.as_deref(), Some("coastal"));
    assert_eq!(sprint.checkpoints.len(), 9);
    let start = sprint.start.expect("start grid");
    let want = [-4404.865, 209.6045, 1511.824];
    assert!(
        [start.position.x, start.position.y, start.position.z].iter().zip(want).all(|(a, b)| close(*a, b)),
        "start {start:?}"
    );
    assert!(close(start.heading_deg, -198.9169));

    let finish = data.trigger(sprint.finish.expect("finish")).expect("finish trigger");
    assert_eq!(finish.dimensions, [3.0, 65.0, 5.0]);
    assert_eq!(finish.particle_effect.as_deref(), Some("fxgame_flare_red"));
    assert_eq!(finish.flare_spacing, Some(5.0));
    assert!(!finish.one_shot && !finish.fire_on_exit);

    let first = data.trigger(sprint.checkpoints[0]).expect("checkpoint 1");
    assert_eq!(first.name, "race_bin_10/10_2_1_sprint/checkpoint1");
    assert_eq!(first.dimensions, [2.0, 60.0, 40.0]);

    let shops = data.markers.iter().filter(|m| m.kind == ActivityKind::AutoShop).count();
    let houses = data.markers.iter().filter(|m| m.kind == ActivityKind::SafeHouse).count();
    assert_eq!((shops, houses), (8, 4));
    let zone = data.markers.iter().find(|m| m.name == "menu_gate_customshop/zone_customshop_1").expect("shop 1");
    let want = [-4523.472, 214.6646, 698.1821];
    assert!(
        [zone.position.x, zone.position.y, zone.position.z].iter().zip(want).all(|(a, b)| close(*a, b)),
        "shop position {zone:?}"
    );
    assert!(close(zone.radius, 6.0));
    assert_eq!(zone.icon_index, Some(15));
    let respawn = zone.respawn.expect("respawn marker");
    assert!(close(respawn.heading_deg, 82.59451));
}
