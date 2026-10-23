use super::super::rate::*;

#[test]
fn a_bucket_starts_full_and_refills_exactly() {
    let mut bucket = RateLimit::new(LOBBY_RATE, 0);
    assert_eq!(bucket.tokens(0), 6);
    assert!((0..6).all(|_| bucket.allow(0)));
    assert!(!bucket.allow(0));
    assert!(!bucket.allow(499));
    assert!(bucket.allow(500));
    assert_eq!(bucket.tokens(100_000), 6, "never above the burst");
}

#[test]
fn fast_polling_loses_no_refill() {
    let mut bucket = RateLimit::new(VOTE_START_RATE, 0);
    assert!(bucket.allow(0));
    let passed = (1..=60_000).filter(|now| bucket.allow(*now)).count();
    assert_eq!(passed, 1, "one token after 60 s of calls every millisecond");
}

#[test]
fn a_clock_going_back_refills_nothing() {
    let mut bucket = RateLimit::new(CHAT_RATE, 10_000);
    assert!((0..3).all(|_| bucket.allow(10_000)));
    assert!(!bucket.allow(5_000));
    assert!(!bucket.allow(10_999));
    assert!(bucket.allow(11_000));
}

#[test]
fn a_zero_period_is_one_millisecond() {
    let mut bucket = RateLimit::new(RateSpec { per_token_ms: 0, burst: 1 }, 0);
    assert!(bucket.allow(0));
    assert!(!bucket.allow(0));
    assert!(bucket.allow(1));
}
