//! Verify chase event routing against installed attributes, without claiming listening parity.

use super::*;
use crate::audio::speech::{Dispatcher, Outcome, Request, random::Rng};
use game_install::GameDir;
use nfsmw_data::car::physics::PhysicsData;

#[test]
#[ignore = "needs installed speech attributes"]
fn installed_chase_radio_ids_resolve_and_queue() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR")).unwrap();
    let data = PhysicsData::load(&dir).unwrap();
    let events = nfsmw_data::speech::events(data.database());
    let tune = nfsmw_data::speech::tune(data.database());
    let mut dispatcher = Dispatcher::new(events, tune, Rng::new(7));
    let cues = [
        Cue::Lost,
        Cue::Regained,
        Cue::HeatJump,
        Cue::TimeExpired,
        Cue::Rain,
        Cue::RoadblockCalled,
        Cue::BeenHit,
        Cue::HeliLost,
        Cue::HeliSpotted,
        Cue::CopDown,
    ];
    let requests = std::iter::once((OPENING_LINE, OPENING_ID)).chain(cues.map(|cue| (cue_line(cue), cue_id(cue))));
    for (label, id) in requests {
        let event = dispatcher.event(id).unwrap_or_else(|| panic!("{label}: installed SpeechID {id} is absent"));
        assert!(!event.name.is_empty(), "{label}: installed event has no collection name");
        println!("{label}: SpeechID {id} -> {}", event.name);
        assert_eq!(dispatcher.request(id, Request::default()), Outcome::Queued, "{label}");
    }
}
