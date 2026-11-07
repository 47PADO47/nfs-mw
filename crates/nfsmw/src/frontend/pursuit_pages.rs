//! The lines of `PostRace_Pursuit.fng` and its two other pages (`docs/specs/pursuit-escape-results.md` section 3): the
//! infractions page ticks each infraction the chase committed, the milestones page shows each milestone of the career's
//! bin with its value and goal and ticks those at stake and banked **[exe]**.
//!
//! A line is the package's `LINEn_GROUP` with its label (`COLUMN1_DATAn`), its value (`COLUMN2_DATAn`), a tick
//! (`CHECKn`) and an empty box (`EMPTYn`), n from 1 to 9. Drawing one first hides both marks and puts the line back to
//! its rest look, as the original's line does.

use blackbox_feng::fe_hash_upper;
use blackbox_pursuit::{Infraction, InfractionLog};
use nfsmw_career::milestones::StatType;

use super::logic::Cx;
use super::post_race::lap_time;
use crate::i18n::fmt::grouped;
use crate::viewer::{MilestoneLine, MilestoneStatus};

/// The label and the value of line `n` (0 to 8) are the name hashes `0x38B136A8 + n` and `0x85AB7369 + n`.
const FIRST_LABEL: u32 = 0x38B1_36A8;
const FIRST_VALUE: u32 = 0x85AB_7369;
/// The package's lines.
pub(super) const LINES: u32 = 9;
/// The scripts of a line's objects: the rest look, hidden, shown, and the dimmed look of a banked milestone.
const REST: u32 = 0x0017_44B3;
const HIDE: u32 = 0x0016_A259;
const SHOW: u32 = 0x001C_A7C0;
const DIM: u32 = 0x0016_3C76;

/// The infractions page's lines, top to bottom, each the infraction (its mask bit in the chase) and its language label
/// **[exe]**: Speeding (mask 2), Excessive Speeding (mask 1), Reckless Driving, Ramming a Police Vehicle, Hit & Run,
/// Damage to Property, Resisting Arrest, Driving off Roadway.
const INFRACTION_LINES: [(Infraction, u32); 8] = [
    (Infraction::Racing, 0x4B0F_F103),
    (Infraction::Speeding, 0x8EC1_A6EC),
    (Infraction::RecklessDriving, 0x370F_331D),
    (Infraction::Assault, 0x8462_A784),
    (Infraction::HitAndRun, 0xDFF2_54BA),
    (Infraction::DamageToProperty, 0x7DBD_5B34),
    (Infraction::ResistingArrest, 0x2B1D_E2A9),
    (Infraction::OffRoad, 0xB0EF_5C12),
];

/// How a line is marked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Mark {
    /// Neither the tick nor the box: the pursuit page.
    None,
    Empty,
    Ticked,
    /// Ticked and the whole line dimmed: a banked milestone.
    Banked,
}

/// One line to draw: its label, its value (`None` hides the value column) and its mark.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Line {
    pub label: String,
    pub value: Option<String>,
    pub mark: Mark,
}

/// Draws line `n` (0 to 8), or empties it when there is no line.
pub(super) fn draw(cx: &mut Cx, n: u32, line: Option<&Line>) {
    let (check, empty, group) = (name("CHECK", n), name("EMPTY", n), fe_hash_upper(&format!("LINE{}_GROUP", n + 1)));
    cx.script(check, HIDE);
    cx.script(empty, HIDE);
    cx.script(group, REST);
    let Some(line) = line else {
        cx.text_object(FIRST_LABEL + n, "");
        cx.text_object(FIRST_VALUE + n, "");
        return;
    };
    cx.text_object(FIRST_LABEL + n, &line.label);
    match &line.value {
        Some(value) => {
            cx.text_object(FIRST_VALUE + n, value);
            cx.script(FIRST_VALUE + n, REST);
        }
        None => cx.script(FIRST_VALUE + n, HIDE),
    }
    match line.mark {
        Mark::None => {}
        Mark::Empty => cx.script(empty, SHOW),
        Mark::Ticked => cx.script(check, SHOW),
        Mark::Banked => {
            cx.script(group, DIM);
            cx.script(check, DIM);
        }
    }
}

/// The name hash of `CHECKn` or `EMPTYn` for line `n` counted from 0.
fn name(prefix: &str, n: u32) -> u32 {
    fe_hash_upper(&format!("{prefix}{}", n + 1))
}

/// The infractions page: every infraction, ticked when the chase committed it, with no value.
pub(super) fn infractions(committed: &InfractionLog) -> Vec<Line> {
    INFRACTION_LINES
        .iter()
        .map(|&(infraction, label)| Line {
            label: crate::i18n::game_or_error(label),
            value: None,
            mark: match committed.contains(infraction) {
                true => Mark::Ticked,
                false => Mark::Empty,
            },
        })
        .collect()
}

/// The milestones page: each milestone's short name, `value/goal` while it is open and the goal alone once it is at
/// stake or banked. The value is clamped to the goal; the two time statistics read as times.
pub(super) fn milestones(lines: &[Option<MilestoneLine>]) -> Vec<Line> {
    lines.iter().flatten().map(milestone).collect()
}

fn milestone(line: &MilestoneLine) -> Line {
    let label = line.label.map_or_else(|| line.stat.name().to_owned(), crate::i18n::game_or_error);
    let text = |value: f32| match line.stat {
        StatType::PursuitLength | StatType::PursuitEvasionTime => lap_time(value),
        _ => grouped(value.round() as i64),
    };
    let (value, mark) = match line.status {
        MilestoneStatus::Open => (format!("{}/{}", text(line.value.min(line.goal)), text(line.goal)), Mark::Empty),
        MilestoneStatus::AtStake => (text(line.goal), Mark::Ticked),
        MilestoneStatus::Complete => (text(line.goal), Mark::Banked),
    };
    Line { label, value: Some(value), mark }
}

#[cfg(test)]
#[path = "pursuit_pages_tests.rs"]
mod tests;
