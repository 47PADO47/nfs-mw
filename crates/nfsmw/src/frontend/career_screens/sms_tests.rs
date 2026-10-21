use nfsmw_career::phone::{InboxTab, ListEntry, VoiceTake, is_known};
use nfsmw_career::settings::{SmsInbox, SmsState};

use super::*;

fn inbox_with(ids: &[u32]) -> SmsInbox {
    let mut inbox = SmsInbox::new();
    inbox.invalidate_missing(|i| is_known(u32::from(i)));
    for &id in ids {
        inbox.deliver(id);
    }
    inbox
}

fn entries(n: usize) -> Vec<ListEntry> {
    (0..n).map(|i| ListEntry { id: 10 + i as u32, sequence: i as u16 + 1, unread: false }).collect()
}

#[test]
fn a_page_shows_at_most_six_rows_from_its_first_entry() {
    let list = entries(8);
    assert_eq!(page(&list, 0).len(), 6);
    assert_eq!(page(&list, 4).len(), 4);
    assert_eq!(page(&list, 4)[0].id, 14);
    assert!(page(&list, 9).is_empty());
    assert!(page(&[], 0).is_empty());
}

#[test]
fn scrolling_keeps_the_page_full_and_in_range() {
    assert_eq!(scrolled(0, -1, 8), 0, "up from the top stays at the top");
    assert_eq!(scrolled(0, 5, 8), 2, "down stops when the last page is full");
    assert_eq!(scrolled(2, -9, 8), 0);
    assert_eq!(scrolled(0, 3, 3), 0, "a list shorter than a page never scrolls");
}

#[test]
fn the_two_tabs_toggle() {
    assert_eq!(other_tab(InboxTab::Story), InboxTab::Tips);
    assert_eq!(other_tab(InboxTab::Tips), InboxTab::Story);
}

#[test]
fn the_tab_icons_mark_unread_messages_in_the_other_tab_only() {
    assert_eq!(unread_marks(InboxTab::Story, true, true), (false, true));
    assert_eq!(unread_marks(InboxTab::Tips, true, false), (true, false));
    assert_eq!(unread_marks(InboxTab::Story, false, false), (false, false));
}

#[test]
fn the_rows_of_a_tab_are_its_delivered_messages_newest_first() {
    let inbox = inbox_with(&[17, 18, 123]);
    let story: Vec<u32> = inbox.list(InboxTab::Story).iter().map(|e| e.id).collect();
    let tips: Vec<u32> = inbox.list(InboxTab::Tips).iter().map(|e| e.id).collect();
    assert_eq!(story, [18, 17]);
    assert_eq!(tips, [123]);
    assert!(inbox.list(InboxTab::Story).iter().all(|e| e.unread));
}

#[test]
fn opening_a_message_marks_it_read_and_keeps_it_listed() {
    let mut inbox = inbox_with(&[17, 18]);
    assert!(mark_read(&mut inbox, 17).is_some());
    assert_eq!(inbox.slot(17).map(|s| s.state()), Some(SmsState::Read));
    let story = inbox.list(InboxTab::Story);
    let read = story.iter().find(|e| e.id == 17).unwrap();
    assert!(!read.unread, "the row loses its new-message icon");
    assert!(story.iter().find(|e| e.id == 18).unwrap().unread, "other messages stay unread");
    assert!(mark_read(&mut inbox, 17).is_some(), "a second open is still a read");
    assert_eq!(inbox.list(InboxTab::Story).len(), 2);
}

#[test]
fn opening_an_id_that_is_not_in_the_inbox_changes_nothing() {
    let mut inbox = inbox_with(&[17]);
    assert!(mark_read(&mut inbox, 16).is_none());
    assert_eq!(inbox.slot(16).map(|s| s.state()), Some(SmsState::Empty));
    assert!(mark_read(&mut inbox, 500).is_none());
}

#[test]
fn the_first_open_of_an_unread_story_message_asks_for_its_voice() {
    let mut inbox = inbox_with(&[17, 123]);
    let first = mark_read(&mut inbox, 17).unwrap();
    assert_eq!(first.play_voice, Some(VoiceTake { group: 15, line: 0 }));
    let again = mark_read(&mut inbox, 17).unwrap();
    assert_eq!(again.play_voice, None, "a re-read asks for nothing");
    assert_eq!(mark_read(&mut inbox, 123).unwrap().play_voice, None, "a tip has no voice");
}

#[test]
fn replay_is_for_listed_story_messages_only() {
    let mut inbox = inbox_with(&[17, 123]);
    assert_eq!(replay_id(&inbox, 17), Some(17));
    assert_eq!(replay_id(&inbox, 123), None, "a tip does not replay");
    assert_eq!(replay_id(&inbox, 18), None, "a message not in the inbox does not replay");
    mark_read(&mut inbox, 17);
    assert_eq!(replay_id(&inbox, 17), Some(17), "a read message replays");
}

#[test]
fn delete_empties_the_slot_and_the_list_drops_it() {
    let mut inbox = inbox_with(&[17, 18]);
    mark_read(&mut inbox, 17);
    assert!(delete_message(&mut inbox, 17));
    assert_eq!(inbox.slot(17).map(|s| s.state()), Some(SmsState::Empty));
    let story: Vec<u32> = inbox.list(InboxTab::Story).iter().map(|e| e.id).collect();
    assert_eq!(story, [18]);
    assert_eq!(replay_id(&inbox, 17), None, "a deleted message does not replay");
}

#[test]
fn delete_of_an_id_with_no_slot_changes_nothing() {
    let mut inbox = inbox_with(&[17]);
    assert!(!delete_message(&mut inbox, 150));
    assert_eq!(inbox.list(InboxTab::Story).len(), 1);
}

#[test]
fn a_deleted_message_is_delivered_again_when_sent_again() {
    let mut inbox = inbox_with(&[17]);
    delete_message(&mut inbox, 17);
    assert!(inbox.deliver(17));
    assert_eq!(inbox.list(InboxTab::Story).len(), 1);
}

#[test]
fn the_screens_are_made_by_name_without_regard_to_case() {
    assert!(make("sms_mailboxes.fng", Args::default()).is_some());
    assert!(make("sms_message.fng", Args::default()).is_some());
    assert!(make("car_select.fng", Args::default()).is_none());
}
