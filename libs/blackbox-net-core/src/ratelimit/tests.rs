use std::net::{IpAddr, Ipv4Addr};

use super::{FailureLimiter, FailureRule, KeyedLimiter, PeerBudget, PreAuthGate, PreAuthLimits, TokenBucket};

fn ip(last: u8) -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(192, 168, 1, last))
}

#[test]
fn a_bucket_spends_its_burst_then_refills_exactly() {
    let mut bucket = TokenBucket::new(5, 5, 10_000);
    for _ in 0..5 {
        assert!(bucket.try_take(0, 1));
    }
    assert!(!bucket.try_take(0, 1), "the burst is spent");
    assert!(!bucket.try_take(1_999, 1), "less than one token has come back");
    assert!(bucket.try_take(2_000, 1), "one token per 2 s");
    assert!(!bucket.try_take(2_000, 1));
    assert_eq!(bucket.available(12_000), 5, "never above the burst");
}

#[test]
fn a_bucket_ignores_a_clock_that_goes_backwards() {
    let mut bucket = TokenBucket::new(1, 1, 1000);
    assert!(bucket.try_take(5_000, 1));
    assert!(!bucket.try_take(1_000, 1), "time going back refills nothing");
    assert!(bucket.try_take(6_000, 1));
}

#[test]
fn would_allow_takes_nothing() {
    let bucket = TokenBucket::new(1, 1, 1000);
    assert!(bucket.would_allow(0, 1));
    assert!(bucket.would_allow(0, 1));
    assert!(!bucket.would_allow(0, 2));
}

#[test]
fn keyed_buckets_are_independent_and_bounded() {
    let mut limiter = KeyedLimiter::new(2, 2, 1000, 2);
    assert!(limiter.try_take(0, ip(1), 1));
    assert!(limiter.try_take(0, ip(1), 1));
    assert!(!limiter.try_take(0, ip(1), 1));
    assert!(limiter.try_take(0, ip(2), 1), "another key has its own bucket");
    assert!(!limiter.try_take(0, ip(3), 1), "the table is full of active keys: fail closed");
    assert!(!limiter.would_allow(0, ip(3), 1));
    assert!(limiter.try_take(1_000, ip(3), 1), "refilled buckets are forgotten to make room");
    assert!(limiter.tracked() <= 2);
}

#[test]
fn password_guesses_lock_with_a_doubling_ceiling() {
    let mut guesses = FailureLimiter::new(FailureRule::PASSWORD, 16);
    let key = 7u32;
    for _ in 0..4 {
        assert!(!guesses.record_failure(0, key));
    }
    assert!(!guesses.is_locked(0, &key));
    assert!(guesses.record_failure(0, key), "the fifth wrong guess locks");
    assert!(guesses.is_locked(59_999, &key));
    assert!(!guesses.is_locked(60_000, &key), "the first lock is 60 s");

    let mut now = 60_000;
    let mut expected = 120_000;
    for _ in 0..6 {
        for _ in 0..5 {
            guesses.record_failure(now, key);
        }
        assert!(guesses.is_locked(now + expected - 1, &key));
        assert!(!guesses.is_locked(now + expected, &key));
        now += expected;
        expected = (expected * 2).min(15 * 60_000);
    }
    assert_eq!(expected, 15 * 60_000, "the doubling stops at 15 minutes");
    assert!(!guesses.is_locked(now, &8), "other keys are not locked: no lockout of the whole room");
    guesses.record_success(&key);
    assert_eq!(guesses.tracked(), 0);
}

#[test]
fn failures_outside_the_window_do_not_add_up() {
    let mut auth = FailureLimiter::new(FailureRule::AUTH, 16);
    for i in 0..30u64 {
        assert!(!auth.record_failure(i * 7_000, ip(1)), "9 per minute never locks");
    }
    assert!(!auth.is_locked(210_000, &ip(1)));
}

#[test]
fn a_full_failure_table_fails_closed_for_new_keys() {
    let mut limiter = FailureLimiter::new(FailureRule::PASSWORD, 1);
    limiter.record_failure(0, 1u8);
    assert!(limiter.is_locked(0, &2u8), "no room to track a new key");
    assert!(!limiter.is_locked(60_000, &2u8), "the old entry is stale after its window");
}

#[test]
fn the_gate_caps_handshakes_per_address() {
    let mut gate = PreAuthGate::new(PreAuthLimits::LAN);
    for i in 0..5 {
        assert!(gate.handshake_allowed(i * 100, ip(1), 0), "handshake {i}");
    }
    assert!(!gate.handshake_allowed(600, ip(1), 0), "5 per 10 s per address");
    assert!(gate.handshake_allowed(1_200, ip(2), 0), "another address is fine");
    assert!(gate.handshake_allowed(12_000, ip(1), 0), "and the address recovers");
    assert!(!gate.handshake_allowed(13_000, ip(3), 64), "the half-open cap holds");
}

#[test]
fn the_gate_sheds_hellos_under_load() {
    let mut gate = PreAuthGate::new(PreAuthLimits::LAN);
    assert!(gate.hello_allowed(0, ip(1), 0));
    assert!(!gate.hello_allowed(0, ip(1), 33), "over 32 half-open: no Challenge");
    for last in 0..8 {
        assert!(gate.handshake_allowed(0, ip(10 + last), 0));
    }
    assert!(!gate.hello_allowed(0, ip(1), 0), "8 new handshakes in the second: no Challenge");
    assert!(gate.hello_allowed(1_000, ip(1), 0));
}

#[test]
fn the_gate_counts_connections_per_address_and_in_total() {
    let mut gate = PreAuthGate::new(PreAuthLimits::LAN);
    for _ in 0..4 {
        gate.connection_opened(ip(1));
    }
    assert!(!gate.handshake_allowed(0, ip(1), 0), "4 connections per address on a LAN");
    gate.connection_closed(ip(1));
    assert!(gate.handshake_allowed(0, ip(1), 0));
    gate.connection_closed(ip(9));
    assert!(!gate.is_full(20), "3 open plus 20 half-open is under 24");
    assert!(gate.is_full(21));
}

#[test]
fn auth_failures_make_an_address_ignored() {
    let mut gate = PreAuthGate::new(PreAuthLimits::LAN);
    for i in 0..10 {
        gate.auth_failed(i, ip(1));
    }
    assert!(gate.is_ignored(10, ip(1)));
    assert!(!gate.hello_allowed(10, ip(1), 0));
    assert!(!gate.handshake_allowed(10, ip(1), 0));
    assert!(!gate.is_ignored(10 + 5 * 60_000, ip(1)), "ignored for 5 minutes");
}

#[test]
fn discovery_replies_are_capped_in_total_and_per_source() {
    let mut gate = PreAuthGate::new(PreAuthLimits::LAN);
    assert!(gate.discovery_reply_allowed(0, ip(1)));
    assert!(gate.discovery_reply_allowed(0, ip(1)));
    assert!(!gate.discovery_reply_allowed(0, ip(1)), "2 per second per source");
    let mut granted = 2;
    for last in 2..40 {
        granted += u32::from(gate.discovery_reply_allowed(0, ip(last)));
    }
    assert_eq!(granted, 10, "10 per second in total");
    assert!(gate.discovery_reply_allowed(1_000, ip(1)));
}

#[test]
fn a_peer_budget_caps_bytes_and_datagrams_per_tick() {
    let limits = PreAuthLimits::LAN;
    let mut budget = PeerBudget::new(&limits);
    for _ in 0..128 {
        assert!(budget.accept(0, 100));
    }
    assert!(!budget.accept(0, 100), "128 datagrams per tick");
    budget.start_tick();
    assert!(budget.accept(0, 100));

    let mut flood = PeerBudget::new(&limits);
    let mut bytes = 0usize;
    for _ in 0..2_000 {
        flood.start_tick();
        if flood.accept(0, 1200) {
            bytes += 1200;
        }
    }
    assert!(bytes <= 128 * 1024, "at most 128 KiB in the first second, got {bytes}");
}
