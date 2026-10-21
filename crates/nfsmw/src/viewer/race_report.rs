//! What a finished race tells the post-race screen (`PostRace_Results.fng`, `frontend/post_race.rs`), and what the
//! screen answers. Plain data: the world builds it, the front end shows it.

use blackbox_race::{RacerStats, StageMark};
use nfsmw_data::gameplay::RaceType;

/// One lap of a racer: its time and the place the racer held at the end of it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReportLap {
    /// Seconds the lap took.
    pub time: f32,
    /// 1 for first, among the racers that had crossed that lap's line by then.
    pub place: u32,
}

/// One racer's results.
#[derive(Debug, Clone, PartialEq)]
pub struct ReportRacer {
    pub name: String,
    /// The final place, 1 for first.
    pub place: u32,
    /// The finish time; `None` for a racer that did not finish.
    pub time: Option<f32>,
    pub laps: Vec<ReportLap>,
    /// The top speed, the stage marks and the drag marks of the run (`blackbox_race::RacerStats`).
    pub stats: RacerStats,
    /// The average speed over the run, m/s.
    pub average_speed: f32,
    /// Seconds behind the winner; `None` for the winner and for a racer that did not finish.
    pub time_behind: Option<f32>,
    /// How many times the racer ran into traffic.
    pub collisions: u32,
    /// The race time at each checkpoint the racer reached (the tollbooths of a tollbooth race).
    pub checkpoints: Vec<f32>,
    /// The racer was knocked out (a lap knockout, or totalled).
    pub knocked_out: bool,
    /// The speed through each speed trap, m/s, `None` for a trap not passed.
    pub traps: Vec<Option<f32>>,
}

impl ReportRacer {
    /// A racer with only a name, a place and a time: no laps, no statistics.
    pub fn bare(name: &str, place: u32, time: Option<f32>) -> Self {
        Self {
            name: name.to_owned(),
            place,
            time,
            laps: Vec::new(),
            stats: RacerStats::default(),
            average_speed: 0.0,
            time_behind: None,
            collisions: 0,
            checkpoints: Vec::new(),
            knocked_out: false,
            traps: Vec::new(),
        }
    }
}

/// Which group of the win photo (`InGamePhotoMaster.fng`) a result takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhotoKind {
    /// A sprint, circuit, knockout or speed-trap race: the time and the average speed (`SPRINT_GROUP`).
    Sprint,
    /// A drag race: the time and the speed at the line (`RIVAL_GROUP`).
    Drag,
    /// A tollbooth race: the time left, the bonus time and the speed (`TOLL_BOOTH_GROUP`).
    Tollbooth,
    /// A speed trap crossed in free roam: the speed and the bounty (`SPEEDTRAP_GROUP`).
    SpeedTrap,
}

/// What the win photo shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhotoReport {
    pub kind: PhotoKind,
    pub cash: u32,
    /// Seconds of the race.
    pub time: f32,
    /// The speed it shows, m/s.
    pub speed: f32,
    /// A tollbooth race: seconds left at the line, and the seconds the bonus checkpoints gave.
    pub time_left: f32,
    pub bonus_secs: f32,
    /// A speed trap: the bounty paid.
    pub bounty: u32,
}

/// The numbers of a pursuit challenge's chase, for `PostRace_Pursuit.fng`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PursuitReport {
    /// Seconds of the chase.
    pub secs: f32,
    /// Seconds the escape took; `None` when the chase did not end in one.
    pub evasion: Option<f32>,
    pub cops_damaged: u32,
    pub cops_destroyed: u32,
    pub roadblocks_dodged: u32,
    pub cost_to_state: u32,
    pub infractions: u32,
    /// The row the challenge asked for (`PursuitReport::ROWS` index) and the value it had to reach.
    pub goal: Option<(usize, f32)>,
}

impl PursuitReport {
    /// The rows of the screen, top to bottom.
    pub const ROWS: [&'static str; 6] =
        ["Length", "Escape Within", "Cops Damaged", "Cops Destroyed", "Roadblocks Dodged", "Cost to State Achieved"];

    /// A report for `nfsmw view-screen`.
    pub fn sample() -> Self {
        Self {
            secs: 95.5,
            evasion: Some(95.5),
            cops_damaged: 12,
            cops_destroyed: 3,
            roadblocks_dodged: 2,
            cost_to_state: 25_000,
            infractions: 4,
            goal: Some((2, 12.0)),
        }
    }
}

/// The finished race, the player first.
#[derive(Debug, Clone, PartialEq)]
pub struct RaceReport {
    /// The player first, then the opponents in grid order.
    pub racers: Vec<ReportRacer>,
    /// The kind of race: it picks the pages' rows (`frontend/post_race`).
    pub kind: RaceType,
    /// The laps of the course (1 for a sprint).
    pub laps: u32,
    /// The photo of a win; `None` for a race that was not won.
    pub photo: Option<PhotoReport>,
    /// A pursuit challenge's chase: it takes `PostRace_Pursuit.fng`, and `racers` is empty.
    pub pursuit: Option<PursuitReport>,
}

impl PhotoReport {
    /// A photo for `nfsmw view-screen`.
    pub fn sample() -> Self {
        Self {
            kind: PhotoKind::Sprint,
            cash: 1200,
            time: 80.5,
            speed: 70.0,
            time_left: 0.0,
            bonus_secs: 0.0,
            bounty: 0,
        }
    }
}

impl RaceReport {
    /// A report for `nfsmw view-screen`: a sprint of four racers, the numbers of the original's screenshots.
    pub fn sample() -> Self {
        let stage = |time: f32, place: u32| Some(StageMark { time, place });
        let racer = |name: &str, place: u32, time: f32, behind: Option<f32>| {
            let mut racer = ReportRacer::bare(name, place, Some(time));
            racer.stats.top_speed = 55.0;
            racer.stats.stages = [
                stage(time * 0.18, place),
                stage(time * 0.35, place),
                stage(time * 0.55, place),
                stage(time * 0.81, place),
            ];
            racer.average_speed = 35.3;
            racer.time_behind = behind;
            racer.collisions = place;
            racer
        };
        let mut player = racer("stock", 1, 172.51, None);
        player.stats.top_speed = 123.0 / 2.236_94;
        player.average_speed = 79.0 / 2.236_94;
        player.collisions = 3;
        player.stats.stages = [stage(30.36, 1), stage(59.92, 1), stage(95.19, 1), stage(140.29, 1)];
        Self {
            racers: vec![
                player,
                racer("Sully", 2, 175.20, Some(2.69)),
                racer("Rhino", 3, 181.90, Some(9.39)),
                racer("Dingo", 4, 190.40, Some(17.89)),
            ],
            kind: RaceType::Sprint,
            laps: 1,
            photo: Some(PhotoReport::sample()),
            pursuit: None,
        }
    }

    /// A circuit report: the player and three opponents over three laps.
    #[cfg(test)]
    pub fn sample_circuit() -> Self {
        let lap = |time: f32, place: u32| ReportLap { time, place };
        let racer = |name: &str, place: u32, laps: Vec<ReportLap>| {
            let mut racer = ReportRacer::bare(name, place, Some(laps.iter().map(|l| l.time).sum()));
            racer.laps = laps;
            racer.stats.top_speed = 60.0;
            racer.average_speed = 40.0;
            racer
        };
        Self {
            racers: vec![
                racer("stock", 2, vec![lap(61.2, 2), lap(59.8, 2), lap(60.1, 2)]),
                racer("Sully", 1, vec![lap(60.4, 1), lap(59.1, 1), lap(59.9, 1)]),
                racer("Rhino", 3, vec![lap(63.0, 3), lap(61.5, 3), lap(62.2, 3)]),
                racer("Dingo", 4, vec![lap(64.8, 4), lap(63.9, 4), lap(63.3, 4)]),
            ],
            kind: RaceType::Circuit,
            laps: 3,
            photo: Some(PhotoReport::sample()),
            pursuit: None,
        }
    }
}

/// What the post-race screen asks the race to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaceAnswer {
    /// Run the same race again.
    Restart,
    /// Leave the race and go on in free roam.
    Exit,
}

/// One milestone a finished chase banked, as the rewards screen lists it.
#[derive(Debug, Clone, PartialEq)]
pub struct RewardLine {
    /// What was achieved, for example `10 police cars destroyed`.
    pub text: String,
    /// The bounty it paid.
    pub bounty: u32,
    /// The texture of its icon in `PostRace_MilestoneRewards.fng`.
    pub icon: &'static str,
}

/// What `PostRace_MilestoneRewards.fng` shows after a chase that banked milestones.
#[derive(Debug, Clone, PartialEq)]
pub struct RewardsReport {
    pub lines: Vec<RewardLine>,
    /// The car's bounty after the rewards.
    pub total_bounty: u32,
}

impl RewardsReport {
    /// A report for `nfsmw view-screen`.
    pub fn sample() -> Self {
        Self {
            lines: vec![
                RewardLine { text: "10 police cars destroyed".into(), bounty: 5000, icon: "Cops_TakenOut_Icon" },
                RewardLine { text: "Chase for 05:00".into(), bounty: 5000, icon: "Milestone_Pursuit_MaxTime" },
            ],
            total_bounty: 1_000_000,
        }
    }
}
