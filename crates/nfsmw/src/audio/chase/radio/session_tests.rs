use super::*;

#[test]
fn batch12_radio_session_reset_clears_chase_and_pending_911_state() {
    let mut radio = ChaseRadio::new(11);
    let tune = ChaseTune { pursuit_init_delay: 3.5, ..Default::default() };
    radio.set_tune(&tune);
    let feed = PursuitFeed { heat: 4, secs: 60.0, player_mph: 90.0, ..Default::default() };
    radio.run(0.2, Some(&feed), EndReason::None, &LevelSpeech::default(), None);
    radio.call_911 = Some(12.0);
    radio.awaiting_911 = 20.0;
    radio.roam_armed = true;
    radio.last_ended = EndReason::Busted;
    assert!(radio.running && radio.last_snapshot.is_some());
    radio.reset_session();
    assert!(!radio.running && radio.last_snapshot.is_none());
    assert_eq!(radio.take_911_call(), None);
    assert_eq!(radio.awaiting_911, 0.0);
    assert!(!radio.roam_armed);
    assert_eq!(radio.last_ended, EndReason::None);
    assert!(radio.sent.is_empty());
    assert_eq!(radio.tune.pursuit_init_delay, 3.5, "database tuning survives a scene change");
    for _ in 0..100 {
        assert!(radio.run(0.1, None, EndReason::None, &LevelSpeech::default(), None).is_empty());
    }
    radio.run(0.2, Some(&feed), EndReason::None, &LevelSpeech::default(), None);
    assert!(radio.running, "the next gameplay session can still start police speech");
}
