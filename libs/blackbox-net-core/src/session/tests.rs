use super::*;

const P1: PeerId = PeerId(1);
const P2: PeerId = PeerId(2);
const P3: PeerId = PeerId(3);

/// A short configuration so that every phase is reachable in a few calls.
fn short_config() -> SessionConfig {
    SessionConfig { load_timeout_ms: 500, intro_ms: 1_000, countdown_ticks: 10 }
}

fn drain(session: &mut Session) -> Vec<SessionEvent> {
    std::iter::from_fn(|| session.poll_event()).collect()
}

/// A session with peers 1 and 2 that has reached `target`. Walks the phases in order, stopping at the first match.
fn session_in(target: Phase) -> Session {
    let mut s = Session::new(short_config());
    s.join(P1).unwrap();
    s.join(P2).unwrap();
    if s.phase() == target {
        return s;
    }
    s.start_loading(0).unwrap();
    if s.phase() == target {
        return s;
    }
    // Deadline 100 + 500 = 600. The peers stay unloaded, so `update` does nothing yet.
    s.host_loaded(100).unwrap();
    if s.phase() == target {
        return s;
    }
    s.peer_loaded(P1).unwrap();
    s.peer_loaded(P2).unwrap();
    s.update(100, 0);
    if s.phase() == target {
        return s;
    }
    // Intro started at 100 and lasts 1000 ms; the countdown starts at tick 5 and lasts 10 ticks.
    s.update(1_100, 5);
    if s.phase() == target {
        return s;
    }
    s.update(1_200, 15);
    if s.phase() == target {
        return s;
    }
    s.finish().unwrap();
    s
}

#[test]
fn a_new_session_is_in_the_lobby_with_no_peers() {
    let mut s = Session::new(SessionConfig::default());
    assert_eq!(s.phase(), Phase::Lobby);
    assert_eq!(s.peers().count(), 0);
    assert_eq!(s.poll_event(), None);
}

#[test]
fn the_default_config_matches_the_contract() {
    let config = SessionConfig::default();
    assert_eq!(config.load_timeout_ms, 120_000);
    assert_eq!(config.intro_ms, 0);
    assert_eq!(config.countdown_ticks, 180);
}

#[test]
fn full_path_with_three_peers() {
    let mut s = Session::new(SessionConfig::default());
    for id in [P1, P2, P3] {
        s.join(id).unwrap();
    }
    assert!(drain(&mut s).is_empty());

    s.start_loading(0).unwrap();
    assert_eq!(s.phase(), Phase::Loading);
    assert!(s.peers().all(|(_, state)| state == PeerState::Loading));

    for id in [P1, P2, P3] {
        s.peer_loaded(id).unwrap();
    }
    s.host_loaded(1_000).unwrap();
    assert_eq!(s.phase(), Phase::WaitLoads);

    // Every peer has loaded, and the intro is 0, so the countdown starts on this call.
    s.update(1_000, 0);
    assert_eq!(s.phase(), Phase::Countdown { start_tick: 180 });
    s.update(1_001, 179);
    assert_eq!(s.phase(), Phase::Countdown { start_tick: 180 });
    s.update(1_002, 180);
    assert_eq!(s.phase(), Phase::Running);
    assert!(s.peers().all(|(_, state)| state == PeerState::InGame));

    s.finish().unwrap();
    assert_eq!(s.phase(), Phase::Results);
    s.back_to_lobby().unwrap();
    assert_eq!(s.phase(), Phase::Lobby);
    assert!(s.peers().all(|(_, state)| state == PeerState::Lobby));

    assert_eq!(
        drain(&mut s),
        vec![
            SessionEvent::Phase(Phase::Loading),
            SessionEvent::Phase(Phase::WaitLoads),
            SessionEvent::Phase(Phase::Countdown { start_tick: 180 }),
            SessionEvent::Phase(Phase::Running),
            SessionEvent::Phase(Phase::Results),
            SessionEvent::Phase(Phase::Lobby),
        ]
    );
}

#[test]
fn peers_iterate_in_id_order() {
    let mut s = Session::new(SessionConfig::default());
    for id in [P3, P1, P2] {
        s.join(id).unwrap();
    }
    let ids: Vec<PeerId> = s.peers().map(|(id, _)| id).collect();
    assert_eq!(ids, vec![P1, P2, P3]);
}

#[test]
fn a_peer_that_never_loads_is_dropped_at_exactly_the_deadline() {
    let mut s = Session::new(SessionConfig::default());
    for id in [P1, P2, P3] {
        s.join(id).unwrap();
    }
    s.start_loading(0).unwrap();
    s.host_loaded(1_000).unwrap();
    s.peer_loaded(P1).unwrap();
    s.peer_loaded(P2).unwrap();
    drain(&mut s);

    // The deadline is 1_000 + 120_000 = 121_000.
    s.update(120_999, 0);
    assert_eq!(s.phase(), Phase::WaitLoads);
    assert_eq!(s.peer(P3), Some(PeerState::Loading));
    assert!(drain(&mut s).is_empty());

    s.update(121_000, 0);
    assert_eq!(s.peer(P3), None);
    assert_eq!(s.phase(), Phase::Countdown { start_tick: 180 });
    assert_eq!(
        drain(&mut s),
        vec![SessionEvent::PeerDropped(P3), SessionEvent::Phase(Phase::Countdown { start_tick: 180 }),]
    );
    assert_eq!(s.peer(P1), Some(PeerState::Loaded));
    assert_eq!(s.peer(P2), Some(PeerState::Loaded));
}

#[test]
fn every_unloaded_peer_is_dropped_in_id_order() {
    let mut s = Session::new(SessionConfig::default());
    for id in [P3, P1, P2] {
        s.join(id).unwrap();
    }
    s.start_loading(0).unwrap();
    s.host_loaded(0).unwrap();
    s.peer_loaded(P2).unwrap();
    drain(&mut s);

    s.update(120_000, 0);
    assert_eq!(s.peers().collect::<Vec<_>>(), vec![(P2, PeerState::Loaded)]);
    assert_eq!(
        drain(&mut s),
        vec![
            SessionEvent::PeerDropped(P1),
            SessionEvent::PeerDropped(P3),
            SessionEvent::Phase(Phase::Countdown { start_tick: 180 }),
        ]
    );
}

#[test]
fn zero_peers_go_on_at_the_first_update() {
    let mut s = Session::new(SessionConfig::default());
    s.start_loading(0).unwrap();
    s.host_loaded(0).unwrap();
    assert_eq!(s.phase(), Phase::WaitLoads);
    s.update(0, 42);
    assert_eq!(s.phase(), Phase::Countdown { start_tick: 222 });
}

#[test]
fn a_peer_may_load_before_the_host() {
    let mut s = Session::new(SessionConfig::default());
    s.join(P1).unwrap();
    s.start_loading(0).unwrap();
    s.peer_loaded(P1).unwrap();
    s.host_loaded(0).unwrap();
    s.update(0, 0);
    assert_eq!(s.phase(), Phase::Countdown { start_tick: 180 });
}

#[test]
fn the_intro_lasts_its_length_then_the_countdown_starts() {
    let config = SessionConfig { load_timeout_ms: 500, intro_ms: 3_000, countdown_ticks: 180 };
    let mut s = Session::new(config);
    s.join(P1).unwrap();
    s.start_loading(0).unwrap();
    s.host_loaded(0).unwrap();
    s.peer_loaded(P1).unwrap();
    s.update(0, 7);
    assert_eq!(s.phase(), Phase::Intro);
    s.update(2_999, 8);
    assert_eq!(s.phase(), Phase::Intro);
    s.update(3_000, 9);
    assert_eq!(s.phase(), Phase::Countdown { start_tick: 189 });
}

#[test]
fn join_is_refused_while_loading() {
    for phase in [Phase::Loading, Phase::WaitLoads, Phase::Intro, Phase::Countdown { start_tick: 15 }, Phase::Results] {
        let mut s = session_in(phase);
        assert_eq!(s.join(P3), Err(SessionError::Closed), "in {phase:?}");
        assert_eq!(s.peer(P3), None);
    }
}

#[test]
fn join_in_progress_while_running_enters_in_game() {
    let mut s = session_in(Phase::Running);
    s.join(P3).unwrap();
    assert_eq!(s.peer(P3), Some(PeerState::InGame));
}

#[test]
fn a_known_id_is_a_duplicate_in_every_phase() {
    for phase in [Phase::Lobby, Phase::Loading, Phase::Running, Phase::Results] {
        let mut s = session_in(phase);
        assert_eq!(s.join(P1), Err(SessionError::Duplicate), "in {phase:?}");
    }
}

#[test]
fn leave_removes_the_peer_and_queues_an_event() {
    let mut s = session_in(Phase::Running);
    drain(&mut s);
    s.leave(P1).unwrap();
    assert_eq!(s.peer(P1), None);
    assert_eq!(drain(&mut s), vec![SessionEvent::PeerLeft(P1)]);
    assert_eq!(s.leave(P1), Err(SessionError::UnknownPeer));
}

#[test]
fn peer_loaded_for_an_unknown_peer_is_refused() {
    let mut s = session_in(Phase::Loading);
    assert_eq!(s.peer_loaded(P3), Err(SessionError::UnknownPeer));
}

#[test]
fn a_repeated_peer_loaded_changes_nothing() {
    let mut s = session_in(Phase::Loading);
    s.peer_loaded(P1).unwrap();
    let before = s.clone();
    s.peer_loaded(P1).unwrap();
    assert_eq!(s, before);
}

#[test]
fn every_wrong_call_in_every_phase_changes_nothing() {
    let phases = [
        Phase::Lobby,
        Phase::Loading,
        Phase::WaitLoads,
        Phase::Intro,
        Phase::Countdown { start_tick: 15 },
        Phase::Running,
        Phase::Results,
    ];
    for phase in phases {
        let mut s = session_in(phase);
        let before = s.clone();

        if phase != Phase::Lobby {
            assert_eq!(s.start_loading(0), Err(SessionError::WrongPhase { phase }), "start_loading in {phase:?}");
        }
        if phase != Phase::Loading {
            assert_eq!(s.host_loaded(0), Err(SessionError::WrongPhase { phase }), "host_loaded in {phase:?}");
        }
        if !matches!(phase, Phase::Loading | Phase::WaitLoads) {
            assert_eq!(s.peer_loaded(P1), Err(SessionError::WrongPhase { phase }), "peer_loaded in {phase:?}");
        }
        if phase != Phase::Running {
            assert_eq!(s.finish(), Err(SessionError::WrongPhase { phase }), "finish in {phase:?}");
        }
        if phase != Phase::Results {
            assert_eq!(s.back_to_lobby(), Err(SessionError::WrongPhase { phase }), "back_to_lobby in {phase:?}");
        }
        assert_eq!(s, before, "state changed in {phase:?}");
    }
}

#[test]
fn a_refused_call_does_not_queue_an_event() {
    let mut s = session_in(Phase::Intro);
    drain(&mut s);
    assert_eq!(s.finish(), Err(SessionError::WrongPhase { phase: Phase::Intro }));
    assert_eq!(s.join(P3), Err(SessionError::Closed));
    assert!(drain(&mut s).is_empty());
}

#[test]
fn session_errors_display_a_message() {
    let errors = [
        SessionError::WrongPhase { phase: Phase::Running },
        SessionError::Duplicate,
        SessionError::Closed,
        SessionError::UnknownPeer,
    ];
    for error in errors {
        assert!(!error.to_string().is_empty());
        let _: &dyn std::error::Error = &error;
    }
}
