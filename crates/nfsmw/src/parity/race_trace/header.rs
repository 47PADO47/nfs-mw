//! The race trace's `header` line: the run, the event, the rules the race runs under and the racers on the grid. Every
//! value is taken from the running race when the line is written, at the green light, so two traces that report the same
//! identity raced the same set-up.

use serde::Serialize;

use super::super::header::RunHeader;
use super::super::race_run::Autopilot;

/// The rule set a race runs under.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// A quick race: the quick race's skill table, catch-up by difficulty, no career result.
    Quick,
    /// A career race: the characters' own skills and the career's catch-up.
    Career,
}

impl Mode {
    pub fn of(career: bool) -> Self {
        match career {
            true => Self::Career,
            false => Self::Quick,
        }
    }
}

/// A quick race's difficulty: the `DifficultyLevel` the race runs with and its class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Difficulty {
    /// 0 to 100; the quick race writes 33, 66 or 100.
    pub level: i32,
    /// `easy` below 34, `medium` to 66, `hard` from 67.
    pub name: &'static str,
}

impl Difficulty {
    /// The difficulty of `level`, named by its class (0 easy, 1 medium, 2 hard).
    pub fn new(level: i32, class: usize) -> Self {
        let name = match class {
            0 => "easy",
            1 => "medium",
            _ => "hard",
        };
        Self { level, name }
    }
}

/// The traffic of the race as the world runs it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Traffic {
    /// The event's traffic density, percent (0 in a drag race).
    pub level: u32,
    /// The event's traffic pattern, `drag` for example.
    pub pattern: Option<String>,
    /// The most traffic cars the world keeps on the road.
    pub cars: usize,
}

/// The build that made the trace: its Git commit, when Git was there at compile time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Build {
    pub git_commit: Option<&'static str>,
    /// The working tree had changes to tracked files when the build script last ran (`build.rs`).
    pub git_dirty: Option<bool>,
}

impl Build {
    /// This binary's build.
    pub fn current() -> Self {
        Self {
            git_commit: option_env!("NFSMW_BUILD_REVISION"),
            git_dirty: option_env!("NFSMW_BUILD_DIRTY").map(|dirty| dirty == "1"),
        }
    }
}

/// One racer on the grid.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RacerRow {
    pub racer: usize,
    pub name: String,
    /// The car: the player's car folder, an opponent's `pvehicle` name.
    pub car: Option<String>,
    /// The race counts this racer as a person: the race is over when every such racer is done.
    pub human: bool,
    /// The base skill its AI driver uses, 0 to 1; the catch-up offsets it tick by tick (`sample`). The player's is the
    /// autopilot's; `null` when no AI driver drives the racer.
    pub skill: Option<f32>,
    /// Where the racer stands at the green light, in the frame of `sample.position`.
    pub grid: Option<[f32; 3]>,
}

/// What a race's first line says about it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Header {
    pub schema: &'static str,
    /// Unix time of the line, milliseconds.
    pub created_unix_ms: u64,
    pub run: RunHeader,
    pub build: Build,
    /// The career event id, for example `16.1.1`.
    pub event: String,
    /// The race's collection name.
    pub race: String,
    pub race_type: String,
    pub mode: Mode,
    /// The quick-race difficulty in effect; `null` in a career race, which does not use it.
    pub difficulty: Option<Difficulty>,
    /// The catch-up (rubber band) is on.
    pub catch_up: bool,
    pub traffic: Traffic,
    /// Police may enter the race.
    pub cops: bool,
    /// The opponents that stand on the grid.
    pub opponents: usize,
    pub laps: u32,
    /// The checkpoints of one lap, without the finish line.
    pub checkpoints_per_lap: usize,
    /// The race has a finish line besides its checkpoints; crossing it closes a lap and raises no `checkpoint` line.
    pub finish_line: bool,
    /// No countdown: the timer starts when the racers are placed.
    pub rolling_start: bool,
    pub racers: Vec<RacerRow>,
    /// The autopilot of the run; `null` when a person (or nobody) drives the player's car.
    pub autopilot: Option<AutopilotRow>,
}

/// The autopilot's options.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct AutopilotRow {
    pub after_secs: f32,
    pub skill: f32,
}

impl From<Autopilot> for AutopilotRow {
    fn from(autopilot: Autopilot) -> Self {
        Self { after_secs: autopilot.after_secs, skill: autopilot.skill }
    }
}
