//! The races' snapshot and the diff of two of them (`nfsmw.races`, `docs/modding/api-nfsmw.md`): what a race list, the
//! running race and the results are, and the notices the change from one snapshot to the next makes. Plain data, so
//! the rule is tested without a world; the mods' state (`modding/races_feed.rs`) publishes the snapshots and queues the
//! notices.

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
pub(crate) fn started(before: Option<Now>, after: Option<Now>, rows: &[RaceRow]) -> Option<Notice> {
    let now = after.filter(|now| before.is_none_or(|was| was.id != now.id))?;
    let row = rows.iter().find(|row| row.id == now.id);
    let (kind, racers, laps) = row.map_or((0, 0, 0), |row| (row.kind, row.racers, row.laps));
    Some(Notice::Started { race: now.id, kind, racers, laps })
}
