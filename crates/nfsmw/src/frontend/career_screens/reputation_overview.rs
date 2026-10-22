//! The Blacklist page of the safe house in the Original look (`SafeHouseReputationOverview.fng`;
//! `docs/specs/career-reputation-overview.md` has the addresses of the original's handlers).
//!
//! It shows one rival: the rank and alias, the portrait, the challenge text and the rival panel (races won,
//! milestones done and bounty against the rival's three requirements, a tick per requirement met), the Challenge
//! Rival button, the DEFEATED stamp and the bounty and cash ticker. Up and Down step the rival between the career's
//! own rival and rival 15, wrapping; Left and Right pick one of four sheets (race events, milestones, bounty, bio),
//! Accept opens it; key 2 (`PAD_BUTTON4`) opens the rival challenge when the button is on; Back leaves for the safe
//! house menu. The rules are pure functions below, tested in `reputation_overview_tests.rs`.

use blackbox_feng::fe_hash_upper;
use blackbox_feng::ids::{PAD_BACK, PAD_BUTTON0, PAD_DOWN, PAD_LEFT, PAD_RIGHT, PAD_UP};
use blackbox_feng::runtime::pad;
use blackbox_text::label_hash;
use nfsmw_career::blacklist::Counters;
use nfsmw_career::settings::{FIRST_RIVAL_STAGE, Flags};

use super::blacklist::{DISABLED_ALPHA, RivalRow, career_rows, current_bin, rank_title, rival_key, start_bin};
use super::{RIVAL_LIST_READY, safehouse_menu::money_text};
use crate::career::CareerHandle;
use crate::frontend::ids::screen;
use crate::frontend::logic::{Args, Command, Cx, Plain, ScreenLogic};
use crate::ui::Text;

/// Objects of the package by name hash. The title, alias, challenge text and ticker lines are groups whose strings get
/// the text; the numbers of the panel are strings.
const TITLE: u32 = 0x2426_57CE;
const ALIAS: u32 = 0x7ac3_d0c9;
const CHALLENGE_TEXT: u32 = 0x79cf_0442;
const PORTRAIT: u32 = 0xc1f6_2308;
const GRAFFITI: u32 = 0xf5a2_a087;
const STAMP: u32 = 0x34d4_433b;
const BUTTON_GROUP: u32 = 0x55f6_aa1a;
const STEPPER_UP: u32 = 0x47b2_2fca;
const STEPPER_DOWN: u32 = 0x72ad_598c;
const OPTION_TITLE: u32 = 0x5e7b_09c9;
const ARROW_LEFT: u32 = 0x3df3_9a82;
const ARROW_RIGHT: u32 = 0xfcd5_b255;
const TICKER_BOUNTY: u32 = 0xb514_e2d8;
const TICKER_CASH: u32 = 0xf91a_59f6;
/// The "current" column of the panel: races won, milestones done, the stable's bounty.
const NOW: [u32; 3] = [0x15d8_0973, 0xd802_fba8, 0x322b_18f9];
/// The "target" column: the rival's requirements, in the same order.
const NEED: [u32; 3] = [0xde7a_d199, 0x7242_962e, 0x055c_6e5f];
/// The tick beside each row.
const TICKS: [u32; 3] = [0x4c3b_1536, 0x4c3b_1537, 0x4c3b_1538];

/// The scripts: a tick that shows and one that hides, and the portrait's slide for a step up and a step down.
const TICK_SHOWN: u32 = 0xE636_1F46;
const TICK_HIDDEN: u32 = 0x0016_A259;
const PORTRAIT_UP: u32 = 0x9E5E_6B5F;
const PORTRAIT_DOWN: u32 = 0xAF9D_73F2;

/// Labels: the ticker's "Bounty:" and "Cash:".
const LABEL_BOUNTY: u32 = 0xCE6B_99B1;
const LABEL_CASH: u32 = 0x073B_79E0;
/// The key 2 button: opens the rival challenge.
const CHALLENGE_KEY: u32 = PAD_BUTTON0 + 4;

/// Lowest rival the overview steps to: the one the career is at (the earlier ones are the beaten rivals above it).
pub fn lowest(current: u8) -> u8 {
    start_bin(current)
}

/// The rival shown when the page opens: the one asked for, kept between the lowest and rival 15; none asked is the lowest.
pub fn first_shown(asked: u8, low: u8) -> u8 {
    if asked == 0 {
        return low;
    }
    asked.clamp(low, FIRST_RIVAL_STAGE)
}

/// One step of Up or Down: Up goes to the next higher rival and wraps from 15 to
/// the lowest, Down goes to the next lower and wraps from the lowest to 15.
pub fn step(shown: u8, low: u8, up: bool) -> u8 {
    if up {
        let next = shown.saturating_add(1);
        if next > FIRST_RIVAL_STAGE {
            return low;
        }
        return next;
    }
    let next = shown.saturating_sub(1);
    if next < low {
        return FIRST_RIVAL_STAGE;
    }
    next
}

/// One row of the panel: what the player has against what the rival asks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Requirement {
    pub have: u64,
    pub need: u32,
}

impl Requirement {
    /// Met when the player has at least what is asked (the panel's tick).
    pub fn met(self) -> bool {
        self.have >= u64::from(self.need)
    }
}

/// The Challenge Rival button: gone for a beaten rival, otherwise on or off.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Hidden,
    Off,
    On,
}

/// What the page shows of one rival.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Panel {
    pub bin: u8,
    pub races: Requirement,
    pub challenges: Requirement,
    pub bounty: Requirement,
    /// A beaten rival: the stamp shows and the button is gone.
    pub defeated: bool,
    pub button: Button,
    pub cash: u32,
}

/// The panel of `row` while the career is at `current`: a rival above the career's bin is beaten, and so is every
/// rival once the career is complete.
pub fn panel(row: &RivalRow, current: u8, flags: Flags, bounty: u64, cash: u32) -> Panel {
    let Counters { races_won, challenges } = row.counters;
    let defeated = flags.contains(Flags::CAREER_COMPLETE) || row.bin > current;
    let button = match (defeated, row.button_active) {
        (true, _) => Button::Hidden,
        (false, true) => Button::On,
        (false, false) => Button::Off,
    };
    Panel {
        bin: row.bin,
        races: Requirement { have: u64::from(races_won), need: row.races_needed },
        challenges: Requirement { have: u64::from(challenges), need: row.challenges_needed },
        bounty: Requirement { have: bounty, need: row.bounty_needed },
        defeated,
        button,
        cash,
    }
}

/// The four sheets the page offers, in the order Left and Right walk them (the original's option ids 0, 1, 2, 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sheet {
    RaceEvents,
    Milestones,
    Bounty,
    Bio,
}

pub const SHEETS: [Sheet; 4] = [Sheet::RaceEvents, Sheet::Milestones, Sheet::Bounty, Sheet::Bio];

impl Sheet {
    /// The label of the sheet's name (`REP_SHEET_RACE_EVENTS`, `REP_SHEET_MILESTONE_EVENTS`, `REP_SHEET_BOUNTY`,
    /// `REP_SHEET_BIO`).
    pub fn label(self) -> u32 {
        match self {
            Sheet::RaceEvents => 0x84E4_A54C,
            Sheet::Milestones => 0x216F_1B81,
            Sheet::Bounty => 0xE451_941E,
            Sheet::Bio => 0x2D15_9737,
        }
    }

    /// The screen the sheet opens. The race, milestone and bounty sheets are not built yet, so those stay greyed.
    pub fn screen(self) -> Option<&'static str> {
        match self {
            Sheet::Bio => Some(screen::SAFE_HOUSE_RIVAL_BIO),
            Sheet::RaceEvents | Sheet::Milestones | Sheet::Bounty => None,
        }
    }
}

/// The page opened at `bin` (the screens it opened come back here).
pub fn open(cx: &mut Cx, bin: u8) {
    let args = Args { safehouse: true, rival: bin, ..Args::default() };
    cx.send(Command::Switch(screen::SAFE_HOUSE_REPUTATION_OVERVIEW.to_string(), args));
}

/// The panel of rival `bin` from the career, or `None` without one.
fn read(bin: u8) -> Option<Panel> {
    let rows = career_rows();
    let row = rows.iter().find(|row| row.bin == bin)?;
    CareerHandle::shared().with(|career| {
        panel(
            row,
            career.blacklist.current_bin(),
            career.settings.flags,
            career.stable.total_bounty(),
            career.settings.cash,
        )
    })
}

struct Overview {
    shown: u8,
    low: u8,
    /// The rival the page was opened for, or 0 for the career's own.
    asked: u8,
    sheet: usize,
}

/// The logic of `name` (lower case) when it is the overview.
pub fn make(name: &str, args: Args) -> Option<Box<dyn ScreenLogic>> {
    if name != screen::SAFE_HOUSE_REPUTATION_OVERVIEW.to_ascii_lowercase() {
        return None;
    }
    // Outside a career there is no Blacklist to show.
    if current_bin().is_none() {
        return Some(Box::new(Plain));
    }
    Some(Box::new(Overview { shown: FIRST_RIVAL_STAGE, low: FIRST_RIVAL_STAGE, asked: args.rival, sheet: 0 }))
}

/// A game text, empty when missing.
fn text_of(cx: &Cx, label: u32) -> String {
    Text::Game(label).resolve(cx.assets).unwrap_or_default()
}

impl Overview {
    fn fill_rival(&self, cx: &mut Cx) {
        if let Some(title) = rank_title(cx, self.shown) {
            cx.text_group(TITLE, &title);
        }
        cx.label_group(ALIAS, label_hash(&rival_key(self.shown, "AKA")));
        cx.label_group(CHALLENGE_TEXT, label_hash(&rival_key(self.shown, "CHALLENGE")));
        let picture = |suffix: &str| fe_hash_upper(&format!("RIVAL_{:02}{suffix}", self.shown));
        for (name, texture) in [(PORTRAIT, picture("")), (GRAFFITI, picture("_GRAF"))] {
            if let Some(o) = cx.object(name) {
                cx.rt.set_texture(o, texture);
            }
        }
    }

    fn fill_panel(&self, cx: &mut Cx) {
        let Some(panel) = read(self.shown) else { return };
        let rows = [panel.races, panel.challenges, panel.bounty];
        for (i, row) in rows.iter().enumerate() {
            let money = i == 2;
            let (have, need) = if money {
                (money_text(row.have), money_text(u64::from(row.need)))
            } else {
                (row.have.to_string(), row.need.to_string())
            };
            cx.text_object(NOW[i], &have);
            cx.text_object(NEED[i], &need);
            cx.script(TICKS[i], if row.met() { TICK_SHOWN } else { TICK_HIDDEN });
        }
        cx.hide(STAMP, !panel.defeated);
        // The button and the panel rows share their place: the package hides the rows when the button is on.
        cx.hide(BUTTON_GROUP, panel.button != Button::On);
        if panel.defeated {
            cx.rt.post_to_package(cx.package, RIVAL_LIST_READY);
        }
        let message = match panel.button {
            Button::Hidden => None,
            Button::On => Some("RIVAL_CHALLENGE_ACTIVATED"),
            Button::Off => Some("RIVAL_CHALLENGE_DEACTIVATED"),
        };
        if let Some(message) = message {
            cx.rt.post_to_package(cx.package, fe_hash_upper(message));
        }
        let bounty = format!("{} {}", text_of(cx, LABEL_BOUNTY), money_text(panel.bounty.have));
        let cash = format!("{} {}", text_of(cx, LABEL_CASH), money_text(u64::from(panel.cash)));
        cx.text_group(TICKER_BOUNTY, &bounty);
        cx.text_group(TICKER_CASH, &cash);
    }

    fn fill_sheet(&self, cx: &mut Cx) {
        let sheet = SHEETS[self.sheet];
        let Some(title) = cx.object(OPTION_TITLE) else { return };
        cx.label(title, sheet.label());
        cx.rt.set_alpha(title, if sheet.screen().is_some() { 255 } else { DISABLED_ALPHA });
        cx.hide(ARROW_LEFT, self.sheet == 0);
        cx.hide(ARROW_RIGHT, self.sheet + 1 == SHEETS.len());
    }

    fn step_rival(&mut self, cx: &mut Cx, up: bool) {
        let next = step(self.shown, self.low, up);
        if next == self.shown {
            return;
        }
        self.shown = next;
        cx.script(PORTRAIT, if up { PORTRAIT_UP } else { PORTRAIT_DOWN });
        self.fill_rival(cx);
        self.fill_panel(cx);
    }

    fn step_sheet(&mut self, cx: &mut Cx, right: bool) {
        let next = if right { (self.sheet + 1).min(SHEETS.len() - 1) } else { self.sheet.saturating_sub(1) };
        self.sheet = next;
        self.fill_sheet(cx);
    }

    fn accept(&self, cx: &mut Cx) {
        let Some(name) = SHEETS[self.sheet].screen() else { return };
        let args = Args { safehouse: true, rival: self.shown, overview: true, ..Args::default() };
        cx.send(Command::Switch(name.to_string(), args));
    }

    /// Key 2 opens the rival challenge at the shown rival, while its button is on.
    fn challenge(&self, cx: &mut Cx) {
        let Some(panel) = read(self.shown) else { return };
        if panel.button != Button::On {
            return;
        }
        let args = Args { safehouse: true, rival: self.shown, overview: true, ..Args::default() };
        cx.send(Command::Switch(screen::SAFE_HOUSE_RIVAL_CHALLENGE.to_string(), args));
    }
}

impl ScreenLogic for Overview {
    fn start(&mut self, cx: &mut Cx) {
        self.low = lowest(current_bin().unwrap_or(FIRST_RIVAL_STAGE));
        self.shown = first_shown(self.asked, self.low);
        // Nothing to step through while the career is at rival 15.
        let single = self.low == FIRST_RIVAL_STAGE;
        cx.hide(STEPPER_UP, single);
        cx.hide(STEPPER_DOWN, single);
        self.fill_rival(cx);
        self.fill_panel(cx);
        self.fill_sheet(cx);
    }

    // The page has no focused button to answer an accept, so the pad press is the accept.
    fn pad_pressed(&mut self, cx: &mut Cx, bits: u32) {
        if bits & pad::ACCEPT != 0 {
            self.accept(cx);
        }
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            PAD_UP => self.step_rival(cx, true),
            PAD_DOWN => self.step_rival(cx, false),
            PAD_LEFT => self.step_sheet(cx, false),
            PAD_RIGHT => self.step_sheet(cx, true),
            CHALLENGE_KEY => self.challenge(cx),
            PAD_BACK => {
                let args = Args { safehouse: true, ..Args::default() };
                cx.send(Command::Switch(screen::MAIN_MENU_SUB.to_string(), args));
            }
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "reputation_overview_tests.rs"]
mod tests;
