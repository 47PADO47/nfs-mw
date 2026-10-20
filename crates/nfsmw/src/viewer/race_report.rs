//! What a finished race tells the post-race screen (`PostRace_Results.fng`, `frontend/post_race.rs`), and what the
//! screen answers. Plain data: the world builds it, the front end shows it.

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
}

/// The finished race, the player first.
#[derive(Debug, Clone, PartialEq)]
pub struct RaceReport {
    pub racers: Vec<ReportRacer>,
}

impl RaceReport {
    /// A report for `nfsmw view-screen`: the player and three opponents over three laps.
    pub fn sample() -> Self {
        let lap = |time: f32, place: u32| ReportLap { time, place };
        let racer = |name: &str, place: u32, laps: Vec<ReportLap>| ReportRacer {
            name: name.to_owned(),
            place,
            time: Some(laps.iter().map(|l| l.time).sum()),
            laps,
        };
        Self {
            racers: vec![
                racer("PLAYER", 2, vec![lap(61.2, 2), lap(59.8, 2), lap(60.1, 2)]),
                racer("RACER 1", 1, vec![lap(60.4, 1), lap(59.1, 1), lap(59.9, 1)]),
                racer("RACER 2", 3, vec![lap(63.0, 3), lap(61.5, 3), lap(62.2, 3)]),
                racer("RACER 3", 4, vec![lap(64.8, 4), lap(63.9, 4), lap(63.3, 4)]),
            ],
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
