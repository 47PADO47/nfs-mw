//! The races as the mods see them (`nfsmw.races`, `docs/modding/api-nfsmw.md`): the snapshot the world publishes each
//! frame, the changes the mods ask for, and the races the mods registered.
//!
//! As the pursuit's outbox is, it is a per-thread state the world and the mods share without a reference to each other:
//! the scenes and the mods run on the main thread. The world publishes the list, the running race and the results, and
//! applies the requests and the new races in its race update (`scenes/world/race/mods.rs`).

use std::cell::RefCell;

/// The id of the first race a mod registers. The install's races take their index in the world's list as their id, so a
/// mod's race never collides with one of them.
pub const CUSTOM_BASE: u32 = 0x1000_0000;
/// Most requests kept while the world is not applying them.
const MAX_REQUESTS: usize = 64;
/// Most notices kept while nothing drains them.
const MAX_NOTICES: usize = 64;

/// One race of the list.
#[derive(Debug, Clone, PartialEq)]
pub struct RaceRow {
    pub id: u32,
    pub name: String,
    /// The `RaceType` number (`nfsmw_data::gameplay::RaceType`).
    pub kind: u32,
    pub laps: u32,
    pub racers: u32,
    /// The race's start, render space, metres.
    pub start: [f32; 3],
    /// The checkpoints and the finish, render space, in running order.
    pub checkpoints: Vec<[f32; 3]>,
    /// The player has finished it this session.
    pub done: bool,
}

/// The race that runs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Now {
    pub id: u32,
    pub lap: u32,
    pub checkpoint: u32,
    pub position: u32,
    pub elapsed: f32,
}

/// How a race ended for the player.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Outcome {
    pub position: u32,
    pub time: f32,
    pub won: bool,
    pub winnings: u32,
}

/// What the world publishes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Snapshot {
    pub rows: Vec<RaceRow>,
    pub now: Option<Now>,
    /// The results by race id.
    pub results: Vec<(u32, Outcome)>,
}

/// A change of the races the mods hear: what a new snapshot says against the one before it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Notice {
    /// A race began.
    Started { race: u32, kind: u32, racers: u32, laps: u32 },
    /// The player finished a race (a new result, or a better or worse one than the race had).
    Finished { race: u32, outcome: Outcome },
}

/// The notices of the change from `before` to `after`, in order: a race that began, then the results that are new.
pub fn observe(before: &Snapshot, after: &Snapshot) -> Vec<Notice> {
    let mut notices = Vec::new();
    notices.extend(started(before.now, after.now, &after.rows));
    for (race, outcome) in &after.results {
        let known = before.results.iter().any(|(id, was)| id == race && was == outcome);
        if !known {
            notices.push(Notice::Finished { race: *race, outcome: *outcome });
        }
    }
    notices
}

/// The notice of a race that began between two frames, with the row of the list for what it was.
fn started(before: Option<Now>, after: Option<Now>, rows: &[RaceRow]) -> Option<Notice> {
    let now = after.filter(|now| before.is_none_or(|was| was.id != now.id))?;
    let row = rows.iter().find(|row| row.id == now.id);
    let (kind, racers, laps) = row.map_or((0, 0, 0), |row| (row.kind, row.racers, row.laps));
    Some(Notice::Started { race: now.id, kind, racers, laps })
}

/// What a mod asks of the races.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Request {
    Start(u32),
    Abort,
    /// The opponents' skill of a race, 0 to 1.
    Skill(u32, f32),
    /// The cash a win of a race pays.
    Reward(u32, u32),
}

/// A race a mod registered.
#[derive(Debug, Clone, PartialEq)]
pub struct Custom {
    pub id: u32,
    pub key: String,
    pub name: String,
    pub kind: u32,
    pub laps: u32,
    pub reward: u32,
    /// Opponents on the grid besides the player.
    pub opponents: u32,
    pub start: [f32; 3],
    /// Radians from +x towards +y.
    pub heading: f32,
    pub checkpoints: Vec<[f32; 3]>,
}

#[derive(Default)]
struct State {
    snapshot: Snapshot,
    requests: Vec<Request>,
    notices: Vec<Notice>,
    /// Every race registered, in order.
    customs: Vec<Custom>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

/// Publish the races as they are now.
pub fn publish(snapshot: Snapshot) {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let changes = observe(&state.snapshot, &snapshot);
        state.snapshot = snapshot;
        state.notices.extend(changes);
        // Nothing drains them in a session without mods: the oldest go first.
        let excess = state.notices.len().saturating_sub(MAX_NOTICES);
        state.notices.drain(..excess);
    });
}

/// Publish the race that runs, the rest of the snapshot as it was: the list and the results change only when a race
/// is registered or finished, so a frame that has nothing new does not build them again.
pub fn publish_now(now: Option<Now>) {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let began = started(state.snapshot.now, now, &state.snapshot.rows);
        state.snapshot.now = now;
        state.notices.extend(began);
        let excess = state.notices.len().saturating_sub(MAX_NOTICES);
        state.notices.drain(..excess);
    });
}

/// The notices queued since the last call, in order. The `ModsPost` system sends them to the mods.
pub fn drain_notices() -> Vec<Notice> {
    STATE.with(|state| std::mem::take(&mut state.borrow_mut().notices))
}

/// Look at the last snapshot. Nothing was published before the first world frame, so the lists are empty then.
pub fn with_snapshot<R>(read: impl FnOnce(&Snapshot) -> R) -> R {
    STATE.with(|state| read(&state.borrow().snapshot))
}

/// Queue a change a mod asks for. The world applies it in its next race update.
pub fn request(request: Request) {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        if state.requests.len() >= MAX_REQUESTS {
            state.requests.remove(0);
        }
        state.requests.push(request);
    });
}

/// The requests queued since the last call, in order.
pub fn take_requests() -> Vec<Request> {
    STATE.with(|state| std::mem::take(&mut state.borrow_mut().requests))
}

/// Register a race; its id is given here, so the mod has it at once. `None` when a race with the same key exists.
pub fn register(mut custom: Custom) -> Option<u32> {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        if state.customs.iter().any(|c| c.key == custom.key) {
            return None;
        }
        custom.id = CUSTOM_BASE + state.customs.len() as u32;
        let id = custom.id;
        // The list shows the race at once; the world's next snapshot keeps it.
        state.snapshot.rows.push(RaceRow {
            id,
            name: custom.name.clone(),
            kind: custom.kind,
            laps: custom.laps,
            racers: 1 + custom.opponents,
            start: custom.start,
            checkpoints: custom.checkpoints.clone(),
            done: false,
        });
        state.customs.push(custom);
        Some(id)
    })
}

/// The races registered after the first `seen` of them: a world asks with the count it has, so a world opened later still
/// gets every race the mods registered.
pub fn customs_after(seen: usize) -> Vec<Custom> {
    STATE.with(|state| state.borrow().customs.get(seen..).unwrap_or_default().to_vec())
}

/// Forget everything (a test).
#[cfg(test)]
pub fn clear() {
    STATE.with(|state| *state.borrow_mut() = State::default());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn custom(key: &str) -> Custom {
        Custom {
            id: 0,
            key: key.into(),
            name: key.into(),
            kind: 0,
            laps: 1,
            reward: 0,
            opponents: 0,
            start: [0.0; 3],
            heading: 0.0,
            checkpoints: vec![[1.0, 0.0, 0.0]],
        }
    }

    #[test]
    fn a_registered_race_gets_the_next_custom_id_and_a_key_only_once() {
        clear();
        assert_eq!(register(custom("m:a")), Some(CUSTOM_BASE));
        assert_eq!(register(custom("m:b")), Some(CUSTOM_BASE + 1));
        assert_eq!(register(custom("m:a")), None);
        assert_eq!(customs_after(0).len(), 2);
        assert!(customs_after(2).is_empty(), "a world that has both asks for none");
        assert_eq!(register(custom("m:c")), Some(CUSTOM_BASE + 2));
        assert_eq!(customs_after(2)[0].key, "m:c");
        assert_eq!(customs_after(0).len(), 3, "a world opened later gets them all");
    }

    fn row(id: u32) -> RaceRow {
        RaceRow {
            id,
            name: String::new(),
            kind: 2,
            laps: 3,
            racers: 4,
            start: [0.0; 3],
            checkpoints: Vec::new(),
            done: false,
        }
    }

    fn now(id: u32) -> Now {
        Now { id, lap: 1, checkpoint: 0, position: 1, elapsed: 0.0 }
    }

    #[test]
    fn a_race_that_begins_and_a_new_result_are_noticed_once() {
        clear();
        let idle = Snapshot { rows: vec![row(7)], ..Snapshot::default() };
        let running = Snapshot { now: Some(now(7)), ..idle.clone() };
        let outcome = Outcome { position: 1, time: 80.0, won: true, winnings: 500 };
        let done = Snapshot { results: vec![(7, outcome)], ..idle.clone() };
        publish(idle);
        assert!(drain_notices().is_empty());
        publish(running.clone());
        publish(running);
        assert_eq!(drain_notices(), vec![Notice::Started { race: 7, kind: 2, racers: 4, laps: 3 }]);
        publish(done.clone());
        publish(done);
        assert_eq!(drain_notices(), vec![Notice::Finished { race: 7, outcome }]);
    }

    #[test]
    fn publishing_only_the_running_race_notices_a_start_once_and_keeps_the_list() {
        clear();
        publish(Snapshot { rows: vec![row(7)], ..Snapshot::default() });
        publish_now(Some(now(7)));
        publish_now(Some(now(7)));
        assert_eq!(drain_notices(), vec![Notice::Started { race: 7, kind: 2, racers: 4, laps: 3 }]);
        publish_now(None);
        publish_now(Some(now(7)));
        assert_eq!(drain_notices().len(), 1, "the same race starting again is a new start");
        assert_eq!(STATE.with(|state| state.borrow().snapshot.rows.len()), 1, "the list stays");
    }

    #[test]
    fn requests_come_back_in_order_and_are_bounded() {
        clear();
        request(Request::Start(3));
        request(Request::Abort);
        assert_eq!(take_requests(), vec![Request::Start(3), Request::Abort]);
        for i in 0..100 {
            request(Request::Reward(i, 1));
        }
        assert_eq!(take_requests().len(), MAX_REQUESTS);
    }
}
