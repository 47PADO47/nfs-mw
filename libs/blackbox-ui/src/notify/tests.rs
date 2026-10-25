use super::*;

/// A notice of `key` at `priority`, up for `duration` seconds, folding repeats within 2 s.
fn notice(key: &'static str, priority: Priority, duration: f32) -> Notice<&'static str, u32> {
    Notice { key, payload: 0, priority, duration, dedupe: Dedupe { window: 2.0 }, max_wait: None }
}

fn keys(queue: &NotifyQueue<&'static str, u32>) -> Vec<&'static str> {
    queue.visible().iter().map(|active| active.key).collect()
}

#[test]
fn a_notice_shows_at_once_with_room_and_goes_at_the_end_of_its_duration() {
    let mut queue = NotifyQueue::new();
    queue.push(notice("a", Priority::Medium, 3.0));
    assert_eq!(keys(&queue), ["a"]);
    assert_eq!(queue.visible()[0].started, 0.0);
    queue.tick(2.9, false);
    assert_eq!(keys(&queue), ["a"]);
    queue.tick(0.2, false);
    assert!(queue.visible().is_empty(), "expired");
}

#[test]
fn at_most_three_are_visible_and_the_rest_wait_most_urgent_first() {
    let mut queue = NotifyQueue::new();
    for key in ["a", "b", "c"] {
        queue.push(notice(key, Priority::Medium, 1.0));
    }
    queue.push(notice("low", Priority::Low, 5.0));
    queue.push(notice("medium", Priority::Medium, 5.0));
    assert_eq!(keys(&queue).len(), CAPACITY);
    assert_eq!(queue.waiting(), 2, "a Medium does not preempt");
    queue.tick(1.5, false);
    assert_eq!(keys(&queue), ["medium", "low"], "the more urgent waiting notice comes up first");
    assert_eq!(queue.visible()[0].started, 1.5, "its animation starts when it comes up");
}

#[test]
fn critical_and_high_evict_the_least_urgent_visible_notice() {
    let mut queue = NotifyQueue::new();
    queue.push(notice("medium", Priority::Medium, 10.0));
    queue.push(notice("background", Priority::Background, 10.0));
    queue.push(notice("low", Priority::Low, 10.0));
    queue.push(notice("critical", Priority::Critical, 10.0));
    assert_eq!(keys(&queue), ["medium", "low", "critical"], "the Background one is gone");
    queue.push(notice("high", Priority::High, 10.0));
    assert_eq!(keys(&queue), ["medium", "critical", "high"]);
    assert_eq!(queue.waiting(), 0, "an evicted notice is dropped, not queued");
}

#[test]
fn a_high_notice_does_not_evict_one_as_urgent_as_itself() {
    let mut queue = NotifyQueue::new();
    for key in ["a", "b", "c"] {
        queue.push(notice(key, Priority::Critical, 10.0));
    }
    queue.push(notice("high", Priority::High, 10.0));
    assert_eq!(keys(&queue), ["a", "b", "c"]);
    assert_eq!(queue.waiting(), 1, "it waits for room");
}

#[test]
fn among_equally_urgent_notices_the_oldest_is_evicted() {
    let mut queue = NotifyQueue::new();
    queue.push(notice("old", Priority::Low, 10.0));
    queue.tick(1.0, false);
    queue.push(notice("new", Priority::Low, 10.0));
    queue.push(notice("medium", Priority::Medium, 10.0));
    queue.push(notice("critical", Priority::Critical, 10.0));
    assert_eq!(keys(&queue), ["new", "medium", "critical"]);
}

#[test]
fn a_repeat_refreshes_the_visible_notice_without_restarting_it() {
    let mut queue = NotifyQueue::new();
    queue.push(notice("near-miss", Priority::Low, 2.0));
    queue.tick(1.5, false);
    queue.push(Notice { payload: 7, ..notice("near-miss", Priority::Low, 2.0) });
    let active = &queue.visible()[0];
    assert_eq!(queue.visible().len(), 1, "one notice, not two");
    assert_eq!(active.started, 0.0, "the animation does not restart");
    assert_eq!(active.expires, 3.5, "the expiry moves out");
    assert_eq!((active.count, active.payload), (2, 7), "the repeat is counted and its payload kept");
    queue.tick(1.9, false);
    assert_eq!(queue.visible().len(), 1, "still up past its first expiry");
}

#[test]
fn a_repeat_of_a_notice_that_has_gone_is_not_shown_again_within_the_window() {
    let mut queue = NotifyQueue::new();
    queue.push(notice("a", Priority::Medium, 0.5));
    queue.tick(1.0, false);
    assert!(queue.visible().is_empty());
    queue.push(notice("a", Priority::Medium, 0.5));
    assert!(queue.visible().is_empty() && queue.waiting() == 0, "1 s after the first push, inside the 2 s window");
    queue.tick(1.5, false);
    queue.push(notice("a", Priority::Medium, 0.5));
    assert!(queue.visible().is_empty(), "the window runs from the last push, so repeats keep it back");
    queue.tick(2.5, false);
    queue.push(notice("a", Priority::Medium, 0.5));
    assert_eq!(keys(&queue), ["a"], "past the window it is a new notice");
    assert_eq!(queue.visible()[0].started, 5.0);
}

#[test]
fn a_repeat_of_a_waiting_notice_is_folded_into_it() {
    let mut queue = NotifyQueue::new();
    queue.tick(0.1, true);
    queue.push(notice("unlock", Priority::Background, 3.0));
    queue.push(Notice { payload: 9, ..notice("unlock", Priority::Background, 3.0) });
    assert_eq!(queue.waiting(), 1);
    queue.tick(0.1, false);
    assert_eq!(keys(&queue), ["unlock"]);
    assert_eq!((queue.visible()[0].count, queue.visible()[0].payload), (2, 9));
}

#[test]
fn low_and_background_wait_while_the_game_is_intense() {
    let mut queue = NotifyQueue::new();
    queue.tick(0.1, true);
    queue.push(notice("low", Priority::Low, 3.0));
    queue.push(notice("background", Priority::Background, 3.0));
    queue.push(notice("medium", Priority::Medium, 3.0));
    assert_eq!(keys(&queue), ["medium"], "with room, the Low and Background notices still wait");
    queue.tick(1.0, true);
    assert_eq!(keys(&queue), ["medium"]);
    queue.tick(1.0, false);
    assert_eq!(keys(&queue), ["medium", "low", "background"], "they come up once it calms down");
}

#[test]
fn a_waiting_notice_with_a_max_wait_goes_stale() {
    let mut queue = NotifyQueue::new();
    queue.tick(0.1, true);
    queue.push(Notice { max_wait: Some(2.0), ..notice("stale", Priority::Low, 3.0) });
    queue.tick(2.5, true);
    queue.tick(0.1, false);
    assert!(queue.visible().is_empty() && queue.waiting() == 0);
}

#[test]
fn an_infinite_duration_stays_until_dismissed() {
    let mut queue = NotifyQueue::new();
    queue.push(notice("busted", Priority::Critical, f32::INFINITY));
    queue.tick(1000.0, false);
    assert_eq!(keys(&queue), ["busted"]);
    assert!(queue.dismiss(&"busted"));
    assert!(queue.visible().is_empty());
    assert!(!queue.dismiss(&"busted"), "already gone");
}

#[test]
fn dismissing_makes_room_for_a_waiting_notice() {
    let mut queue = NotifyQueue::new();
    for key in ["a", "b", "c", "d"] {
        queue.push(notice(key, Priority::Medium, 10.0));
    }
    assert!(queue.dismiss(&"b"));
    queue.tick(0.1, false);
    assert_eq!(keys(&queue), ["a", "c", "d"]);
    assert!(queue.dismiss(&"d"));
    assert_eq!(keys(&queue), ["a", "c"]);
}

#[test]
fn a_bad_frame_time_does_not_move_the_clock() {
    let mut queue: NotifyQueue<&'static str, u32> = NotifyQueue::new();
    queue.tick(f32::NAN, false);
    queue.tick(-1.0, false);
    assert_eq!(queue.clock(), 0.0);
}

#[test]
fn clear_forgets_everything() {
    let mut queue = NotifyQueue::with_capacity(1);
    queue.push(notice("a", Priority::Medium, 10.0));
    queue.push(notice("b", Priority::Medium, 10.0));
    queue.clear();
    assert!(queue.visible().is_empty() && queue.waiting() == 0);
    queue.push(notice("a", Priority::Medium, 10.0));
    assert_eq!(keys(&queue), ["a"], "the dedupe window was forgotten too");
}
