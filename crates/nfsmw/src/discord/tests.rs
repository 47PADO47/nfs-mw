use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Instant;

use super::presence::{Allowed, LanGame, Presence, PresenceStatus, join_address, presence_for};
use super::worker::{Command, Link, RETRY_EVERY, Worker};
use super::{app_id_is_set, is_app_id};

const ALL: Allowed = Allowed { discord: true, time_played: true, join: true };

fn lan_game(open: bool) -> LanGame {
    LanGame { host: "192.168.1.20:9750".parse().unwrap(), players: 2, max_players: 8, open }
}

#[test]
fn the_presence_follows_the_discord_and_time_settings() {
    let off = Allowed { discord: false, ..ALL };
    assert_eq!(presence_for(PresenceStatus::Playing, None, off, 5), None, "off shows nothing");
    let plain = presence_for(PresenceStatus::Playing, None, Allowed { time_played: false, ..ALL }, 5).unwrap();
    assert_eq!((plain.details.as_str(), plain.started_at_ms), ("Playing NFS: Most Wanted", None));
    let timed = presence_for(PresenceStatus::Playing, None, ALL, 5).unwrap();
    assert_eq!(timed.started_at_ms, Some(5), "the session start is what Discord counts from");
}

#[test]
fn an_open_lan_game_is_a_party_with_a_join_secret() {
    let shown = presence_for(PresenceStatus::Lan, Some(&lan_game(true)), ALL, 5).unwrap();
    assert_eq!(shown.details, "In a LAN game");
    let party = shown.party.unwrap();
    assert_eq!((party.id.as_str(), party.size), ("nfsmw:192.168.1.20:9750", (2, 8)));
    assert_eq!(party.join.as_deref(), Some("192.168.1.20:9750"));
}

#[test]
fn a_closed_lan_game_is_a_party_nobody_can_join() {
    let shown = presence_for(PresenceStatus::Lan, Some(&lan_game(false)), ALL, 5).unwrap();
    assert_eq!(shown.party.unwrap().join, None, "a password or a full room offers no secret");
}

#[test]
fn without_the_join_setting_there_is_no_party() {
    let shown = presence_for(PresenceStatus::Lan, Some(&lan_game(true)), Allowed { join: false, ..ALL }, 5).unwrap();
    assert_eq!(shown.party, None);
}

#[test]
fn only_a_socket_address_is_a_join_secret() {
    assert_eq!(join_address("192.168.1.20:9750"), Some("192.168.1.20:9750".parse().unwrap()));
    assert_eq!(join_address(" [::1]:9750 "), Some("[::1]:9750".parse().unwrap()));
    for secret in ["", "192.168.1.20", "example.com:9750", "192.168.1.20:99999", "1.2.3.4:9750 extra"] {
        assert_eq!(join_address(secret), None, "{secret:?}");
    }
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
    answers: Vec<(u64, bool)>,
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

    fn answer_join_request(&mut self, user: u64, accept: bool) -> Result<(), String> {
        self.0.borrow_mut().answers.push((user, accept));
        Ok(())
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
    Command::Show(presence_for(PresenceStatus::Playing, None, Allowed { time_played, ..ALL }, 1_000).unwrap())
}

fn shown_in_lan(open: bool) -> Command {
    Command::Show(presence_for(PresenceStatus::Lan, Some(&lan_game(open)), ALL, 1_000).unwrap())
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
fn hiding_closes_the_connection_and_a_later_show_connects_again() {
    let record: Shared = Default::default();
    let mut worker = worker(&record);
    let now = Instant::now();
    worker.apply(shown(true));
    worker.sync(now);
    worker.apply(Command::Hide);
    worker.sync(now);
    assert_eq!(record.borrow().closes, 1);
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
fn the_run_loop_closes_the_connection_when_the_game_lets_go() {
    let record: Shared = Default::default();
    let (sender, commands) = mpsc::channel();
    sender.send(shown(true)).unwrap();
    drop(sender);
    worker(&record).run(commands);
    let record = record.borrow();
    assert_eq!((record.sets.len(), record.closes), (1, 1));
}

#[test]
fn the_run_loop_stops_on_quit_even_while_senders_remain() {
    let record: Shared = Default::default();
    let (sender, commands) = mpsc::channel();
    sender.send(shown(true)).unwrap();
    sender.send(Command::Quit).unwrap();
    worker(&record).run(commands);
    assert_eq!(record.borrow().closes, 1, "the link's own senders would keep the channel open");
}

#[test]
fn a_join_request_is_accepted_while_the_party_offers_a_secret() {
    let record: Shared = Default::default();
    let mut worker = worker(&record);
    let now = Instant::now();
    worker.apply(shown_in_lan(true));
    worker.sync(now);
    worker.apply(Command::JoinRequest(42));
    worker.sync(now);
    assert_eq!(record.borrow().answers, vec![(42, true)]);
}

#[test]
fn a_join_request_is_declined_when_the_game_is_closed_or_not_shown() {
    let record: Shared = Default::default();
    let mut worker = worker(&record);
    let now = Instant::now();
    worker.apply(shown_in_lan(false));
    worker.sync(now);
    worker.apply(Command::JoinRequest(7));
    worker.sync(now);
    assert_eq!(record.borrow().answers, vec![(7, false)], "a full or locked room");
    worker.apply(Command::Hide);
    worker.apply(Command::JoinRequest(8));
    worker.sync(now);
    assert_eq!(record.borrow().answers.len(), 1, "nothing is answered while nothing is shown");
    worker.apply(shown_in_lan(true));
    worker.sync(now);
    assert_eq!(record.borrow().answers.len(), 1, "a request from before is not answered later");
}
