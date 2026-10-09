//! The speech events of the attribute database: 133 collections and the tuning.

use blackbox_attrib::Database;
use nfsmw_data::speech::{SpeechEvent, events, tune};

use crate::install;

fn find<'a>(list: &'a [SpeechEvent], name: &str) -> &'a SpeechEvent {
    list.iter().find(|e| e.name == name).unwrap_or_else(|| panic!("no speech event {name}"))
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_speech_events_carry_their_scheduling_data() {
    let Some(dir) = install() else { return };
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).expect("attributes.bin");
    let list = events(&db);
    assert_eq!(list.len(), 133);
    assert_eq!(tune(&db).dropoff_ramp, [30.0, 500.0]);
    assert!(list.iter().all(|e| !e.name.is_empty() && e.expiry > 0.0));

    // The template everything inherits from.
    assert_eq!(*find(&list, "default"), SpeechEvent { name: "default".into(), ..SpeechEvent::default() });

    let acknowledge = find(&list, "acknowledge");
    assert_eq!((acknowledge.id, acknowledge.priority, acknowledge.interruptable), (57, 50, false));
    assert_eq!((acknowledge.expiry, acknowledge.interval, acknowledge.back_time), (15.0, 60.0, 10.0));
    assert_eq!(acknowledge.dep_follow, [154, 58, 216, 84]);

    let ram = find(&list, "anytimeevents_intenttoram");
    assert!(ram.interrupt && !ram.interruptable && ram.do_not_dropout);
    assert_eq!((ram.id, ram.priority, ram.expiry), (113, 86, 1.5));

    let weather = find(&list, "anytimeevents_weatherreport");
    assert_eq!((weather.max_playback, weather.min_player_speed), (1, 30.0));

    let delayed = find(&list, "interrupts_interruptramhigh");
    assert_eq!((delayed.init_delay, delayed.enforce_dead_air), (0.5, 1.0));
}
