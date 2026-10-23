use super::*;
use crate::schema::InputReport;

/// The server's tick at client time `now_ms` when the server started 12.5 s before the client's clock origin.
fn truth(now_ms: f64) -> f64 {
    (now_ms + 12_500.0) * 60.0 / 1000.0
}

#[test]
fn pings_go_four_a_second_then_one_a_second() {
    let mut schedule = PingSchedule::default();
    let sent: Vec<u64> = (0..9000).filter(|now| schedule.due(*now).is_some()).collect();
    let fast = sent.iter().filter(|t| **t < FAST_PHASE_MS).count();
    assert_eq!(fast, 20);
    assert_eq!(&sent[..3], &[0, 250, 500]);
    assert!(sent.windows(2).filter(|w| w[0] >= FAST_PHASE_MS).all(|w| w[1] - w[0] == SLOW_PING_MS));
}

#[test]
fn ping_sequence_numbers_wrap() {
    let mut schedule = PingSchedule::default();
    let seqs: Vec<u8> = (0..300u64).filter_map(|i| schedule.due(i * 1000)).map(|p| p.seq).collect();
    assert_eq!(seqs[255], 255);
    assert_eq!(seqs[256], 0);
}

/// A client 40 ms from the server each way, the server holding each ping for 3 ms.
fn exchange(clock: &mut NetClock, sent_ms: u64) -> Option<f64> {
    let ping = Ping { seq: 0, client_time_ms: sent_ms as u32 };
    let recv = truth(sent_ms as f64 + 40.0);
    let send = truth(sent_ms as f64 + 43.0);
    let pong =
        Pong { seq: ping.seq, client_time_ms: ping.client_time_ms, server_recv_tick: recv, server_send_tick: send };
    clock.on_pong(&pong, sent_ms + 83)
}

#[test]
fn the_estimate_follows_the_server_and_the_rtt_leaves_out_the_server_time() {
    let mut clock = NetClock::new();
    assert_eq!(clock.update(0), ClockUpdate::Unsynced);
    assert_eq!(clock.server_tick(), None);
    let rtt = exchange(&mut clock, 100).unwrap();
    assert!((rtt - 80.0).abs() < 1.0e-6, "{rtt}");
    assert!(matches!(clock.update(200), ClockUpdate::Stepped(_)));
    let mut now = 200;
    for i in 0..600u64 {
        now += 16;
        if i % 15 == 0 {
            exchange(&mut clock, now);
        }
        assert_eq!(clock.update(now), ClockUpdate::Slewed);
        let error = clock.server_tick().unwrap() - truth(now as f64);
        assert!(error.abs() < 0.5, "{error} ticks off at {now}");
    }
    // The held time is taken out in whole milliseconds, half at each end: 83 - 2 × 1.
    assert_eq!(clock.best_rtt_ms(), Some(81.0));
}

#[test]
fn impossible_pongs_are_ignored() {
    let mut clock = NetClock::new();
    let backward = Pong { seq: 0, client_time_ms: 100, server_recv_tick: 10.0, server_send_tick: 9.0 };
    assert_eq!(clock.on_pong(&backward, 200), None);
    let nan = Pong { seq: 0, client_time_ms: 100, server_recv_tick: f64::NAN, server_send_tick: 9.0 };
    assert_eq!(clock.on_pong(&nan, 200), None);
    // Held longer than the whole round trip.
    let held = Pong { seq: 0, client_time_ms: 100, server_recv_tick: 0.0, server_send_tick: 60.0 };
    assert_eq!(clock.on_pong(&held, 200), None);
    // An echo from before the client's clock started.
    let future = Pong { seq: 0, client_time_ms: 500, server_recv_tick: 1.0, server_send_tick: 1.0 };
    assert_eq!(clock.on_pong(&future, 200), None);
    assert_eq!(clock.update(300), ClockUpdate::Unsynced);
}

#[test]
fn a_server_jump_steps_the_clock() {
    let mut clock = NetClock::new();
    exchange(&mut clock, 0);
    clock.update(100);
    // Ten samples from a server 30 ticks further on push the old ones out.
    for i in 0..10u64 {
        let sent = 1000 + i * 100;
        let pong = Pong {
            seq: 0,
            client_time_ms: sent as u32,
            server_recv_tick: truth(sent as f64 + 10.0) + 30.0,
            server_send_tick: truth(sent as f64 + 10.0) + 30.0,
        };
        clock.on_pong(&pong, sent + 20);
    }
    let ClockUpdate::Stepped(by) = clock.update(2000) else {
        panic!("expected a step");
    };
    assert!((by - 30.0).abs() < 1.0, "{by}");
}

fn report(on_time: u8, missing: u8, earliness: u8) -> InputReport {
    InputReport { early: 0, on_time, late: 0, missing, earliness }
}

#[test]
fn the_starting_lead_is_half_the_round_trip_plus_the_buffer() {
    assert_eq!(LeadControl::from_rtt(2.0, 1).ticks(), 2);
    assert_eq!(LeadControl::from_rtt(150.0, 2).ticks(), 7);
    assert_eq!(LeadControl::from_rtt(f64::INFINITY, 1).ticks(), crate::input::MAX_AHEAD_TICKS);
    assert_eq!(LeadControl::new(0).ticks(), 1);
}

#[test]
fn a_missing_input_raises_the_lead_at_most_every_250_ms() {
    let mut lead = LeadControl::new(2);
    assert_eq!(lead.on_report(&report(30, 2, 1), 0), 1);
    assert_eq!(lead.on_report(&report(30, 2, 1), 100), 0);
    assert_eq!(lead.on_report(&report(30, 0, 0), 250), 1);
    assert_eq!(lead.ticks(), 4);
    assert_eq!(lead.on_report(&report(32, 0, 2), 600), 0);
    assert_eq!(lead.predicted_tick(100.0), 104.0);
}

#[test]
fn high_earliness_for_two_seconds_lowers_the_lead() {
    let mut lead = LeadControl::new(8);
    let mut changes = Vec::new();
    for now in (0..=6000).step_by(50) {
        let change = lead.on_report(&report(32, 0, 5), now);
        if change != 0 {
            changes.push((now, change));
        }
    }
    assert_eq!(changes, vec![(2000, -1), (4000, -1), (6000, -1)]);
    // A dip below the high mark restarts the two seconds.
    let mut lead = LeadControl::new(8);
    lead.on_report(&report(32, 0, 5), 0);
    lead.on_report(&report(32, 0, 2), 1500);
    assert_eq!(lead.on_report(&report(32, 0, 5), 2100), 0);
    assert_eq!(lead.on_report(&report(32, 0, 5), 4100), -1);
}
