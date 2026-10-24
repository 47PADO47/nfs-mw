use nfsmw_modding::abi::EndReason as MwEndReason;

use super::*;

/// Runs `frames` frames of the same snapshot after a baseline, and returns every event raised.
fn run(feed: &mut HudFeed, frames: &[FeedSnapshot]) -> Vec<HudEvent> {
    let mut all = Vec::new();
    for snapshot in frames {
        feed.begin_frame();
        feed.observe(*snapshot, 1.0 / 60.0);
        all.extend_from_slice(feed.events());
    }
    all
}

fn snapshot() -> FeedSnapshot {
    FeedSnapshot { bounty: Some(0), ..FeedSnapshot::default() }
}

#[test]
fn the_first_snapshot_only_sets_the_baseline() {
    let mut feed = HudFeed::default();
    let busy = FeedSnapshot { phase: PursuitPhase::Active, nos_burning: true, place: Some(2), ..snapshot() };
    assert!(run(&mut feed, &[busy]).is_empty(), "a world that opens into a state does not announce it");
}

#[test]
fn an_identical_state_over_many_frames_raises_one_event() {
    let mut feed = HudFeed::default();
    let burning = FeedSnapshot { nos_burning: true, ..snapshot() };
    let mut frames = vec![snapshot()];
    frames.extend(std::iter::repeat_n(burning, 120));
    assert_eq!(run(&mut feed, &frames), [HudEvent::NitrousOn]);
}

#[test]
fn the_events_are_gone_the_next_frame_and_the_frame_counts() {
    let mut feed = HudFeed::default();
    feed.begin_frame();
    feed.observe(snapshot(), 0.016);
    feed.push(HudEvent::WrongWay);
    let first = feed.frame();
    assert_eq!(feed.events(), [HudEvent::WrongWay]);
    feed.begin_frame();
    assert!(feed.events().is_empty());
    assert_eq!(feed.frame(), first + 1);
}

#[test]
fn a_chase_raises_its_phase_changes_with_the_contact_lost_and_the_cool_down() {
    use PursuitPhase as P;
    let mut feed = HudFeed::default();
    let at = |phase| FeedSnapshot { phase, ..snapshot() };
    let events = run(&mut feed, &[at(P::None), at(P::Active), at(P::Search), at(P::Cooldown), at(P::Escaped)]);
    assert_eq!(
        events,
        [
            HudEvent::PhaseChanged { from: P::None, to: P::Active },
            HudEvent::PhaseChanged { from: P::Active, to: P::Search },
            HudEvent::ContactLost,
            HudEvent::PhaseChanged { from: P::Search, to: P::Cooldown },
            HudEvent::CooldownStarted,
            HudEvent::PhaseChanged { from: P::Cooldown, to: P::Escaped },
        ]
    );
}

#[test]
fn a_reacquire_is_a_phase_change_and_not_a_lost_contact() {
    use PursuitPhase as P;
    let mut feed = HudFeed::default();
    let at = |phase| FeedSnapshot { phase, ..snapshot() };
    let events = run(&mut feed, &[at(P::Cooldown), at(P::Active)]);
    assert_eq!(events, [HudEvent::PhaseChanged { from: P::Cooldown, to: P::Active }]);
}

#[test]
fn reports_sightings_damage_and_places_are_raised_on_their_rising_edge() {
    let mut feed = HudFeed::default();
    let before = FeedSnapshot { place: Some(3), ..snapshot() };
    let after = FeedSnapshot {
        roadblock_reported: true,
        helicopter_tracking: true,
        place: Some(2),
        damaged_tires: 1,
        heavy_damage: true,
        critical: true,
        ..snapshot()
    };
    let events = run(&mut feed, &[before, after, after]);
    assert_eq!(
        events,
        [
            HudEvent::RoadblockReported,
            HudEvent::HelicopterTracking,
            HudEvent::PositionChanged { from: 3, to: 2 },
            HudEvent::TirePunctured,
            HudEvent::HeavyDamage,
            HudEvent::VehicleCritical,
        ]
    );
    let healed = run(&mut feed, &[snapshot()]);
    assert!(healed.is_empty(), "a repair raises nothing, and leaving the race is no place change: {healed:?}");
}

#[test]
fn a_second_puncture_is_raised_again() {
    let mut feed = HudFeed::default();
    let tires = |damaged_tires| FeedSnapshot { damaged_tires, ..snapshot() };
    assert_eq!(run(&mut feed, &[tires(0), tires(1), tires(1), tires(2)]).len(), 2);
}

#[test]
fn near_misses_are_throttled() {
    let mut feed = HudFeed::default();
    let passes = |near_misses| FeedSnapshot { near_misses, ..snapshot() };
    let mut told = 0;
    feed.begin_frame();
    feed.observe(passes(0), 0.1);
    // A near miss every 0.1 s for 3 s: one at once, then one every 1.5 s.
    for n in 1..=30 {
        feed.begin_frame();
        feed.observe(passes(n), 0.1);
        told += feed.events().iter().filter(|event| **event == HudEvent::NearMiss).count();
    }
    assert_eq!(told, 2);
}

#[test]
fn a_bounty_milestone_is_the_highest_multiple_passed() {
    assert_eq!(milestone(Some(9_500), Some(10_200)), Some(10_000));
    assert_eq!(milestone(Some(9_500), Some(31_000)), Some(30_000));
    assert_eq!(milestone(Some(10_200), Some(19_999)), None);
    assert_eq!(milestone(Some(25_000), Some(5_000)), None, "a bounty that falls passes no milestone");
    assert_eq!(milestone(None, Some(50_000)), None, "entering career mode is not a milestone");
}

#[test]
fn a_bounty_gain_is_raised_with_its_amount() {
    assert_eq!(gained(Some(9_500), Some(10_200)), Some(700));
    assert_eq!(gained(Some(10_200), Some(10_200)), None, "an unchanged bounty gained nothing");
    assert_eq!(gained(Some(25_000), Some(5_000)), None, "a bounty that falls gained nothing");
    assert_eq!(gained(None, Some(50_000)), None, "entering career mode is not a gain");
    let mut feed = HudFeed::default();
    let richer = FeedSnapshot { bounty: Some(10_500), ..snapshot() };
    let events = run(&mut feed, &[snapshot(), richer, richer]);
    assert_eq!(events, [HudEvent::BountyMilestone { bounty: 10_000 }, HudEvent::BountyGained { amount: 10_500 }]);
}

#[test]
fn the_chase_notices_give_heat_up_busted_and_escaped() {
    let mut feed = HudFeed::default();
    feed.begin_frame();
    feed.notices(&[
        Notice::HeatChanged { level: 3, old_heat: 2.9, new_heat: 3.0 },
        Notice::HeatChanged { level: 2, old_heat: 3.0, new_heat: 2.9 },
        Notice::Busted { level: 3, bounty: 500 },
        Notice::Escaped { level: 3, duration: 80.0 },
        Notice::PursuitEnded { reason: MwEndReason::Evaded, bounty: 0, duration: 80.0 },
    ]);
    assert_eq!(feed.events(), [HudEvent::HeatUp { level: 3 }, HudEvent::Busted, HudEvent::Escaped]);
}

#[test]
fn only_the_players_race_events_are_raised() {
    let mut feed = HudFeed::default();
    feed.begin_frame();
    for event in [
        RaceEvent::Checkpoint { racer: 0, number: 2 },
        RaceEvent::Checkpoint { racer: 1, number: 2 },
        RaceEvent::Lap { racer: 0, lap: 1 },
        RaceEvent::SpeedTrap { racer: 0, slot: 0, kmh: 180.0 },
        RaceEvent::Reset { racer: 0 },
        RaceEvent::Go,
    ] {
        feed.race_event(&event, 5, 3);
    }
    assert_eq!(
        feed.events(),
        [
            HudEvent::Checkpoint { number: 2, total: 5 },
            HudEvent::Lap { lap: 2, laps: 3 },
            HudEvent::SpeedTrap { kmh: 180.0, passed: true },
            HudEvent::WrongWay,
        ]
    );
}
