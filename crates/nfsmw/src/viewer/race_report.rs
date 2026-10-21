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

/// The backdrops of `GLOBAL/HUDTEXTURESPHOTOFINISH.BIN`: a polaroid on gravel and leaves whose picture is transparent, so
/// the world seen through it is the photo (`docs/specs/race-finish-effects.md` section 2). A race names one.
pub const PHOTO_PICTURES: [&str; 4] =
    ["PHOTOFINISH_SPRINT", "PHOTOFINISH_RIVAL", "PHOTOFINISH_DRAG", "PHOTOFINISH_TOOBOOTH"];

impl PhotoKind {
    /// The backdrop the kind takes when its race names none the pack has.
    pub fn default_picture(self) -> Option<&'static str> {
        match self {
            PhotoKind::Sprint => Some(PHOTO_PICTURES[0]),
            PhotoKind::Drag => Some(PHOTO_PICTURES[2]),
            PhotoKind::Tollbooth => Some(PHOTO_PICTURES[3]),
            PhotoKind::SpeedTrap => None,
        }
    }
}

/// The backdrop of a race's photo: the texture its data names when the pack has it (7 races of the install name
/// `PhotoFinish_Sprint.tga`, which it does not), else the kind's own.
pub fn photo_picture(named: Option<&str>, kind: PhotoKind) -> Option<&'static str> {
    let known = named.and_then(|name| PHOTO_PICTURES.iter().find(|known| known.eq_ignore_ascii_case(name.trim())));
    known.copied().or_else(|| kind.default_picture())
}

/// What the win photo shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhotoReport {
    pub kind: PhotoKind,
    /// The backdrop texture (one of [`PHOTO_PICTURES`]); `None` leaves the package's own picture hidden, as the speed
    /// trap's photo does.
    pub picture: Option<&'static str>,
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
    pub racers: Vec<ReportRacer>,
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
            picture: PhotoKind::Sprint.default_picture(),
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
