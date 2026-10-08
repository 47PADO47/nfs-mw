use crate::Timeline;

/// 30 fps keeps the arithmetic exact in tests.
fn timeline() -> Timeline {
    Timeline::new(30, 1, 90)
}

#[test]
fn frame_at_follows_the_clock() {
    let t = timeline();
    assert_eq!(t.frame_at(-0.1), None);
    assert_eq!(t.frame_at(0.0), Some(0));
    assert_eq!(t.frame_at(1.0 / 30.0 - 1e-6), Some(0));
    assert_eq!(t.frame_at(1.0), Some(30));
    assert_eq!(t.frame_at(2.99), Some(89));
    assert_eq!(t.frame_at(100.0), Some(89));
    assert_eq!(t.frame_at(f64::NAN), None);
}

#[test]
fn update_presents_each_frame_once() {
    let mut t = timeline();
    let u = t.update(0.0);
    assert_eq!((u.present, u.skipped), (Some(0), 0..0));
    let u = t.update(0.01);
    assert_eq!((u.present, u.skipped), (None, 1..1));
    let u = t.update(1.0 / 30.0);
    assert_eq!((u.present, u.skipped), (Some(1), 1..1));
    assert_eq!(t.next_frame(), 2);
}

#[test]
fn a_late_clock_skips_the_frames_in_between() {
    let mut t = timeline();
    t.update(0.0);
    let u = t.update(5.0 / 30.0 + 0.001);
    assert_eq!(u.present, Some(5));
    assert_eq!(u.skipped, 1..5);
    assert!(!u.finished);
}

#[test]
fn time_never_runs_backwards() {
    let mut t = timeline();
    t.update(1.0);
    let u = t.update(0.5);
    assert_eq!((u.present, u.skipped.is_empty()), (None, true));
    assert_eq!(t.next_frame(), 31);
}

#[test]
fn finishes_after_the_last_frame_time() {
    let mut t = timeline();
    let u = t.update(2.99);
    assert_eq!(u.present, Some(89));
    assert!(!u.finished);
    let u = t.update(3.0);
    assert_eq!(u.present, None);
    assert!(u.finished);
    assert_eq!(t.next_deadline(), None);
}

#[test]
fn next_deadline_is_the_next_frame_time() {
    let mut t = timeline();
    assert_eq!(t.next_deadline(), Some(0.0));
    t.update(0.0);
    assert!((t.next_deadline().unwrap() - 1.0 / 30.0).abs() < 1e-12);
}

#[test]
fn ntsc_rate_matches_the_header_values() {
    let mut t = Timeline::new(982_047, 32_767, 932);
    assert_eq!(t.frame_at(10.0), Some(299));
    t.update(10.0);
    assert_eq!(t.next_frame(), 300);
    assert!((t.frame_time(300) - 10.0100).abs() < 1e-3);
}

#[test]
fn empty_or_broken_timelines_never_present() {
    let mut t = Timeline::new(30, 1, 0);
    let u = t.update(1.0);
    assert_eq!((u.present, u.finished), (None, false));
    let mut t = Timeline::new(0, 1, 10);
    assert_eq!(t.frame_at(1.0), None);
    assert_eq!(t.update(1.0).present, None);
    assert_eq!(t.next_deadline(), None);
}
