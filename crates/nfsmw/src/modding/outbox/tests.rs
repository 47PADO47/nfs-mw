//! The outbox: snapshots become notices once, requests come back in order, and the queues stay bounded.

use blackbox_pursuit::{EndReason, Fines};

use super::{MAX_NOTICES, drain_notices, publish, take_requests, view};
use crate::modding::pursuit_feed::{Notice, PursuitRequest, PursuitView};

fn fines() -> Fines {
    Fines::new([100, 110, 120, 130, 140, 150, 160, 170])
}

fn chase(heat: f32) -> PursuitView {
    PursuitView { pursuing: true, heat, level: heat as u32, ..PursuitView::idle(fines()) }
}

#[test]
fn a_started_chase_is_drained_once() {
    publish(PursuitView::idle(fines()));
    publish(chase(2.5));
    assert_eq!(drain_notices(), [Notice::PursuitStarted { level: 2, cops: 0, heat: 2.5 }]);
    assert!(drain_notices().is_empty(), "a drained notice is not sent again");
}

#[test]
fn the_view_is_the_last_snapshot() {
    publish(chase(3.0));
    assert_eq!(view().map(|v| v.heat), Some(3.0));
    drain_notices();
}

#[test]
fn requests_come_back_in_order_and_once() {
    crate::modding::outbox::request(PursuitRequest::SetHeat(5.0));
    crate::modding::outbox::request(PursuitRequest::SetHeat(6.0));
    assert_eq!(take_requests(), [PursuitRequest::SetHeat(5.0), PursuitRequest::SetHeat(6.0)]);
    assert!(take_requests().is_empty());
}

#[test]
fn an_undrained_queue_keeps_only_its_newest_notices() {
    publish(PursuitView::idle(fines()));
    // Each pass starts or ends the chase, so each pass queues exactly one notice.
    for step in 0..(MAX_NOTICES + 10) {
        publish(PursuitView { pursuing: step % 2 == 0, end_reason: EndReason::None, ..chase(1.0) });
    }
    assert_eq!(drain_notices().len(), MAX_NOTICES, "the queue is bounded");
}
