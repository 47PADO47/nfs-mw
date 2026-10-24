//! The milestones sheet (`SafeHouseMilestones.fng`): the pursuit milestones of the rival's bin, then its speed traps
//! (`career-rapsheet-2.md` section 5, `career-milestones.md` section 5).
//!
//! A milestone that is banked, and a speed trap that is done, show as complete and Accept does nothing on them. A locked
//! one (its bin is not reached) is greyed. Any other asks `REP_SHEET_MILESTONE_CONFIRM` (`REP_SHEET_PHOTOTICKET_CONFIRM`
//! for a speed trap) and, on Yes, the car goes to the entry's spawn point ([`Command::JumpTo`]).

use blackbox_text::label_hash;
use glam::Vec3;
use nfsmw_career::milestones::speedtrap::SpeedTrap;
use nfsmw_career::milestones::{Milestone, MilestoneSet, MilestoneState, StatType};
use nfsmw_data::gameplay::GameplayData;

use super::{Ask, Page, Status};
use crate::career::CareerHandle;
use crate::frontend::logic::{Command, Cx};
use crate::i18n::fmt::grouped;
use crate::i18n::t;
use crate::settings::SpeedUnits;
use crate::ui::{Text, UiAssets};

/// Objects of `SafeHouseMilestones.fng` by name hash: the header (and its shadow), the description, and the bounty reward.
const HEADER: [u32; 2] = [0xCFA2_3A2D, 0xB511_7FDE];
const DESCRIPTION: u32 = 0x0280_49D6;
const REWARD: u32 = 0xB21D_69BD;
/// The three recorded lines (average, best time, top speed): a milestone keeps none of them, so they are hidden.
const RECORDED: [u32; 6] = [0xEBD7_F926, 0x4B4D_89F5, 0xDEDC_8803, 0x8FD4_1BB4, 0xDE91_45FB, 0xD18D_600A];
/// The language label of the km/h and mph units.
const KPH_LABEL: u32 = 0x8569_A25F;
const MPH_LABEL: u32 = 0x8569_AB44;
const KMH_TO_MPH: f32 = 0.621_371;

/// Every object of the package the page names.
#[cfg(test)]
pub(super) fn used() -> Vec<u32> {
    let mut all = vec![DESCRIPTION, REWARD];
    all.extend(HEADER);
    all.extend(RECORDED);
    all
}

/// What one entry of the sheet is.
#[derive(Clone, Debug, PartialEq)]
pub enum Entry {
    /// A pursuit milestone.
    Pursuit { stat: StatType, goal: f32, bounty: u32, state: MilestoneState, label: Option<i32> },
    /// A speed trap: its threshold in km/h, the speed it was passed at (km/h, 0 before) and whether it is open.
    Trap { threshold: u16, bounty: u32, unlocked: bool, done: bool, best: f32 },
}

/// One row: the entry and the place the car goes to, in render space.
#[derive(Clone, Debug, PartialEq)]
pub struct MilestoneRow {
    pub entry: Entry,
    pub spawn: Option<[f32; 2]>,
}

impl MilestoneRow {
    pub fn status(&self) -> Status {
        match &self.entry {
            Entry::Pursuit { state, .. } => match state {
                MilestoneState::Locked => Status::Locked,
                MilestoneState::Active | MilestoneState::AtStake => Status::Open,
                MilestoneState::Complete => Status::Done,
            },
            Entry::Trap { unlocked, done, .. } => match (unlocked, done) {
                (_, true) => Status::Done,
                (true, false) => Status::Open,
                (false, false) => Status::Locked,
            },
        }
    }
}

/// Physics space to the render-space ground position the car is put at (`space::to_render` without the height).
pub fn render_xy(physics: Vec3) -> [f32; 2] {
    [physics.z, -physics.x]
}

/// The rows of bin `bin`: its milestones (every state), then its speed traps.
pub fn milestone_rows(set: &MilestoneSet, traps: &[SpeedTrap], data: &GameplayData, bin: u8) -> Vec<MilestoneRow> {
    let pursuits = set.milestones().iter().filter(|m| m.def.bin == bin).map(|m| pursuit_row(m, data));
    let speed = traps.iter().filter(|trap| trap.bin() == bin).map(|trap| trap_row(trap, data));
    pursuits.chain(speed).collect()
}

fn pursuit_row(milestone: &Milestone, data: &GameplayData) -> MilestoneRow {
    let def = &milestone.def;
    let spawn = data.milestones.iter().find(|m| m.key == def.key).and_then(|m| m.spawn).map(render_xy);
    let label = data.milestone_types.iter().find(|kind| kind.name == def.stat.name()).and_then(|kind| kind.label);
    MilestoneRow {
        entry: Entry::Pursuit {
            stat: def.stat,
            goal: def.threshold,
            bounty: def.bounty,
            state: milestone.state,
            label,
        },
        spawn,
    }
}

fn trap_row(trap: &SpeedTrap, data: &GameplayData) -> MilestoneRow {
    let spawn = data.speed_traps.iter().find(|t| t.key == trap.key()).and_then(|t| t.spawn).map(render_xy);
    MilestoneRow {
        entry: Entry::Trap {
            threshold: trap.threshold_kmh(),
            bounty: trap.bounty(),
            unlocked: trap.unlocked(),
            done: trap.done(),
            best: trap.recorded_mps() * 3.6,
        },
        spawn,
    }
}

/// A goal as the label writes it: minutes and seconds for the two time statistics, a count otherwise.
pub fn goal_text(stat: StatType, goal: f32) -> String {
    let n = goal.round().max(0.0) as u64;
    match stat {
        StatType::PursuitLength | StatType::PursuitEvasionTime => format!("{}:{:02}", n / 60, n % 60),
        _ => n.to_string(),
    }
}

/// The rows of the milestones sheet.
pub struct MilestonePage {
    rows: Vec<MilestoneRow>,
}

impl MilestonePage {
    /// The page of bin `bin` from the career being played.
    pub fn read(bin: u8) -> MilestonePage {
        let rows = CareerHandle::shared()
            .with(|career| milestone_rows(&career.milestones, &career.traps, &career.data.gameplay, bin))
            .unwrap_or_default();
        MilestonePage { rows }
    }
}

fn game_text(assets: &UiAssets, label: u32) -> String {
    Text::Game(label).resolve().unwrap_or_default()
}

/// The title and the description of an entry, from the language table.
fn words(assets: &UiAssets, entry: &Entry, metric: bool) -> (String, String) {
    match entry {
        Entry::Pursuit { stat, goal, label, .. } => {
            let goal = goal_text(*stat, *goal);
            let Some(index) = label else { return (stat.name().to_owned(), goal) };
            let title = game_text(assets, label_hash(&format!("BLACKLIST_PURSUIT_MILESTONES_{index:02}_SHORT")));
            let body = game_text(assets, label_hash(&format!("BLACKLIST_PURSUIT_MILESTONES_{index:02}")));
            (title, blackbox_text::printf::fill(&body, &[goal.as_str()]))
        }
        Entry::Trap { threshold, .. } => {
            let (speed, unit) = speed_in(assets, f32::from(*threshold), metric);
            (t!("career-sheet-speed-trap"), format!("{speed} {unit}"))
        }
    }
}

/// A speed in km/h as the player's unit writes it, and the unit's word.
fn speed_in(assets: &UiAssets, kmh: f32, metric: bool) -> (String, String) {
    let unit = game_text(assets, if metric { KPH_LABEL } else { MPH_LABEL });
    let value = if metric { kmh } else { kmh * KMH_TO_MPH };
    (format!("{value:.0}"), unit)
}

impl Page for MilestonePage {
    fn len(&self) -> usize {
        self.rows.len()
    }

    fn status(&self, at: usize) -> Status {
        self.rows.get(at).map_or(Status::Locked, MilestoneRow::status)
    }

    fn name(&self, assets: &UiAssets, at: usize) -> String {
        self.rows.get(at).map(|row| words(assets, &row.entry, true).0).unwrap_or_default()
    }

    fn summary(&self, _assets: &UiAssets, at: usize) -> String {
        let Some(row) = self.rows.get(at) else { return String::new() };
        let bounty = match row.entry {
            Entry::Pursuit { bounty, .. } | Entry::Trap { bounty, .. } => bounty,
        };
        match row.status() {
            Status::Done => t!("career-sheet-complete"),
            Status::Locked => t!("career-sheet-locked"),
            Status::Open => grouped(i64::from(bounty)),
        }
    }

    fn heading(&self) -> u32 {
        label_hash("REP_SHEET_MILESTONE_EVENTS")
    }

    fn fill(&self, cx: &mut Cx, at: usize) {
        let Some(row) = self.rows.get(at) else { return };
        let metric = cx.settings.speed_units == SpeedUnits::Metric;
        let (title, body) = words(cx.assets, &row.entry, metric);
        for name in HEADER {
            cx.text_object(name, &title);
        }
        cx.text_object(DESCRIPTION, &body);
        let bounty = match row.entry {
            Entry::Pursuit { bounty, .. } | Entry::Trap { bounty, .. } => bounty,
        };
        cx.text_object(REWARD, &grouped(i64::from(bounty)));
        for hidden in RECORDED {
            cx.hide(hidden, true);
        }
    }

    fn ask(&self, at: usize) -> Option<Ask> {
        let row = self.rows.get(at).filter(|row| row.status() == Status::Open)?;
        let at = row.spawn?;
        let label = match row.entry {
            Entry::Trap { .. } => "REP_SHEET_PHOTOTICKET_CONFIRM",
            Entry::Pursuit { .. } => "REP_SHEET_MILESTONE_CONFIRM",
        };
        Some(Ask { label: label_hash(label), then: Command::JumpTo { x: at[0], y: at[1] } })
    }

    /// The sheet starts on the first milestone still to do.
    fn first(&self) -> usize {
        self.rows.iter().position(|row| row.status() == Status::Open).unwrap_or(0)
    }
}
