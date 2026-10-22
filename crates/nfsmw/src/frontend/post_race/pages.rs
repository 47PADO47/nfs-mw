//! What the three pages of the post-race screen say: standings, statistics and stages, as rows of three cells for
//! the racer shown. Pure data and text rules, so the screen's logic only copies them into the package's strings.
//! Spec: `docs/specs/race-results.md`; the original's rows: `docs/provenance/race-results-screens.md`.

use nfsmw_data::gameplay::RaceType;

use crate::settings::SpeedUnits;
use crate::viewer::{RaceReport, ReportRacer};

/// The page the screen shows. Key 2 walks them in this order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Standings,
    Statistics,
    Stages,
}

impl Page {
    /// The page the second button shows next.
    pub fn next(self) -> Self {
        match self {
            Page::Standings => Page::Statistics,
            Page::Statistics => Page::Stages,
            Page::Stages => Page::Standings,
        }
    }

    /// The page the back button returns to: the one the second button came from.
    pub fn previous(self) -> Self {
        match self {
            Page::Standings => Page::Stages,
            Page::Statistics => Page::Standings,
            Page::Stages => Page::Statistics,
        }
    }
}

/// One cell of a row.
#[derive(Clone, Debug, PartialEq)]
pub enum Cell {
    Empty,
    Text(String),
    /// A string of the language table.
    Label(u32),
    /// A number and a unit label of the language table (`123 MPH`).
    WithUnit(String, u32),
}

/// A page ready to show.
#[derive(Clone, Debug, PartialEq)]
pub struct Table {
    /// The language labels of the title, of the footer's second button and of the column headers (none: hidden).
    pub title: u32,
    pub footer: u32,
    pub headers: [Option<u32>; 3],
    pub rows: Vec<[Cell; 3]>,
    /// The racer's name between the two arrows is shown (the statistics and the stages).
    pub filter: bool,
}

/// Language labels the pages use (`nfsmw strings 0xHASH` shows the text).
mod label {
    pub const STANDINGS_TITLE: u32 = 0xFF11_5FFF;
    pub const STATISTICS_TITLE: u32 = 0x4E70_6980;
    pub const STAGES_TITLE: u32 = 0x34BA_50FF;
    pub const LAPS_TITLE: u32 = 0x9C8D_7FE8;
    pub const TOLLBOOTHS_TITLE: u32 = 0xD10A_8EA2;
    pub const SPEEDTRAPS_TITLE: u32 = 0xECD0_E6A6;
    pub const STATISTICS_BUTTON: u32 = 0xD0B8_AA33;
    pub const STAGES_BUTTON: u32 = 0x1135_F776;
    pub const LAPS_BUTTON: u32 = 0x7B8F_45DF;
    pub const TOLLBOOTHS_BUTTON: u32 = 0xAC23_368C;
    pub const SPEEDTRAPS_BUTTON: u32 = 0xAEF5_1E9D;
    pub const STANDINGS_BUTTON: u32 = 0x8159_A0B2;
    pub const POSITION: u32 = 0x96B0_5F47;
    pub const NAME: u32 = 0xCE67_8AD3;
    pub const TIME: u32 = 0xB67D_A102;
    pub const SPEED: u32 = 0x7540_FB04;
    pub const STAGE: u32 = 0xE8B7_D527;
    pub const LAP: u32 = 0x0000_BF9C;
    pub const TOLLBOOTH: u32 = 0xA15E_4505;
    pub const SPEEDTRAP: u32 = 0xEE1E_DC76;
    pub const NONE: u32 = 0x0FC1_BF40;
    pub const KNOCKED_OUT: u32 = 0x5D82_DBA2;
    pub const MPH: u32 = 0x8569_AB44;
    pub const KMH: u32 = 0x8569_A25F;
    pub const TOP_SPEED: u32 = 0x0EF3_4382;
    pub const AVERAGE_SPEED: u32 = 0x57F4_140A;
    pub const TIME_BEHIND: u32 = 0xAB44_ED8B;
    pub const LAP_VARIANCE: u32 = 0x4121_E8C4;
    pub const COLLISIONS: u32 = 0x094B_FDFC;
    pub const PERFECT_SHIFTS: u32 = 0x680A_C597;
    pub const ZERO_TO_SIXTY: u32 = 0xCCBC_22B3;
    pub const ZERO_TO_HUNDRED: u32 = 0xB8CF_16FC;
    pub const QUARTER_MILE: u32 = 0x49FD_5DCB;
    pub const FOUR_HUNDRED_M: u32 = 0x1C6F_2A82;
    pub const ACCUMULATED_SPEED: u32 = 0xD57E_02AB;
    pub const SPEED_VARIANCE: u32 = 0x6EEA_BE8C;
    pub const SPEED_BEHIND: u32 = 0x2E54_B7ED;
}

/// How the race's rows are laid out: the original's mode (0 sprint, 1 circuit, 2 drag, 3 knockout, 4 tollbooth,
/// 5 speed trap) in four families.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Family {
    Sprint,
    Laps,
    Drag,
    Toll,
    Trap,
}

fn family(kind: RaceType) -> Family {
    match kind {
        RaceType::Circuit | RaceType::Knockout => Family::Laps,
        RaceType::Drag => Family::Drag,
        RaceType::Tollbooth => Family::Toll,
        RaceType::SpeedTrap => Family::Trap,
        _ => Family::Sprint,
    }
}

/// A race time as the game's text shows it: `30.36`, `1:35.19`, `2:52.51`, `1:02:03` past an hour. The original counts
/// 4000 ticks to the second and cuts, never rounds, the hundredths.
pub fn race_time(secs: f32) -> String {
    let ticks = (secs.max(0.0) * 4000.0 + 0.5) as u64;
    let whole = ticks / 4000;
    let hundredths = (ticks % 4000) / 4 / 10;
    let (hours, minutes, seconds) = (whole / 3600, (whole / 60) % 60, whole % 60);
    if hours > 0 {
        return format!("{hours}:{minutes:02}:{seconds:02}");
    }
    if minutes > 0 {
        return format!("{minutes}:{seconds:02}.{hundredths:02}");
    }
    format!("{seconds}.{hundredths:02}")
}

/// A speed in m/s as the number and unit the pages show: the original multiplies by 2.23699 for miles per hour and by
/// 1.60931 more for kilometres.
pub fn speed_cell(mps: f32, units: SpeedUnits) -> Cell {
    let mph = mps * 2.236_99;
    match units {
        SpeedUnits::Imperial => Cell::WithUnit(format!("{mph:.0}"), label::MPH),
        SpeedUnits::Metric => Cell::WithUnit(format!("{:.0}", mph * 1.609_31), label::KMH),
    }
}

/// The time of a cell, or the original's dashes for none.
fn time_cell(time: Option<f32>) -> Cell {
    time.map_or(Cell::Label(label::NONE), |t| Cell::Text(race_time(t)))
}

/// The racers by final place, ties by grid order.
pub fn by_place(report: &RaceReport) -> Vec<&ReportRacer> {
    let mut racers: Vec<&ReportRacer> = report.racers.iter().collect();
    racers.sort_by_key(|r| r.place);
    racers
}

/// The page of `report` for the racer at `shown` (the standings ignore it).
pub fn table(report: &RaceReport, page: Page, shown: usize, units: SpeedUnits) -> Table {
    let family = family(report.kind);
    let racer = report.racers.get(shown);
    match page {
        Page::Standings => standings(report, family, units),
        Page::Statistics => statistics(report, family, racer, units),
        Page::Stages => stages(report, family, racer, units),
    }
}

fn button(page: Page, family: Family) -> u32 {
    match (page, family) {
        (Page::Standings, _) => label::STATISTICS_BUTTON,
        (Page::Statistics, Family::Sprint | Family::Drag) => label::STAGES_BUTTON,
        (Page::Statistics, Family::Laps) => label::LAPS_BUTTON,
        (Page::Statistics, Family::Toll) => label::TOLLBOOTHS_BUTTON,
        (Page::Statistics, Family::Trap) => label::SPEEDTRAPS_BUTTON,
        (Page::Stages, _) => label::STANDINGS_BUTTON,
    }
}

/// The rows the standings page has (`PostRace_Results.fng` has ten **[verified]**).
pub const STANDINGS_ROWS: usize = 10;

/// The standings' racers: the first [`STANDINGS_ROWS`] by place. A player placed below them keeps the last row, so the
/// page always has the player's own place (`racers[0]` is the player).
fn standings_racers(report: &RaceReport) -> Vec<&ReportRacer> {
    let placed = by_place(report);
    let mut shown: Vec<&ReportRacer> = placed.iter().take(STANDINGS_ROWS).copied().collect();
    let Some(player) = report.racers.first() else { return shown };
    let below = placed.iter().position(|r| std::ptr::eq(*r, player)).is_some_and(|at| at >= STANDINGS_ROWS);
    if below {
        shown.truncate(STANDINGS_ROWS - 1);
        shown.push(player);
    }
    shown
}

fn standings(report: &RaceReport, family: Family, units: SpeedUnits) -> Table {
    let third = if family == Family::Trap { label::SPEED } else { label::TIME };
    let rows = standings_racers(report)
        .into_iter()
        .map(|racer| {
            let last = match family {
                Family::Trap => speed_cell(accumulated(racer), units),
                _ if racer.knocked_out => Cell::Label(label::KNOCKED_OUT),
                _ => time_cell(racer.time),
            };
            [Cell::Text(racer.place.to_string()), Cell::Text(racer.name.clone()), last]
        })
        .collect();
    Table {
        title: label::STANDINGS_TITLE,
        footer: button(Page::Standings, family),
        headers: [Some(label::POSITION), Some(label::NAME), Some(third)],
        rows,
        filter: false,
    }
}

/// The sum of a racer's speed-trap speeds, m/s.
fn accumulated(racer: &ReportRacer) -> f32 {
    racer.traps.iter().flatten().sum()
}

/// The race's winner: the racer whose final place is first.
fn winner(report: &RaceReport) -> Option<&ReportRacer> {
    report.racers.iter().find(|r| r.place == 1 && r.time.is_some())
}

fn statistics(report: &RaceReport, family: Family, racer: Option<&ReportRacer>, units: SpeedUnits) -> Table {
    let rows = racer.map_or_else(Vec::new, |racer| statistic_rows(report, family, racer, units));
    Table {
        title: label::STATISTICS_TITLE,
        footer: button(Page::Statistics, family),
        headers: [None; 3],
        rows,
        filter: true,
    }
}

fn row(label: u32, value: Cell) -> [Cell; 3] {
    [Cell::Label(label), Cell::Empty, value]
}

fn statistic_rows(report: &RaceReport, family: Family, racer: &ReportRacer, units: SpeedUnits) -> Vec<[Cell; 3]> {
    let top = row(label::TOP_SPEED, speed_cell(racer.stats.top_speed, units));
    let average = row(label::AVERAGE_SPEED, speed_cell(racer.average_speed, units));
    let behind = row(label::TIME_BEHIND, time_cell(racer.time_behind));
    let collisions = row(label::COLLISIONS, Cell::Text(racer.collisions.to_string()));
    match family {
        Family::Sprint => vec![top, average, behind, collisions],
        Family::Laps => {
            vec![top, average, behind, row(label::LAP_VARIANCE, time_cell(lap_variance(racer))), collisions]
        }
        Family::Toll => vec![top, average, collisions],
        Family::Drag => {
            let metric = units == SpeedUnits::Metric;
            let sixty = if metric { label::ZERO_TO_HUNDRED } else { label::ZERO_TO_SIXTY };
            let quarter = if metric { label::FOUR_HUNDRED_M } else { label::QUARTER_MILE };
            vec![
                row(sixty, time_cell(racer.stats.sixty_at)),
                row(quarter, time_cell(racer.stats.quarter_at)),
                // The shift rating is not read, so no count is kept.
                row(label::PERFECT_SHIFTS, Cell::Label(label::NONE)),
                behind,
                collisions,
            ]
        }
        Family::Trap => {
            let mine = accumulated(racer);
            let gap = winner(report).map(|w| (accumulated(w) - mine).abs());
            let passed: Vec<f32> = racer.traps.iter().flatten().copied().collect();
            let variance = match passed.len() > 1 {
                true => {
                    let high = passed.iter().copied().fold(f32::MIN, f32::max);
                    let low = passed.iter().copied().fold(f32::MAX, f32::min);
                    speed_cell(high - low, units)
                }
                false => Cell::Label(label::NONE),
            };
            vec![
                row(label::ACCUMULATED_SPEED, speed_cell(mine, units)),
                row(label::SPEED_VARIANCE, variance),
                row(label::SPEED_BEHIND, gap.map_or(Cell::Label(label::NONE), |g| speed_cell(g, units))),
            ]
        }
    }
}

/// The spread of a racer's lap times, longest minus shortest; none under two laps (the original needs more than one).
pub fn lap_variance(racer: &ReportRacer) -> Option<f32> {
    if racer.laps.len() < 2 {
        return None;
    }
    let high = racer.laps.iter().map(|l| l.time).fold(f32::MIN, f32::max);
    let low = racer.laps.iter().map(|l| l.time).fold(f32::MAX, f32::min);
    Some(high - low)
}

fn stages(report: &RaceReport, family: Family, racer: Option<&ReportRacer>, units: SpeedUnits) -> Table {
    let Some(racer) = racer else {
        return Table {
            title: label::STAGES_TITLE,
            footer: button(Page::Stages, family),
            headers: [None; 3],
            rows: Vec::new(),
            filter: true,
        };
    };
    let (title, first, middle, last, rows) = match family {
        Family::Sprint | Family::Drag => {
            (label::STAGES_TITLE, label::STAGE, label::POSITION, label::TIME, stage_rows(racer))
        }
        Family::Laps => (label::LAPS_TITLE, label::LAP, label::POSITION, label::TIME, lap_rows(report, racer)),
        Family::Toll => (label::TOLLBOOTHS_TITLE, label::TOLLBOOTH, label::POSITION, label::TIME, toll_rows(racer)),
        Family::Trap => {
            (label::SPEEDTRAPS_TITLE, label::SPEEDTRAP, label::POSITION, label::SPEED, trap_rows(report, racer, units))
        }
    };
    let middle = if family == Family::Toll { None } else { Some(middle) };
    Table {
        title,
        footer: button(Page::Stages, family),
        headers: [Some(first), middle, Some(last)],
        rows,
        filter: true,
    }
}

/// A row that has nothing to say: the dashes in the place and the time.
fn dashes(first: String) -> [Cell; 3] {
    [Cell::Text(first), Cell::Label(label::NONE), Cell::Label(label::NONE)]
}

/// 20, 40, 60 and 80 percent from the marks, and 100 percent from the finish.
fn stage_rows(racer: &ReportRacer) -> Vec<[Cell; 3]> {
    let mut rows: Vec<[Cell; 3]> = blackbox_race::STAGE_PERCENTS
        .iter()
        .zip(racer.stats.stages)
        .map(|(percent, mark)| match mark {
            Some(mark) => [
                Cell::Text(format!("{percent:.0}%")),
                Cell::Text(mark.place.to_string()),
                Cell::Text(race_time(mark.time)),
            ],
            None => dashes(format!("{percent:.0}%")),
        })
        .collect();
    rows.push(match racer.time {
        Some(time) => [Cell::Text("100%".into()), Cell::Text(racer.place.to_string()), Cell::Text(race_time(time))],
        None => dashes("100%".into()),
    });
    rows
}

/// One row per lap of the course: the lap, the place held at its end and its time.
fn lap_rows(report: &RaceReport, racer: &ReportRacer) -> Vec<[Cell; 3]> {
    (0..report.laps.max(1) as usize)
        .map(|lap| match racer.laps.get(lap) {
            Some(done) => [
                Cell::Text((lap + 1).to_string()),
                Cell::Text(done.place.to_string()),
                Cell::Text(race_time(done.time)),
            ],
            None => dashes((lap + 1).to_string()),
        })
        .collect()
}

/// One row per tollbooth reached, with the race time there, and the finish after them.
fn toll_rows(racer: &ReportRacer) -> Vec<[Cell; 3]> {
    let mut rows: Vec<[Cell; 3]> = racer
        .checkpoints
        .iter()
        .enumerate()
        .map(|(i, time)| [Cell::Text((i + 1).to_string()), Cell::Empty, Cell::Text(race_time(*time))])
        .collect();
    rows.push([Cell::Text((racer.checkpoints.len() + 1).to_string()), Cell::Empty, time_cell(racer.time)]);
    rows
}

/// One row per speed trap: its number, the place the racer's speed there ranks among the racers, and the speed.
fn trap_rows(report: &RaceReport, racer: &ReportRacer, units: SpeedUnits) -> Vec<[Cell; 3]> {
    racer
        .traps
        .iter()
        .enumerate()
        .map(|(slot, speed)| {
            let Some(speed) = *speed else { return dashes((slot + 1).to_string()) };
            let faster =
                report.racers.iter().filter(|r| r.traps.get(slot).copied().flatten().is_some_and(|s| s > speed));
            [Cell::Text((slot + 1).to_string()), Cell::Text((faster.count() + 1).to_string()), speed_cell(speed, units)]
        })
        .collect()
}
