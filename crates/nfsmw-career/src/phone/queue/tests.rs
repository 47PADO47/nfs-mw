use super::*;
use crate::phone::ids::is_known;
use crate::settings::SmsSlot;

fn inbox() -> SmsInbox {
    let mut inbox = SmsInbox::new();
    inbox.invalidate_missing(|i| is_known(u32::from(i)));
    inbox
}

fn only(mutate: impl Fn(&mut FlushContext)) -> FlushContext {
    let mut ctx = FlushContext::ALL_CLEAR;
    mutate(&mut ctx);
    ctx
}

#[test]
fn the_context_needs_all_five_conditions() {
    assert!(FlushContext::ALL_CLEAR.can_show());
    assert!(!FlushContext::default().can_show());
    assert!(!only(|c| c.free_roam = false).can_show());
    assert!(!only(|c| c.race_idle = false).can_show());
    assert!(!only(|c| c.overlay_allowed = false).can_show());
    assert!(!only(|c| c.speech_idle = false).can_show());
    assert!(!only(|c| c.player_car_ok = false).can_show());
}

#[test]
fn a_blocked_flush_leaves_the_queue_and_the_inbox_alone() {
    let mut inbox = inbox();
    let mut queue = PhoneQueue::new();
    queue.add_sms(&mut inbox, 17);
    for blocked in [
        only(|c| c.free_roam = false),
        only(|c| c.race_idle = false),
        only(|c| c.overlay_allowed = false),
        only(|c| c.speech_idle = false),
        only(|c| c.player_car_ok = false),
    ] {
        assert_eq!(queue.flush(&mut inbox, &blocked), None);
    }
    assert_eq!(queue.pending(), &[17]);
    assert_eq!(inbox.slots[17].state, 0);
    assert_eq!(inbox.counter, 0);
}

#[test]
fn an_empty_queue_flushes_to_nothing() {
    let mut inbox = inbox();
    assert_eq!(PhoneQueue::new().flush(&mut inbox, &FlushContext::ALL_CLEAR), None);
}

#[test]
fn queued_messages_arrive_together_in_queue_order_and_the_first_story_one_opens() {
    let mut inbox = inbox();
    let mut queue = PhoneQueue::new();
    assert_eq!(queue.add_sms(&mut inbox, 123), AddOutcome::Queued);
    assert_eq!(queue.add_sms(&mut inbox, 18), AddOutcome::Queued);
    assert_eq!(queue.add_sms(&mut inbox, 17), AddOutcome::Queued);
    let out = queue.flush(&mut inbox, &FlushContext::ALL_CLEAR).unwrap();
    assert_eq!(out.delivered, [123, 18, 17]);
    assert_eq!(out.open_at, 18, "the tip is skipped, the first story message opens");
    assert!(out.engage_flag);
    assert!(queue.pending().is_empty());
    assert_eq!(inbox.slots[123], SmsSlot { id: 123, state: 2, sequence: 1 });
    assert_eq!(inbox.slots[18].sequence, 2);
    assert_eq!(inbox.slots[17].sequence, 3, "the newest sorts first in the inbox");
    assert_eq!(inbox.counter, 3);
}

#[test]
fn a_batch_of_tips_opens_the_oldest() {
    let mut inbox = inbox();
    let mut queue = PhoneQueue::new();
    queue.add_sms(&mut inbox, 130);
    queue.add_sms(&mut inbox, 124);
    let out = queue.flush(&mut inbox, &FlushContext::ALL_CLEAR).unwrap();
    assert_eq!(out.open_at, 130);
    assert_eq!(out.delivered, [130, 124]);
}

#[test]
fn a_repeated_trigger_cannot_duplicate_a_message() {
    let mut inbox = inbox();
    let mut queue = PhoneQueue::new();
    assert_eq!(queue.add_sms(&mut inbox, 17), AddOutcome::Queued);
    assert_eq!(queue.add_sms(&mut inbox, 17), AddOutcome::AlreadyPending);
    assert_eq!(queue.pending(), &[17]);
    queue.flush(&mut inbox, &FlushContext::ALL_CLEAR);
    assert_eq!(queue.add_sms(&mut inbox, 17), AddOutcome::Ignored, "unread");
    inbox.open_message(17);
    assert_eq!(queue.add_sms(&mut inbox, 17), AddOutcome::Ignored, "read");
    assert!(queue.pending().is_empty());
    inbox.delete(17);
    assert_eq!(queue.add_sms(&mut inbox, 17), AddOutcome::Queued, "a deleted message can be sent again");
}

#[test]
fn immediate_ids_are_delivered_at_once_with_a_call() {
    let mut inbox = inbox();
    let mut queue = PhoneQueue::new();
    for id in [10, 11, 12, 13, 95, 96] {
        assert_eq!(queue.add_sms(&mut inbox, id), AddOutcome::Delivered { call: CellCallRequest { id } });
    }
    assert!(queue.pending().is_empty());
    assert_eq!(inbox.slots[10].sequence, 1);
    assert_eq!(inbox.slots[96].sequence, 6);
    assert_eq!(queue.add_sms(&mut inbox, 10), AddOutcome::Ignored, "already delivered");
    assert_eq!(inbox.counter, 6);
}

#[test]
fn an_immediate_id_with_a_missing_label_is_still_delivered_because_the_slot_exists() {
    let mut inbox = inbox();
    let mut queue = PhoneQueue::new();
    assert_eq!(inbox.slots[14].id, crate::settings::SMS_INVALID_ID);
    assert!(matches!(queue.add_sms(&mut inbox, 14), AddOutcome::Delivered { .. }));
}

#[test]
fn an_id_with_no_slot_is_queued_and_the_flush_skips_it() {
    let mut inbox = inbox();
    let mut queue = PhoneQueue::new();
    assert_eq!(queue.add_sms(&mut inbox, 200), AddOutcome::Queued);
    queue.add_sms(&mut inbox, 18);
    let out = queue.flush(&mut inbox, &FlushContext::ALL_CLEAR).unwrap();
    assert_eq!(out.delivered, [18]);
    assert_eq!(out.open_at, 18);
    assert_eq!(inbox.counter, 1);
}

#[test]
fn messages_queued_while_blocked_wait_without_a_timeout() {
    let mut inbox = inbox();
    let mut queue = PhoneQueue::new();
    queue.add_sms(&mut inbox, 18);
    let blocked = only(|c| c.race_idle = false);
    for _ in 0..1000 {
        assert_eq!(queue.flush(&mut inbox, &blocked), None);
    }
    assert!(queue.flush(&mut inbox, &FlushContext::ALL_CLEAR).is_some());
}

#[test]
fn text_languages_are_dutch_swedish_danish_polish_and_finnish() {
    let text: Vec<u32> = (0..20).filter(|&id| Language::from_language_id(id) == Language::Text).collect();
    assert_eq!(text, [5, 6, 7, 12, 13]);
}

#[test]
fn a_voice_language_call_speaks_and_never_touches_the_inbox() {
    let mut inbox = inbox();
    let out = process_cell_call(&mut inbox, 17, Language::Voice, false);
    assert_eq!(out, CellCallOutcome::Speak(VoiceTake { group: 15, line: 0 }));
    assert_eq!(inbox.slots[17].state, 0);
    assert_eq!(process_cell_call(&mut inbox, 120, Language::Voice, true), CellCallOutcome::Silent);
    assert_eq!(process_cell_call(&mut inbox, -1, Language::Voice, true), CellCallOutcome::Silent);
}

#[test]
fn a_text_language_call_marks_the_slot_delivered_without_a_sequence_and_opens_the_inbox() {
    let mut inbox = inbox();
    let out = process_cell_call(&mut inbox, 10, Language::Text, true);
    assert_eq!(out, CellCallOutcome::OpenInbox { at: Some(10) });
    assert_eq!(inbox.slots[10], SmsSlot { id: 10, state: 2, sequence: 0 });
    assert_eq!(inbox.counter, 0);
}

#[test]
fn a_text_language_call_that_cannot_open_is_lost() {
    let mut inbox = inbox();
    assert_eq!(process_cell_call(&mut inbox, 10, Language::Text, false), CellCallOutcome::Lost);
    assert_eq!(inbox.slots[10].state, 0);
}

#[test]
fn a_negative_call_id_opens_the_list() {
    let mut inbox = inbox();
    assert_eq!(process_cell_call(&mut inbox, -1, Language::Text, true), CellCallOutcome::OpenInbox { at: None });
    assert!(inbox.slots.iter().all(|s| s.state == 0));
}

#[test]
fn an_immediate_delivery_followed_by_its_text_call_keeps_the_sequence() {
    let mut inbox = inbox();
    let mut queue = PhoneQueue::new();
    let AddOutcome::Delivered { call } = queue.add_sms(&mut inbox, 95) else { panic!("not delivered") };
    process_cell_call(&mut inbox, call.id as i32, Language::Text, true);
    assert_eq!(inbox.slots[95], SmsSlot { id: 95, state: 2, sequence: 1 });
}
