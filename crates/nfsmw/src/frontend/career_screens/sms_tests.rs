use nfsmw_career::phone::{InboxTab, ListEntry, is_known};
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
    assert!(mark_read(&mut inbox, 17));
    assert_eq!(inbox.slot(17).map(|s| s.state()), Some(SmsState::Read));
    let story = inbox.list(InboxTab::Story);
    let read = story.iter().find(|e| e.id == 17).unwrap();
    assert!(!read.unread, "the row loses its new-message icon");
    assert!(story.iter().find(|e| e.id == 18).unwrap().unread, "other messages stay unread");
    assert!(mark_read(&mut inbox, 17), "a second open is still a read");
    assert_eq!(inbox.list(InboxTab::Story).len(), 2);
}

#[test]
fn opening_an_id_that_is_not_in_the_inbox_changes_nothing() {
    let mut inbox = inbox_with(&[17]);
    assert!(!mark_read(&mut inbox, 16));
    assert_eq!(inbox.slot(16).map(|s| s.state()), Some(SmsState::Empty));
    assert!(!mark_read(&mut inbox, 500));
}

#[test]
fn the_screens_are_made_by_name_without_regard_to_case() {
    assert!(make("sms_mailboxes.fng", Args::default()).is_some());
    assert!(make("sms_message.fng", Args::default()).is_some());
    assert!(make("car_select.fng", Args::default()).is_none());
}
