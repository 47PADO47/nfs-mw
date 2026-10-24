//! The bounty sheet (`SafeHouseBounty.fng`): the places the car can jump to for a pursuit (`career-rapsheet-2.md`
//! section 5, `career-text.md` section 4).
//!
//! The 12 places of the install are numbered; the first four are always on offer, places 5 to 8 once the career is
//! below bin 13, and places 9 to 12 below bin 9. Accept asks `REP_SHEET_BOUNTY_CONFIRM` and, on Yes, the car goes to the
//! place ([`Command::JumpTo`]).

use blackbox_text::label_hash;
use nfsmw_data::gameplay::BountyJump;

use super::milestones::render_xy;
use super::{Ask, Page, Status};
use crate::career::CareerHandle;
use crate::frontend::logic::{Command, Cx};
use crate::ui::{Text, UiAssets};

/// Objects of `SafeHouseBounty.fng` by name hash: the header (and its shadow) and the description.
const HEADER: [u32; 2] = [0xCFA2_3A2D, 0x2D90_7846];
const DESCRIPTION: u32 = 0x0280_49D6;
/// The medal block of the package (times, cash rewards and the medal line) belongs to the challenge series and is hidden.
const MEDALS: [u32; 11] = [
    0xA3EA_9C37,
    0xA3EA_9C38,
    0xA3EA_9C39,
    0x281E_2E87,
    0x281E_2E88,
    0x281E_2E89,
    0x042D_0D22,
    0x042D_1142,
    0x1C8F_C866,
    0x7AF6_7920,
    0xEBD7_F926,
];

/// Every object of the package the page names.
#[cfg(test)]
pub(super) fn used() -> Vec<u32> {
    let mut all = vec![DESCRIPTION];
    all.extend(HEADER);
    all.extend(MEDALS);
    all
}

/// The places always on offer, and the stages below which the next ones are (`career-rapsheet-2.md` section 5).
const ALWAYS: i32 = 4;
const SECOND_GROUP: (i32, u8) = (8, 13);
const THIRD_GROUP: u8 = 9;

/// One place of the sheet.
#[derive(Clone, Debug, PartialEq)]
pub struct BountyRow {
    /// The number `NN` of the labels `BLACKLIST_BOUNTY_SPAWN_POINT_NN[_DESC]`.
    pub number: i32,
    /// Where the car goes, in render space.
    pub at: [f32; 2],
}

/// Whether the place `number` is on offer while the career is at `stage`.
pub fn on_offer(number: i32, stage: u8) -> bool {
    if number <= ALWAYS {
        return true;
    }
    if number <= SECOND_GROUP.0 {
        return stage < SECOND_GROUP.1;
    }
    stage < THIRD_GROUP
}

/// The places on offer at `stage`, in their numbers' order. A place with no position is left out.
pub fn bounty_rows(jumps: &[BountyJump], stage: u8) -> Vec<BountyRow> {
    jumps
        .iter()
        .filter(|jump| on_offer(jump.number, stage))
        .filter_map(|jump| Some(BountyRow { number: jump.number, at: render_xy(jump.position?) }))
        .collect()
}

/// The rows of the bounty sheet.
pub struct BountyPage {
    rows: Vec<BountyRow>,
}

impl BountyPage {
    pub fn read() -> BountyPage {
        let rows = CareerHandle::shared()
            .with(|career| bounty_rows(&career.data.gameplay.bounty_jumps, career.blacklist.current_bin()))
            .unwrap_or_default();
        BountyPage { rows }
    }
}

fn place_key(number: i32, suffix: &str) -> u32 {
    label_hash(&format!("BLACKLIST_BOUNTY_SPAWN_POINT_{number:02}{suffix}"))
}

fn game_text(assets: &UiAssets, label: u32) -> String {
    Text::Game(label).resolve().unwrap_or_default()
}

impl Page for BountyPage {
    fn len(&self) -> usize {
        self.rows.len()
    }

    fn status(&self, at: usize) -> Status {
        match self.rows.get(at) {
            Some(_) => Status::Open,
            None => Status::Locked,
        }
    }

    fn name(&self, assets: &UiAssets, at: usize) -> String {
        self.rows.get(at).map(|row| game_text(assets, place_key(row.number, ""))).unwrap_or_default()
    }

    fn summary(&self, assets: &UiAssets, at: usize) -> String {
        self.rows.get(at).map(|row| game_text(assets, place_key(row.number, "_DESC"))).unwrap_or_default()
    }

    fn heading(&self) -> u32 {
        label_hash("REP_SHEET_BOUNTY")
    }

    fn fill(&self, cx: &mut Cx, at: usize) {
        let Some(row) = self.rows.get(at) else { return };
        let title = game_text(cx.assets, place_key(row.number, ""));
        for name in HEADER {
            cx.text_object(name, &title);
        }
        cx.text_object(DESCRIPTION, &game_text(cx.assets, place_key(row.number, "_DESC")));
        for hidden in MEDALS {
            cx.hide(hidden, true);
        }
    }

    fn ask(&self, at: usize) -> Option<Ask> {
        let row = self.rows.get(at)?;
        Some(Ask {
            label: label_hash("REP_SHEET_BOUNTY_CONFIRM"),
            then: Command::JumpTo { x: row.at[0], y: row.at[1] },
        })
    }
}
