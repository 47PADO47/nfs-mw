use super::*;
use nfsmw_career::phone::is_known;

fn inbox() -> SmsInbox {
    let mut inbox = SmsInbox::new();
    inbox.invalidate_missing(|index| is_known(u32::from(index)));
    inbox
}

/// A frame with nothing going on: the car is ready and the world is free.
fn clear() -> PhoneFacts {
    PhoneFacts { car_ready: true, ..PhoneFacts::default() }
}

#[test]
fn a_clear_frame_lets_the_flush_run() {
    assert!(flush_context(&clear()).can_show());
}

#[test]
fn a_chase_holds_the_flush_back() {
    let facts = PhoneFacts { pursuing: true, ..clear() };
    assert!(!flush_context(&facts).can_show());
}

#[test]
fn a_zone_card_holds_the_flush_back() {
    let facts = PhoneFacts { card_up: true, ..clear() };
    let ctx = flush_context(&facts);
    assert!(!ctx.free_roam && !ctx.can_show());
}

#[test]
fn an_overlay_closes_the_overlay_test_and_the_flush_with_it() {
    let facts = PhoneFacts { overlay_blocked: true, ..clear() };
    let ctx = flush_context(&facts);
    assert!(!ctx.overlay_allowed && !ctx.can_show());
}

#[test]
fn a_car_still_loading_holds_the_flush_back() {
    let facts = PhoneFacts { car_ready: false, ..PhoneFacts::default() };
    assert!(!flush_context(&facts).can_show());
}

#[test]
fn a_queued_message_waits_until_the_world_is_clear_then_lands_unread() {
    let mut phone = PhoneState::default();
    let mut inbox = inbox();
    assert_eq!(phone.add_sms(&mut inbox, 15), AddOutcome::Queued);

    let chase = flush_context(&PhoneFacts { pursuing: true, ..clear() });
    assert!(phone.flush(&mut inbox, &chase).is_none());
    assert_eq!(phone.queue.pending(), &[15]);
    assert_eq!(inbox.slots[15].state, 0);

    let outcome = phone.flush(&mut inbox, &flush_context(&clear())).expect("the world is clear");
    assert_eq!(outcome.delivered, vec![15]);
    assert_eq!(outcome.open_at, 15);
    assert_eq!(inbox.slots[15].state, 2, "delivered and unread");
    assert!(phone.queue.pending().is_empty());
}

#[test]
fn a_repeated_send_queues_once_and_a_delivered_message_is_ignored() {
    let mut phone = PhoneState::default();
    let mut inbox = inbox();
    assert_eq!(phone.add_sms(&mut inbox, 15), AddOutcome::Queued);
    assert_eq!(phone.add_sms(&mut inbox, 15), AddOutcome::AlreadyPending);
    phone.flush(&mut inbox, &flush_context(&clear()));
    assert_eq!(phone.add_sms(&mut inbox, 15), AddOutcome::Ignored);
    assert!(phone.queue.pending().is_empty());
}

#[test]
fn an_immediate_id_is_delivered_at_once_and_its_call_waits_for_processing() {
    let mut phone = PhoneState::default();
    let mut inbox = inbox();
    let outcome = phone.add_sms(&mut inbox, 11);
    assert_eq!(outcome, AddOutcome::Delivered { call: CellCallRequest { id: 11 } });
    assert_eq!(inbox.slots[11].state, 2);
    assert!(phone.queue.pending().is_empty());
    assert_eq!(phone.take_calls(), vec![CellCallRequest { id: 11 }]);
    assert!(phone.take_calls().is_empty(), "the calls are taken once");
}

#[test]
fn queue_bin_messages_counts_only_the_newly_queued_ids() {
    let mut phone = PhoneState::default();
    let mut inbox = inbox();
    assert_eq!(phone.queue_bin_messages(&mut inbox, &[30, 31, 30]), 2);
    assert_eq!(phone.queue.pending(), &[30, 31]);
}

#[test]
fn queue_bin_messages_skips_what_the_inbox_already_has() {
    let mut phone = PhoneState::default();
    let mut inbox = inbox();
    inbox.deliver(30);
    assert_eq!(phone.queue_bin_messages(&mut inbox, &[30, 31]), 1);
    assert_eq!(phone.queue.pending(), &[31]);
}
