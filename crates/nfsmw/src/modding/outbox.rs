//! The mods' outbox: the chase snapshot the world publishes, the notices the mods hear and the requests they make of
//! the chase (ADR 0005 section 8, the `nfsmw.*` interfaces).
//!
//! The world and the mods share it without a reference to each other. The world publishes its snapshot in its police
//! update and applies the requests; the `ModsPost` system drains the notices and sends them. The state is per thread: the
//! scenes and the mods run on the main thread. Each queue is bounded, so a session without mods (no runtime drains them)
//! does not grow without limit; the oldest entries go first.

use std::cell::RefCell;

use super::pursuit_feed::{Notice, PursuitRequest, PursuitView, observe};

/// Most notices kept while nothing drains them.
const MAX_NOTICES: usize = 256;
/// Most requests kept while nothing applies them.
const MAX_REQUESTS: usize = 256;

#[derive(Default)]
struct Outbox {
    /// The last snapshot the world published; `None` before the first one.
    view: Option<PursuitView>,
    notices: Vec<Notice>,
    requests: Vec<PursuitRequest>,
}

thread_local! {
    static OUTBOX: RefCell<Outbox> = RefCell::new(Outbox::default());
}

/// Publish the chase as it is now. The notices of the change since the last snapshot are queued for the `ModsPost`
/// system. The first snapshot is compared with an idle chase, so it says what it finds.
pub fn publish(view: PursuitView) {
    OUTBOX.with(|outbox| {
        let mut outbox = outbox.borrow_mut();
        let before = outbox.view.unwrap_or_else(|| PursuitView::idle(view.fines));
        let changes = observe(&before, &view);
        outbox.view = Some(view);
        push_bounded(&mut outbox.notices, changes, MAX_NOTICES);
    });
}

/// The chase as the world last published it, or `None` before the first publish.
pub fn view() -> Option<PursuitView> {
    OUTBOX.with(|outbox| outbox.borrow().view)
}

/// Queue a change a mod asks of the chase. The world applies it in its next police update.
pub fn request(request: PursuitRequest) {
    OUTBOX.with(|outbox| push_bounded(&mut outbox.borrow_mut().requests, vec![request], MAX_REQUESTS));
}

/// The requests queued since the last call, in order.
pub fn take_requests() -> Vec<PursuitRequest> {
    OUTBOX.with(|outbox| std::mem::take(&mut outbox.borrow_mut().requests))
}

/// The notices queued since the last call, in order. The `ModsPost` system sends them to the mods.
pub fn drain_notices() -> Vec<Notice> {
    OUTBOX.with(|outbox| std::mem::take(&mut outbox.borrow_mut().notices))
}

/// Append `items` to `queue`, then drop the oldest entries beyond `limit`.
fn push_bounded<T>(queue: &mut Vec<T>, items: Vec<T>, limit: usize) {
    queue.extend(items);
    let excess = queue.len().saturating_sub(limit);
    queue.drain(..excess);
}

#[cfg(test)]
mod tests;
