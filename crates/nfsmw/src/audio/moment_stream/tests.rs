use super::*;

const LOW: Rank = Rank { priority: 1, can_interrupt: false };
const HIGH: Rank = Rank { priority: 5, can_interrupt: true };
const CAR2CAR: u32 = vlt_hash("car2car");
const STING: u32 = vlt_hash("purs_break_start_fx");
const GATE: u32 = vlt_hash("gate");
const BOOTH: u32 = vlt_hash("tollbooth");

fn playing(owner: Owner, key: u32, rank: Rank) -> Arbiter {
    let mut stream = Arbiter::default();
    stream.started(owner, key, rank, Some(Vec3::ZERO));
    stream
}

#[test]
fn an_idle_stream_admits_anything_a_cinematic_stream_does_not_block() {
    let stream = Arbiter::default();
    assert_eq!(stream.receive(CAR2CAR, LOW, &[], false), Decision::Start { interrupt: None });
    assert_eq!(stream.receive(CAR2CAR, LOW, &[], true), Decision::Refuse);
}

#[test]
fn a_lower_priority_moment_is_refused_and_an_equal_or_higher_one_interrupts() {
    // The capture: the breaker sting (higher priority) refused the car crash that followed it.
    let stream = playing(Owner::Breaker, STING, HIGH);
    assert_eq!(stream.receive(CAR2CAR, LOW, &[], false), Decision::Refuse);
    let stream = playing(Owner::World, CAR2CAR, LOW);
    assert_eq!(stream.receive(STING, HIGH, &[], false), Decision::Start { interrupt: Some(Owner::World) });
    assert_eq!(
        stream.receive(vlt_hash("logs_drop"), LOW, &[], false),
        Decision::Start { interrupt: Some(Owner::World) },
        "equal priority replaces"
    );
}

#[test]
fn the_same_moment_waits_two_seconds_except_booths_and_windows() {
    let mut stream = playing(Owner::World, CAR2CAR, LOW);
    assert_eq!(stream.receive(CAR2CAR, LOW, &[], false), Decision::Refuse);
    stream.update(1.9, &[Vec3::ZERO], true, false, |_| None);
    assert_eq!(stream.receive(CAR2CAR, LOW, &[], false), Decision::Refuse);
    stream.update(0.11, &[Vec3::ZERO], true, false, |_| None);
    assert_eq!(stream.receive(CAR2CAR, LOW, &[], false), Decision::Start { interrupt: Some(Owner::World) });
    let stream = playing(Owner::World, BOOTH, LOW);
    assert_eq!(stream.receive(BOOTH, LOW, &[], false), Decision::Start { interrupt: Some(Owner::World) });
}

#[test]
fn a_finished_voice_frees_the_stream_and_a_distant_moment_is_stopped() {
    let mut stream = playing(Owner::World, CAR2CAR, LOW);
    stream.update(0.016, &[Vec3::ZERO], false, false, |_| None);
    assert_eq!(stream.current(), None);
    let mut stream = playing(Owner::Petrol, vlt_hash("expl_gas_station"), HIGH);
    let tick = stream.update(0.016, &[Vec3::X * 121.0], true, false, |_| None);
    assert_eq!((tick.stop, stream.current()), (Some(Owner::Petrol), None));
    let mut stream = Arbiter::default();
    stream.started(Owner::Thunder, vlt_hash("thunder_close"), LOW, None);
    let tick = stream.update(0.016, &[Vec3::X * 500.0], true, false, |_| None);
    assert_eq!((tick.stop, stream.current()), (None, Some(vlt_hash("thunder_close"))), "heard at the listener");
}

#[test]
fn approaching_a_breaker_marker_queues_its_moment_and_unpause_releases_it_there() {
    let marker = Vec3::new(100.0, 50.0, 3.0);
    let mut stream = Arbiter::default();
    stream.set_markers(vec![(marker, GATE)]);
    let ranks = |key| (key == GATE).then_some(HIGH);
    let far = stream.update(0.016, &[marker + Vec3::new(21.0, 0.0, 0.0)], false, false, ranks);
    assert_eq!(far.queued, None, "outside 20 m on one axis");
    let near = stream.update(0.016, &[marker + Vec3::new(15.0, -12.0, 0.0)], false, false, ranks);
    assert_eq!(near.queued, Some(GATE));
    assert!(stream.is_held());
    assert_eq!(stream.point(), Some(marker));
    // Queued, nothing plays; staying near keeps it.
    stream.update(0.016, &[marker + Vec3::new(5.0, 30.0, 0.0)], false, false, ranks);
    assert_eq!(stream.current(), Some(GATE), "within 20 m on either axis keeps the queue");
    // The breaker's `unpause` within 25 m names it; the request releases the queued moment.
    assert_eq!(stream.unpause_target(marker + Vec3::new(24.0, -24.0, 9.0)), Some((GATE, marker)));
    assert_eq!(stream.unpause_target(marker + Vec3::new(26.0, 0.0, 0.0)), None);
    assert_eq!(stream.receive(GATE, HIGH, &[], false), Decision::Release);
    stream.started(Owner::World, GATE, HIGH, Some(marker));
    assert!(!stream.is_held());
    assert_eq!(stream.receive(GATE, HIGH, &[], false), Decision::Refuse, "two seconds before it may restart");
}

#[test]
fn leaving_a_queued_marker_drops_the_queue_without_a_voice_to_stop() {
    let marker = Vec3::ZERO;
    let mut stream = Arbiter::default();
    stream.set_markers(vec![(marker, GATE)]);
    stream.update(0.016, &[Vec3::new(10.0, 10.0, 0.0)], false, false, |_| Some(HIGH));
    assert!(stream.is_held());
    let tick = stream.update(0.016, &[Vec3::new(30.0, 30.0, 0.0)], false, false, |_| Some(HIGH));
    assert_eq!((tick.stop, stream.current()), (None, None));
}

#[test]
fn a_queued_moment_interrupts_a_playing_one_only_when_it_may_interrupt_and_ranks_as_high() {
    let marker = Vec3::ZERO;
    let near = [Vec3::new(5.0, 5.0, 0.0)];
    // The capture: a booth crash started next to a gate marker and the queue replaced it at once.
    let mut stream = playing(Owner::World, BOOTH, LOW);
    stream.set_markers(vec![(marker, GATE)]);
    stream.started(Owner::World, BOOTH, LOW, Some(Vec3::ZERO));
    let tick = stream.update(0.016, &near, true, false, |_| Some(HIGH));
    assert_eq!(tick, Tick { stop: Some(Owner::World), queued: Some(GATE) });
    let mut stream = Arbiter::default();
    stream.set_markers(vec![(marker, GATE)]);
    stream.started(Owner::World, BOOTH, LOW, Some(Vec3::ZERO));
    let polite = Rank { can_interrupt: false, ..HIGH };
    assert_eq!(stream.update(0.016, &near, true, false, |_| Some(polite)), Tick::default());
    let mut stream = Arbiter::default();
    stream.set_markers(vec![(marker, GATE)]);
    stream.started(Owner::Breaker, STING, Rank { priority: 9, can_interrupt: false }, None);
    assert_eq!(stream.update(0.016, &near, true, false, |_| Some(HIGH)), Tick::default(), "outranked");
}

#[test]
fn a_nearer_queued_marker_keeps_its_queue_against_a_farther_one() {
    let near_marker = Vec3::ZERO;
    let far_marker = Vec3::new(16.0, 0.0, 0.0);
    let other = vlt_hash("porch");
    let mut stream = Arbiter::default();
    stream.set_markers(vec![(near_marker, GATE), (far_marker, other)]);
    let player = [Vec3::new(2.0, 0.0, 0.0)];
    let tick = stream.update(0.016, &player, false, false, |_| Some(HIGH));
    assert_eq!(tick.queued, Some(GATE), "the farther marker does not take the queue from the nearer one");
    assert_eq!(stream.current(), Some(GATE));
}

#[test]
fn an_owner_that_stops_on_its_own_frees_only_its_own_moment() {
    let mut stream = playing(Owner::Breaker, STING, HIGH);
    stream.stopped(Owner::World);
    assert_eq!(stream.current(), Some(STING));
    stream.stopped(Owner::Breaker);
    assert_eq!(stream.current(), None);
}
