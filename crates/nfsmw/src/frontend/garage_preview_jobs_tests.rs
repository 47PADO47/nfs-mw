use super::*;
use std::sync::mpsc;
use std::time::{Duration, Instant};

fn completed<Q: Clone + Send + PartialEq + 'static, R: Send + 'static>(
    jobs: &mut Jobs<Q, R>,
) -> Option<(Q, Result<R>)> {
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(value) = jobs.poll(PartialEq::eq) {
            return Some(value);
        }
        if jobs.needs_start() || jobs.desired.is_none() {
            return None;
        }
        assert!(Instant::now() < until, "preview decoder must finish");
        std::thread::yield_now();
    }
}

#[test]
fn batch14_my_cars_decoder_drops_stale_results_and_coalesces_to_latest_request() {
    let mut jobs = Jobs::default();
    let (release, wait) = mpsc::channel();
    let (started, seen) = mpsc::channel();
    jobs.request(1);
    jobs.start(move |value| {
        started.send(value).unwrap();
        wait.recv().unwrap();
        Ok(value)
    })
    .unwrap();
    assert_eq!(seen.recv().unwrap(), 1);
    jobs.request(2);
    jobs.request(3);
    assert!(!jobs.needs_start(), "there is only one active decoder");
    release.send(()).unwrap();
    assert!(completed(&mut jobs).is_none(), "old completion must never publish");
    assert_eq!(jobs.desired, Some(3), "intermediate request is replaced");
    jobs.start(|value| Ok(value * 10)).unwrap();
    let (request, result) = completed(&mut jobs).unwrap();
    assert_eq!((request, result.unwrap()), (3, 30));
    assert!(jobs.desired.is_none());
}

#[test]
fn batch14_my_cars_cancel_does_not_wait_and_cannot_publish_late_errors() {
    let mut jobs = Jobs::<u8, u8>::default();
    let (release, wait) = mpsc::channel();
    jobs.request(1);
    jobs.start(move |_| {
        wait.recv().unwrap();
        anyhow::bail!("cancelled old decode")
    })
    .unwrap();
    jobs.cancel();
    assert!(jobs.desired.is_none());
    release.send(()).unwrap();
    // A new cache hit need not wait for the abandoned decoder. Its eventual error remains private.
    let until = Instant::now() + Duration::from_secs(5);
    while !jobs.active.as_ref().unwrap().1.is_finished() {
        assert!(Instant::now() < until, "cancelled decoder must finish");
        std::thread::yield_now();
    }
    assert!(jobs.poll(PartialEq::eq).is_none());
    assert!(jobs.active.is_none() && jobs.desired.is_none());
}
