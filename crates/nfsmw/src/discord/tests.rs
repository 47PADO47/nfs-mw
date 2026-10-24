use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Instant;

use super::presence::{Presence, PresenceStatus, presence_for};
use super::worker::{Command, Link, RETRY_EVERY, Worker};
use super::{app_id_is_set, is_app_id};

#[test]
fn the_presence_follows_the_discord_and_time_settings() {
    assert_eq!(presence_for(PresenceStatus::Playing, false, true, 5), None, "off shows nothing");
    let plain = presence_for(PresenceStatus::Playing, true, false, 5).unwrap();
    assert_eq!((plain.details.as_str(), plain.started_at_ms), ("Playing NFS: Most Wanted", None));
    let timed = presence_for(PresenceStatus::Playing, true, true, 5).unwrap();
    assert_eq!(timed.started_at_ms, Some(5), "the session start is what Discord counts from");
}

#[test]
fn only_a_numeric_application_id_counts_as_set() {
    assert!(is_app_id("1234567890"));
    assert!(!is_app_id(""));
    assert!(!is_app_id("PLACEHOLDER_DISCORD_APP_ID"));
    assert!(!is_app_id("123 456"));
    assert_eq!(app_id_is_set(), is_app_id(super::DISCORD_APP_ID));
}

/// What the fake Discord client was asked to do.
#[derive(Default)]
struct Record {
    opens: u32,
    sets: Vec<Presence>,
    clears: u32,
    closes: u32,
    refuse_connect: bool,
    refuse_set: bool,
}

type Shared = Rc<RefCell<Record>>;

struct Fake(Shared);

impl Link for Fake {
    fn connect(&mut self) -> Result<(), String> {
        if self.0.borrow().refuse_connect {
            return Err("no Discord client".into());
        }
        Ok(())
    }

    fn set_activity(&mut self, presence: &Presence) -> Result<(), String> {
        let mut record = self.0.borrow_mut();
        if record.refuse_set {
            return Err("the pipe closed".into());
        }
        record.sets.push(presence.clone());
        Ok(())
    }

    fn clear_activity(&mut self) {
        self.0.borrow_mut().clears += 1;
    }

    fn close(&mut self) {
        self.0.borrow_mut().closes += 1;
    }
}

/// A worker whose links all report to `record`.
fn worker(record: &Shared) -> Worker<Fake, impl FnMut() -> Fake> {
    let record = Rc::clone(record);
    Worker::new(move || {
        record.borrow_mut().opens += 1;
        Fake(Rc::clone(&record))
    })
}

fn shown(time_played: bool) -> Command {
    Command::Show(presence_for(PresenceStatus::Playing, true, time_played, 1_000).unwrap())
}

#[test]
fn a_shown_presence_is_sent_once_and_again_only_when_it_changes() {
    let record: Shared = Default::default();
    let mut worker = worker(&record);
    let now = Instant::now();
    worker.apply(shown(true));
    worker.sync(now);
    worker.apply(shown(true));
    worker.sync(now);
    assert_eq!((record.borrow().opens, record.borrow().sets.len()), (1, 1));
    worker.apply(shown(false));
    worker.sync(now);
    let record = record.borrow();
    assert_eq!(record.sets.len(), 2, "the time played was switched off");
    assert_eq!(record.sets[1].started_at_ms, None);
    assert!(worker.next_wait(now).is_none(), "connected: nothing to retry");
}

#[test]
fn hiding_clears_the_activity_and_a_later_show_connects_again() {
    let record: Shared = Default::default();
    let mut worker = worker(&record);
    let now = Instant::now();
    worker.apply(shown(true));
    worker.sync(now);
    worker.apply(Command::Hide);
    worker.sync(now);
    assert_eq!((record.borrow().clears, record.borrow().closes), (1, 1));
    worker.apply(shown(true));
    worker.sync(now);
    assert_eq!((record.borrow().opens, record.borrow().sets.len()), (2, 2));
}

#[test]
fn without_discord_the_attempts_are_paced_and_nothing_is_set() {
    let record: Shared = Default::default();
    record.borrow_mut().refuse_connect = true;
    let mut worker = worker(&record);
    let start = Instant::now();
    worker.apply(shown(true));
    worker.sync(start);
    assert_eq!(record.borrow().opens, 1);
    assert_eq!(worker.next_wait(start), Some(RETRY_EVERY));
    worker.sync(start + RETRY_EVERY / 2);
    assert_eq!(record.borrow().opens, 1, "no attempt before the pace allows one");
    worker.sync(start + RETRY_EVERY);
    assert_eq!(record.borrow().opens, 2);
    record.borrow_mut().refuse_connect = false;
    worker.sync(start + RETRY_EVERY * 2);
    assert_eq!(record.borrow().sets.len(), 1, "Discord appeared: the presence is shown");
}

#[test]
fn a_failed_set_drops_the_connection_and_reconnects_later() {
    let record: Shared = Default::default();
    let mut worker = worker(&record);
    let start = Instant::now();
    worker.apply(shown(true));
    worker.sync(start);
    record.borrow_mut().refuse_set = true;
    worker.apply(shown(false));
    worker.sync(start);
    assert_eq!(record.borrow().closes, 1, "the dropped connection is closed");
    assert_eq!(worker.next_wait(start), Some(RETRY_EVERY));
    record.borrow_mut().refuse_set = false;
    worker.sync(start + RETRY_EVERY);
    assert_eq!((record.borrow().opens, record.borrow().sets.len()), (2, 2));
}

#[test]
fn the_run_loop_clears_the_activity_when_the_game_lets_go() {
    let record: Shared = Default::default();
    let (sender, commands) = mpsc::channel();
    sender.send(shown(true)).unwrap();
    drop(sender);
    worker(&record).run(commands);
    let record = record.borrow();
    assert_eq!((record.sets.len(), record.clears, record.closes), (1, 1, 1));
}
