use super::catalog::{AudioCue, Category};
use super::*;
use crate::hud::RaceHud;

/// Every kind of event, with sample values.
fn every_event() -> Vec<HudEvent> {
    use PursuitPhase as P;
    vec![
        HudEvent::PhaseChanged { from: P::Active, to: P::Search },
        HudEvent::HeatUp { level: 4 },
        HudEvent::RoadblockReported,
        HudEvent::HelicopterTracking,
        HudEvent::ContactLost,
        HudEvent::CooldownStarted,
        HudEvent::Escaped,
        HudEvent::Busted,
        HudEvent::Checkpoint { number: 2, total: 6 },
        HudEvent::WrongWay,
        HudEvent::Lap { lap: 2, laps: 3 },
        HudEvent::PositionChanged { from: 3, to: 2 },
        HudEvent::NearMiss,
        HudEvent::SpeedTrap { kmh: 190.0, passed: true },
        HudEvent::SpeedTrap { kmh: 90.0, passed: false },
        HudEvent::BountyMilestone { bounty: 20_000 },
        HudEvent::TirePunctured,
        HudEvent::HeavyDamage,
        HudEvent::VehicleCritical,
        HudEvent::NitrousOn,
    ]
}

/// A HUD state of frame `frame` carrying `events`.
fn frame(frame: u64, events: Vec<HudEvent>) -> HudState {
    HudState { events, events_frame: frame, ..Default::default() }
}

fn visible(notifications: &HudNotifications) -> Vec<NoticeId> {
    notifications.queue.visible().iter().map(|active| active.key).collect()
}

#[test]
fn every_text_of_the_catalogue_is_in_the_english_catalogue() {
    for event in every_event() {
        let Some(entry) = spec(&event) else { continue };
        for id in entry.text.into_iter().chain(entry.secondary) {
            let text = crate::i18n::text(id, &crate::i18n::Args::new());
            assert_ne!(text, id, "{id} is missing from i18n/en/hud.ftl");
        }
    }
}

#[test]
fn the_priorities_follow_the_briefs_examples() {
    let priority = |event: HudEvent| spec(&event).unwrap().priority;
    assert_eq!(priority(HudEvent::RoadblockReported), Priority::Critical, "a hazard on the road ahead");
    assert_eq!(priority(HudEvent::TirePunctured), Priority::High, "a mechanical warning");
    assert_eq!(priority(HudEvent::HeavyDamage), Priority::High);
    assert_eq!(priority(HudEvent::WrongWay), Priority::Medium, "the missed checkpoint");
    assert_eq!(priority(HudEvent::NearMiss), Priority::Low, "passing news");
}

#[test]
fn the_race_and_chase_notices_show_while_the_game_is_intense() {
    for event in every_event() {
        let Some(entry) = spec(&event).filter(|entry| entry.text.is_some()) else { continue };
        if matches!(entry.category, Category::Race | Category::Pursuit) {
            assert!(!entry.priority.deferrable(), "{:?} would wait during a race or a chase", entry.id);
        }
    }
}

#[test]
fn a_phase_change_has_no_notice_and_the_nitrous_only_feedback() {
    assert_eq!(spec(&every_event()[0]), None);
    let nitrous = spec(&HudEvent::NitrousOn).unwrap();
    assert_eq!((nitrous.text, nitrous.audio), (None, Some(AudioCue::Nitrous)));
    let mut notifications = HudNotifications::default();
    notifications.take_events(&frame(1, vec![HudEvent::NitrousOn]), NotificationLevel::All);
    assert!(visible(&notifications).is_empty());
}

#[test]
fn an_identical_state_over_many_frames_gives_one_notification() {
    let mut notifications = HudNotifications::default();
    let punctured = frame(1, vec![HudEvent::TirePunctured]);
    // The same frame handed over again (a paused scene) adds nothing.
    for _ in 0..5 {
        notifications.take_events(&punctured, NotificationLevel::All);
        notifications.tick(1.0 / 60.0, false);
    }
    // The next frames have no event: the state did not change.
    for n in 2..60 {
        notifications.take_events(&frame(n, Vec::new()), NotificationLevel::All);
        notifications.tick(1.0 / 60.0, false);
    }
    assert_eq!(visible(&notifications), [NoticeId::TirePunctured]);
    assert_eq!(notifications.queue.visible()[0].count, 1);
}

#[test]
fn the_level_filters_by_priority() {
    let events = vec![HudEvent::Checkpoint { number: 1, total: 4 }, HudEvent::RoadblockReported];
    let mut important = HudNotifications::default();
    important.take_events(&frame(1, events.clone()), NotificationLevel::Important);
    assert_eq!(visible(&important), [NoticeId::Checkpoint, NoticeId::Roadblock], "important keeps the Medium ones");
    let mut off = HudNotifications::default();
    off.take_events(&frame(1, events), NotificationLevel::Off);
    assert_eq!(visible(&off), [NoticeId::Roadblock], "a Critical warning is never filtered out");
}

#[test]
fn a_notice_of_a_phase_goes_when_the_chase_leaves_it() {
    use PursuitPhase as P;
    let mut notifications = HudNotifications::default();
    let lost = vec![HudEvent::PhaseChanged { from: P::Active, to: P::Search }, HudEvent::ContactLost];
    notifications.take_events(&frame(1, lost), NotificationLevel::All);
    assert_eq!(visible(&notifications), [NoticeId::ContactLost]);
    let reacquired = vec![HudEvent::PhaseChanged { from: P::Search, to: P::Active }];
    notifications.take_events(&frame(2, reacquired), NotificationLevel::All);
    assert!(visible(&notifications).is_empty(), "the search is over");
}

#[test]
fn the_dismiss_action_takes_the_oldest_notice() {
    let mut notifications = HudNotifications::default();
    notifications.take_events(&frame(1, vec![HudEvent::HeavyDamage, HudEvent::TirePunctured]), NotificationLevel::All);
    assert!(notifications.dismiss_oldest());
    assert_eq!(visible(&notifications), [NoticeId::TirePunctured]);
    assert!(notifications.dismiss_oldest());
    assert!(!notifications.dismiss_oldest(), "nothing left to dismiss");
}

#[test]
fn near_misses_wait_during_an_active_chase_and_go_stale() {
    let chase = HudState { pursuit_phase: PursuitPhase::Active, ..frame(1, vec![HudEvent::NearMiss]) };
    assert!(intense(&chase));
    let mut notifications = HudNotifications::default();
    notifications.tick(0.016, intense(&chase));
    notifications.take_events(&chase, NotificationLevel::All);
    assert!(visible(&notifications).is_empty(), "a Low notice waits while the chase is active");
    notifications.tick(2.0, true);
    notifications.tick(0.016, false);
    assert!(visible(&notifications).is_empty(), "too late to tell: it went stale");
}

#[test]
fn a_race_is_intense_once_the_countdown_is_over() {
    let race = |count| RaceHud { place: 1, racers: 4, lap: None, clock: 0.0, progress: 0.0, count };
    assert!(!intense(&HudState { race: Some(race(Some(crate::hud::Count::Two))), ..Default::default() }));
    assert!(intense(&HudState { race: Some(race(None)), ..Default::default() }));
    assert!(!intense(&HudState::default()));
}
