//! Installed event overrides must select the actual authored vehicle mix and cruising speeds.

use super::*;
use glam::Vec3;
use nfsmw_data::gameplay::RaceType;

#[test]
#[ignore = "requires NFSMW_GAME_DIR"]
fn installed_traffic_event_patterns_resolve_the_authored_ambient_mix() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR"))
        .expect("game installation");
    let db = nfsmw_data::shared::attributes(&dir).expect("installed attributes");
    let gameplay = nfsmw_data::gameplay::cached(&dir).expect("installed events");
    let authored = patterns(&db);
    let mut selected = build_patterns(&db, None).expect("installed traffic patterns");
    let mut checked = 0;
    for race in &gameplay.races {
        if race.race_type == Some(RaceType::Drag) {
            continue;
        }
        let Some(name) = race.params.traffic_pattern.as_deref().filter(|name| !name.is_empty()) else { continue };
        let expected = authored
            .iter()
            .find(|pattern| pattern.name.eq_ignore_ascii_case(name))
            .unwrap_or_else(|| panic!("{} has an unresolved ambient pattern {name}", race.name));
        selected.select_event(Vec3::ZERO, Some(name));
        let actual = selected.current().expect("event pattern overrides the absent zone");
        assert_eq!(actual, &pattern_of(expected), "event {} must use its authored traffic mix", race.name);
        assert!(!selected.select_event(Vec3::X, Some(name)), "holding an event cannot reset spawn timers");
        checked += 1;
    }
    assert!(checked > 0, "no non-drag event override was checked");
    eprintln!("checked {checked} ambient event overrides against {} installed patterns", authored.len());
}
