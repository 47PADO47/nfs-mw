//! The Blacklist page of the safe house in the Original look (`SafeHouseReputationOverview.fng`;
//! `docs/specs/career-reputation-overview.md` has the addresses of the original's handlers).
//!
//! It shows one rival: the rank and alias, the portrait, the challenge text and the rival panel (races won,
//! milestones done and bounty against the rival's three requirements, a tick per requirement met), the Challenge
//! Rival button, the DEFEATED stamp and the bounty and cash ticker. Up and Down step the rival between the career's
//! own rival and rival 15, wrapping; Left and Right pick one of four sheets (race events, milestones, bounty, bio),
//! Accept opens it; key 2 (`PAD_BUTTON4`) opens the rival challenge when the button is on; Back leaves for the safe
//! house menu. The rules are pure functions below, tested in `reputation_overview_tests.rs`.

use blackbox_feng::ids::{PAD_BACK, PAD_BUTTON0, PAD_DOWN, PAD_LEFT, PAD_RIGHT, PAD_UP};
use blackbox_feng::runtime::pad;
use blackbox_feng::{ObjectRef, fe_hash_upper};
use blackbox_text::label_hash;
use glam::Vec2;

mod model;
pub use model::*;
use nfsmw_career::settings::FIRST_RIVAL_STAGE;

use super::blacklist::{DISABLED_ALPHA, career_rows, current_bin, rank_title, rival_key};
use super::{RIVAL_LIST_READY, safehouse_menu::money_text};
use crate::career::CareerHandle;
use crate::frontend::ids::screen;
use crate::frontend::logic::{Args, Command, Cx, Footer, Pick, Plain, ScreenLogic};
use crate::ui::theme::{DEFAULT_ASPECT, MenuEntry, MenuList, MenuStyle};
use crate::ui::{Text, UiAssets};

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
/// What a click on the option takes: its icon, its plate and its title.
const OPTION_PARTS: [u32; 3] = [0xb046_69e3, 0xb39a_7599, 0x0c07_934e];
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

/// The page opened at `bin` (the screens it opened come back here).
pub fn open(cx: &mut Cx, bin: u8, from: Args) {
    let args = super::blacklist::carry(from, bin, false);
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

/// The object named `name` and everything inside it.
fn parts(cx: &Cx, name: u32) -> Vec<ObjectRef> {
    let (Some(root), Some(def)) = (cx.object(name), cx.rt.package(cx.package)) else { return Vec::new() };
    let root_guid = def.objects[root.index].guid;
    let inside = |index: usize| {
        let mut at = Some(def.objects[index].guid);
        while let Some(guid) = at {
            if guid == root_guid {
                return true;
            }
            at = cx.rt.find_guid(cx.package, guid).and_then(|o| def.objects[o.index].parent);
        }
        false
    };
    (0..def.objects.len())
        .filter(|&index| inside(index))
        .map(|index| ObjectRef { package: cx.package, index })
        .collect()
}

/// Whether a shown part of the object named `name` lies under `point`.
pub(super) fn hit(cx: &Cx, name: u32, point: Vec2) -> bool {
    cx.rt.object_at(&parts(cx, name), point).is_some()
}

struct Overview {
    shown: u8,
    low: u8,
    /// The rival the page was opened for, or 0 for the career's own.
    asked: u8,
    sheet: usize,
    /// What the page was opened with, for the menu it goes back to and the screens it opens.
    args: Args,
    /// The Remastered look is on: the page is a list and `cursor` is the entry on it.
    vertical: bool,
    cursor: usize,
    row: Row,
}

#[path = "reputation_overview_row.rs"]
mod row;
use row::Row;

/// The logic of `name` (lower case) when it is the overview.
pub fn make(name: &str, args: Args) -> Option<Box<dyn ScreenLogic>> {
    if name != screen::SAFE_HOUSE_REPUTATION_OVERVIEW.to_ascii_lowercase() {
        return None;
    }
    // Outside a career there is no Blacklist to show.
    if current_bin().is_none() {
        return Some(Box::new(Plain));
    }
    Some(Box::new(Overview {
        shown: FIRST_RIVAL_STAGE,
        low: FIRST_RIVAL_STAGE,
        asked: args.rival,
        sheet: 0,
        args,
        vertical: false,
        cursor: 0,
        row: Row::default(),
    }))
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
        if !self.vertical {
            self.row.place(cx, self.sheet);
        }
        let sheet = SHEETS[self.sheet];
        let Some(title) = cx.object(OPTION_TITLE) else { return };
        cx.label(title, sheet.label());
        cx.rt.set_alpha(title, if sheet.screen().is_some() { 255 } else { DISABLED_ALPHA });
        cx.hide(ARROW_LEFT, self.sheet == 0);
        cx.hide(ARROW_RIGHT, self.sheet + 1 == SHEETS.len());
    }

    /// The Remastered list's entries of the shown rival.
    fn list_entries(&self) -> Vec<(Entry, bool)> {
        entries(read(self.shown).map_or(Button::Hidden, |panel| panel.button))
    }

    /// Does what the entry under the cursor is: opens its sheet, or starts the challenge.
    fn activate(&self, cx: &mut Cx) {
        let Some((entry, enabled)) = self.list_entries().get(self.cursor).copied() else { return };
        if !enabled {
            return;
        }
        match entry {
            Entry::Sheet(sheet) => {
                let Some(name) = sheet.screen() else { return };
                let args = super::blacklist::carry(self.args, self.shown, true);
                cx.send(Command::Switch(name.to_string(), args));
            }
            Entry::Challenge => cx.send(Command::ChallengeRival { bin: self.shown }),
        }
    }

    fn back(&self, cx: &mut Cx) {
        // The Remastered look goes back to the list of rivals, the Original to the menu it came from.
        if self.vertical {
            let args = super::blacklist::carry(self.args, self.shown, false);
            return cx.send(Command::Switch(screen::SAFE_HOUSE_RIVAL_CHALLENGE.to_string(), args));
        }
        let (next, args) = super::blacklist::home(self.args);
        cx.send(Command::Switch(next.to_string(), args));
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
        let args = super::blacklist::carry(self.args, self.shown, true);
        cx.send(Command::Switch(name.to_string(), args));
    }

    /// Key 2 opens the rival challenge at the shown rival, while its button is on.
    fn challenge(&self, cx: &mut Cx) {
        let Some(panel) = read(self.shown) else { return };
        if panel.button != Button::On {
            return;
        }
        let args = super::blacklist::carry(self.args, self.shown, true);
        cx.send(Command::Switch(screen::SAFE_HOUSE_RIVAL_CHALLENGE.to_string(), args));
    }
}

impl ScreenLogic for Overview {
    fn start(&mut self, cx: &mut Cx) {
        self.vertical = cx.vertical_look();
        self.row.start(cx);
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

    fn tick(&mut self, cx: &mut Cx, dt: f32) {
        self.vertical = cx.vertical_look();
        if !self.vertical {
            self.row.tick(cx, self.sheet, dt);
        }
    }

    fn footer(&self) -> Footer {
        if self.vertical {
            return Footer::Standard;
        }
        Footer::BackAccept
    }

    fn vertical(&self) -> bool {
        self.vertical
    }

    /// Up and Down step the Remastered list as Left and Right step the sheets of the Original page.
    fn rotate_pad(&self) -> bool {
        self.vertical
    }

    fn list(&self, assets: &UiAssets) -> Option<MenuList> {
        if !self.vertical {
            return None;
        }
        let text = |label: u32| Text::Game(label).resolve(assets).unwrap_or_default();
        let entries = self
            .list_entries()
            .into_iter()
            .map(|(entry, enabled)| match entry {
                Entry::Sheet(sheet) => MenuEntry::plain(text(sheet.label()), enabled),
                Entry::Challenge => MenuEntry::plain(text(label_hash("RIVAL_CHALLENGE_ACTIVATED")), enabled),
            })
            .collect();
        let title = text(label_hash("TITLE_BLACKLIST_NUMBER")).replace("%d", &self.shown.to_string());
        Some(MenuList {
            title,
            message: String::new(),
            entries,
            selected: self.cursor,
            first: 0,
            hints: Vec::new(),
            style: MenuStyle::Sub,
            nudge: None,
            scroll: 0.0,
            aspect: DEFAULT_ASPECT,
        })
    }

    fn pick_row(&mut self, cx: &mut Cx, row: usize, pick: Pick) {
        if row >= self.list_entries().len() {
            return;
        }
        self.cursor = row;
        if matches!(pick, Pick::Press(..)) {
            self.activate(cx);
        }
    }

    // The page has no focused button to answer an accept, so the pad press is the accept.
    fn pad_pressed(&mut self, cx: &mut Cx, bits: u32) {
        if bits & pad::ACCEPT != 0 && self.vertical {
            return self.activate(cx);
        }
        if bits & pad::ACCEPT != 0 {
            self.accept(cx);
        }
    }

    // The mouse takes the screen's own buttons: the two chevrons, the two arrows, the option and the challenge button
    // (the footer's Back and Accept are taken by the front end).
    fn click(&mut self, cx: &mut Cx, point: Vec2) -> bool {
        if !self.vertical
            && let Some(sheet) = self.row.hit(cx, point)
        {
            self.sheet = sheet;
            self.fill_sheet(cx);
            let before = cx.commands.len();
            self.accept(cx);
            if before != cx.commands.len() {
                crate::audio::menu::request(blackbox_feng::ids::BUTTON_PRESSED);
            }
            return true;
        }
        if hit(cx, STEPPER_UP, point) {
            let before = self.shown;
            self.step_rival(cx, true);
            if before != self.shown {
                crate::audio::menu::request(PAD_UP);
            }
            return true;
        }
        if hit(cx, STEPPER_DOWN, point) {
            let before = self.shown;
            self.step_rival(cx, false);
            if before != self.shown {
                crate::audio::menu::request(PAD_DOWN);
            }
            return true;
        }
        if hit(cx, ARROW_LEFT, point) {
            let before = self.sheet;
            self.step_sheet(cx, false);
            if before != self.sheet {
                crate::audio::menu::request(PAD_LEFT);
            }
            return true;
        }
        if hit(cx, ARROW_RIGHT, point) {
            let before = self.sheet;
            self.step_sheet(cx, true);
            if before != self.sheet {
                crate::audio::menu::request(PAD_RIGHT);
            }
            return true;
        }
        if hit(cx, BUTTON_GROUP, point) {
            let before = cx.commands.len();
            self.challenge(cx);
            if before != cx.commands.len() {
                crate::audio::menu::request(blackbox_feng::ids::BUTTON_PRESSED);
            }
            return true;
        }
        if OPTION_PARTS.iter().any(|&part| hit(cx, part, point)) {
            let before = cx.commands.len();
            self.accept(cx);
            if before != cx.commands.len() {
                crate::audio::menu::request(blackbox_feng::ids::BUTTON_PRESSED);
            }
            return true;
        }
        false
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            PAD_LEFT | PAD_RIGHT if self.vertical => {
                let last = self.list_entries().len().saturating_sub(1);
                self.cursor = match message == PAD_RIGHT {
                    true => (self.cursor + 1).min(last),
                    false => self.cursor.saturating_sub(1),
                };
            }
            PAD_UP => self.step_rival(cx, true),
            PAD_DOWN => self.step_rival(cx, false),
            PAD_LEFT => self.step_sheet(cx, false),
            PAD_RIGHT => self.step_sheet(cx, true),
            CHALLENGE_KEY => self.challenge(cx),
            PAD_BACK => self.back(cx),
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "reputation_overview_tests.rs"]
mod tests;
