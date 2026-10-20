//! The Blacklist screens (`docs/specs/career-safehouse-2.md`, `career-blacklist.md` sections 3 and 5.4, `career-text.md`
//! sections 1 and 2).
//!
//! `SafeHouseRivalChallenge.fng` walks the 15 rivals with Left and Right, from rival 15 down to rival 1, and shows each
//! one's state, name, bounty and requirements. Its challenge button starts the rival's boss race; race mode is not built,
//! so the button only shows whether it is on, and pressing it on is answered with an alert. Back, and Continue, leave for
//! the safe house menu, or for the region unlock screen when the stage byte is one that unlocks a region (8 and 12,
//! `career-regions.md` section 7.1).
//!
//! `SafeHouseRivalBio.fng` shows the rival of the current bin: name, alias, car, strength and bio. Accept and Back leave
//! for the safe house menu.
//!
//! Object names are the FEng name hashes of the install's packages (`nfsmw dump-screen`); the text is looked up by the
//! label hashes of `career-text.md`. The pure parts (the rows, the stepper, the leave target) are in this file's helpers
//! and tested in `blacklist_tests.rs`.

use blackbox_feng::fe_hash_upper;
use blackbox_feng::ids::{PAD_ACCEPT, PAD_BACK, PAD_LEFT, PAD_RIGHT};
use blackbox_text::label_hash;
use nfsmw_career::blacklist::{BinRecord, BinState, Blacklist, Counters, rival_gate_open};
use nfsmw_career::settings::{FIRST_RIVAL_STAGE, Flags};

use super::{
    RIVAL_BIO_CONTENT, RIVAL_CONTENT_START, RIVAL_LIST_READY, RIVAL_OPENED_CAREER, RIVAL_OPENED_PLAIN, post_opening,
};
use crate::career::CareerHandle;
use crate::frontend::confirm::Alert;
use crate::frontend::ids::screen;
use crate::frontend::infractions::money;
use crate::frontend::logic::{Args, Command, Cx, Pick, ScreenLogic};
use crate::ui::theme::{DEFAULT_ASPECT, MenuEntry, MenuList, MenuStyle, MenuValue, RivalInfo, RivalStatus};
use crate::ui::{Text, UiAssets};

/// The alpha of a disabled text (the same value the infractions screen uses).
const DISABLED_ALPHA: u8 = 110;

/// Objects of `SafeHouseRivalChallenge.fng` by name hash. The title group holds the rank ("blacklist %d"), the name group
/// the rival's name, the bounty group the bounty needed, and the defeated group the stamp of a beaten rival.
const CHALLENGE_TITLE: u32 = 0x2426_57CE;
const CHALLENGE_NAME: u32 = 0xb514_e2d8;
const CHALLENGE_BOUNTY: u32 = 0xf91a_59f6;
const CHALLENGE_DEFEATED: u32 = 0x34d4_433b;
/// The challenge button's holder (it takes the focus and sends the press) and its text, by GUID. The holder sends the
/// accept message to the game when it is pressed, the same message as Continue.
const CHALLENGE_HOLDER: u32 = 0x0011_6220;
const CHALLENGE_TEXT: u32 = 0x0011_6221;

/// Objects of `SafehouseRivalBio.fng` by name hash.
const BIO_TITLE: u32 = 0x2426_57CE;
const BIO_NAME: u32 = 0x7ac3_d0c9;
const BIO_AKA: u32 = 0xb1f2_748d;
const BIO_CAR: u32 = 0xcb5b_f41a;
const BIO_STRENGTH: u32 = 0xa6f0_7bf3;
const BIO_TEXT: u32 = 0x27e1_d6d8;

/// The text key of one rival, `BLACKLIST_RIVAL_<NN>_<SUFFIX>` with the bin number in two digits (`career-text.md`
/// section 1).
pub fn rival_key(bin: u8, suffix: &str) -> String {
    format!("BLACKLIST_RIVAL_{bin:02}_{suffix}")
}

/// Where a rival is in the walk down the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RivalState {
    /// Not entered, or entered with the gate closed.
    Locked,
    /// The gate is open: the boss races are unlocked and the rival can be challenged.
    Unlocked,
    /// Every boss race is won.
    Beaten,
}

/// What the challenge screen shows for one rival. The races and challenges lines are not drawn yet (their objects are
/// not identified in the package dump), so those fields are only read by the tests.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RivalRow {
    /// The bin number, which is the rival's rank (15 is the first rival, 1 the last).
    pub bin: u8,
    pub state: RivalState,
    /// Whether the three requirements of the gate are met now (`career-blacklist.md` section 3.1).
    pub gate_open: bool,
    /// Races won and challenges passed so far.
    pub counters: Counters,
    pub races_needed: u32,
    pub challenges_needed: u32,
    pub bounty_needed: u32,
    /// Whether the challenge button is on (`career-blacklist.md` section 3.3).
    pub button_active: bool,
}

/// One row per rival, from rival 15 down to rival 1.
pub fn rival_rows(blacklist: &Blacklist, flags: Flags) -> Vec<RivalRow> {
    (1..=FIRST_RIVAL_STAGE).rev().filter_map(|bin| rival_row(blacklist, bin, flags)).collect()
}

fn rival_row(blacklist: &Blacklist, bin: u8, flags: Flags) -> Option<RivalRow> {
    let record = blacklist.bin(bin)?;
    let counters = record.counters();
    Some(RivalRow {
        bin,
        state: state_of(record),
        gate_open: rival_gate_open(&record.def, counters, blacklist.bounty()),
        counters,
        races_needed: record.def.races_needed,
        challenges_needed: record.def.challenges_needed,
        bounty_needed: record.def.bounty_needed,
        button_active: blacklist.rival_challenge_active(bin, flags, boss_race_open(record)),
    })
}

fn state_of(record: &BinRecord) -> RivalState {
    match record.state {
        Some(BinState::RivalUnlocked) => RivalState::Unlocked,
        Some(BinState::Done) => RivalState::Beaten,
        Some(BinState::RivalLocked) | None => RivalState::Locked,
    }
}

/// True while the rival is unlocked and some boss race of it is not yet won.
fn boss_race_open(record: &BinRecord) -> bool {
    record.state == Some(BinState::RivalUnlocked) && record.rival_races_won.len() < record.def.rival_events.len()
}

/// The bin the stepper starts at: the current bin, clamped to the rivals (bin 16 is the prologue, before rival 15).
pub fn start_bin(current: u8) -> u8 {
    current.clamp(1, FIRST_RIVAL_STAGE)
}

/// One step of the stepper. Right goes to the next rival down the list (the lower bin), Left back up to the earlier
/// one. The walk stops at rival 15 and at rival 1.
pub fn step_bin(selected: u8, right: bool) -> u8 {
    let selected = selected.clamp(1, FIRST_RIVAL_STAGE);
    if right {
        return selected.saturating_sub(1).max(1);
    }
    (selected + 1).min(FIRST_RIVAL_STAGE)
}

/// The screen that follows leaving the rival challenge screen: the safe house menu. The region unlock screen is not
/// shown from here: the world asks for it once, when the stage reaches 12 or 8 (`CareerRequest::RegionUnlock`), so
/// leaving the stepper again at those stages does not repeat it (`career-regions.md` open question 5).
pub fn leave_screen(_stage: u8) -> &'static str {
    screen::MAIN_MENU_SUB
}

/// The current bin of the career, or `None` without a career.
fn current_bin() -> Option<u8> {
    CareerHandle::shared().with(|career| career.blacklist.current_bin())
}

/// The rows of the career's Blacklist, or nothing without a career.
fn career_rows() -> Vec<RivalRow> {
    CareerHandle::shared().with(|career| rival_rows(&career.blacklist, career.settings.flags)).unwrap_or_default()
}

/// The rank title ("blacklist %d") with the bin number.
fn rank_title(cx: &Cx, bin: u8) -> Option<String> {
    let text = Text::Game(label_hash("TITLE_BLACKLIST_NUMBER")).resolve(cx.assets)?;
    Some(text.replace("%d", &bin.to_string()))
}

/// A text of the game's table, empty when it is missing.
fn game_text(assets: &UiAssets, key: &str) -> String {
    Text::Game(label_hash(key)).resolve(assets).unwrap_or_default()
}

/// Where the player stands with the rival of `row` while `current` is the career's bin: a rival beaten is complete, the one
/// the career is at (or one with its gate open) is in progress, and the ones further down the list are locked.
pub fn status_of(row: &RivalRow, current: u8) -> RivalStatus {
    if row.state == RivalState::Beaten {
        return RivalStatus::Complete;
    }
    if row.state == RivalState::Unlocked || row.bin == current {
        return RivalStatus::InProgress;
    }
    RivalStatus::Locked
}

/// What the Remastered look shows of one rival: the name is the entry's label, the rest is here.
fn rival_entry(assets: &UiAssets, bin: u8, status: RivalStatus, bounty: u32) -> MenuEntry {
    let name = game_text(assets, &rival_key(bin, "NAME"));
    let info = RivalInfo {
        rank: bin,
        portrait: fe_hash_upper(&format!("RIVAL_{bin:02}")),
        alias: game_text(assets, &rival_key(bin, "AKA")),
        car: game_text(assets, &rival_key(bin, "CAR")),
        strength: game_text(assets, &rival_key(bin, "STRENGTH")),
        bounty: money(bounty),
        status,
    };
    MenuEntry { label: name, enabled: true, value: MenuValue::Rival(info), icon: None }
}

/// Whether the holder with this GUID has the focus.
fn is_focused(cx: &Cx, guid: u32) -> bool {
    let Some(focus) = cx.rt.current_button(cx.package) else { return false };
    let Some(def) = cx.rt.package(cx.package) else { return false };
    def.objects.get(focus.index).is_some_and(|o| o.guid == guid)
}

/// Opens `next` in place of this screen, with the safe house argument the screens of the safe house pass on.
fn switch(cx: &mut Cx, next: &str) {
    cx.send(Command::Switch(next.to_string(), Args { safehouse: true, ..Args::default() }));
}

/// Leaves for the next screen of [`leave_screen`] at the career's current bin.
fn leave(cx: &mut Cx) {
    let stage = current_bin().unwrap_or(FIRST_RIVAL_STAGE);
    switch(cx, leave_screen(stage));
}

/// The rival challenge stepper (`SafeHouseRivalChallenge.fng`).
struct ChallengeLogic {
    selected: u8,
    /// Whether the Remastered look draws the list of rivals (`Cx::vertical_look`).
    vertical: bool,
}

impl ChallengeLogic {
    fn new() -> Self {
        Self { selected: FIRST_RIVAL_STAGE, vertical: false }
    }

    /// Opens the bio of the selected rival.
    fn view_rival(&self, cx: &mut Cx) {
        let args = Args { safehouse: true, rival: self.selected, ..Args::default() };
        cx.send(Command::Switch(screen::SAFE_HOUSE_RIVAL_BIO.to_string(), args));
    }

    /// The row of the selected rival, when there is a career.
    fn selected_row(&self) -> Option<RivalRow> {
        career_rows().into_iter().find(|row| row.bin == self.selected)
    }

    fn fill(&self, cx: &mut Cx) {
        let Some(row) = self.selected_row() else { return };
        if let Some(title) = rank_title(cx, row.bin) {
            cx.text_group(CHALLENGE_TITLE, &title);
        }
        cx.label_group(CHALLENGE_NAME, label_hash(&rival_key(row.bin, "NAME")));
        cx.text_group(CHALLENGE_BOUNTY, &money(row.bounty_needed));
        cx.hide(CHALLENGE_DEFEATED, row.state != RivalState::Beaten);
        if let Some(text) = cx.guid(CHALLENGE_TEXT) {
            let alpha = if row.button_active { 255 } else { DISABLED_ALPHA };
            cx.rt.set_alpha(text, alpha);
        }
    }

    fn step(&mut self, cx: &mut Cx, right: bool) {
        self.selected = step_bin(self.selected, right);
        self.fill(cx);
    }

    /// Accept on the challenge button starts the boss race, which is not built: an alert when the button is on.
    /// Accept anywhere else is Continue, which leaves like Back.
    fn accept(&self, cx: &mut Cx) {
        if self.vertical {
            return self.view_rival(cx);
        }
        if !is_focused(cx, CHALLENGE_HOLDER) {
            return leave(cx);
        }
        if self.selected_row().is_some_and(|row| row.button_active) {
            cx.send(Command::Alert(Alert::new(Text::Custom("Race mode is not built yet".to_owned()))));
        }
    }
}

impl ScreenLogic for ChallengeLogic {
    fn start(&mut self, cx: &mut Cx) {
        self.vertical = cx.vertical_look();
        post_opening(cx, &[RIVAL_OPENED_CAREER, RIVAL_LIST_READY, RIVAL_CONTENT_START]);
        if let Some(bin) = current_bin() {
            self.selected = start_bin(bin);
        }
        self.fill(cx);
    }

    fn tick(&mut self, cx: &mut Cx, _dt: f32) {
        self.vertical = cx.vertical_look();
    }

    fn vertical(&self) -> bool {
        self.vertical
    }

    /// Up and Down step the list as Left and Right step the package's stepper.
    fn rotate_pad(&self) -> bool {
        self.vertical
    }

    fn list(&self, assets: &UiAssets) -> Option<MenuList> {
        if !self.vertical {
            return None;
        }
        let current = start_bin(current_bin().unwrap_or(FIRST_RIVAL_STAGE));
        let rows = career_rows();
        let entries =
            rows.iter().map(|row| rival_entry(assets, row.bin, status_of(row, current), row.bounty_needed)).collect();
        let beaten = rows.iter().filter(|row| row.state == RivalState::Beaten).count();
        Some(MenuList {
            title: "Blacklist".into(),
            message: format!("{beaten}/{}", rows.len()),
            entries,
            selected: rows.iter().position(|row| row.bin == self.selected).unwrap_or(0),
            first: 0,
            hints: Vec::new(),
            style: MenuStyle::Blacklist,
            nudge: None,
            scroll: 0.0,
            aspect: DEFAULT_ASPECT,
        })
    }

    fn pick_row(&mut self, cx: &mut Cx, row: usize, pick: Pick) {
        let Some(picked) = career_rows().get(row).map(|row| row.bin) else { return };
        if self.selected != picked {
            self.selected = picked;
            self.fill(cx);
        }
        if matches!(pick, Pick::Press(..)) {
            self.view_rival(cx);
        }
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            PAD_LEFT => self.step(cx, false),
            PAD_RIGHT => self.step(cx, true),
            PAD_ACCEPT => self.accept(cx),
            PAD_BACK => leave(cx),
            _ => {}
        }
    }
}

/// The rival bio (`SafeHouseRivalBio.fng`): the rival of the current bin.
struct BioLogic {
    bin: u8,
    /// The rival the screen was opened for, or 0 for the career's current one.
    asked: u8,
    /// Whether the Remastered look draws the page (`Cx::vertical_look`).
    vertical: bool,
}

impl BioLogic {
    fn new(args: Args) -> Self {
        Self { bin: FIRST_RIVAL_STAGE, asked: args.rival, vertical: false }
    }

    fn fill(&self, cx: &mut Cx) {
        if let Some(title) = rank_title(cx, self.bin) {
            cx.text_group(BIO_TITLE, &title);
        }
        cx.label_group(BIO_NAME, label_hash(&rival_key(self.bin, "NAME")));
        cx.label_group(BIO_AKA, label_hash(&rival_key(self.bin, "AKA")));
        cx.label_group(BIO_CAR, label_hash(&rival_key(self.bin, "CAR")));
        cx.label_group(BIO_STRENGTH, label_hash(&rival_key(self.bin, "STRENGTH")));
        cx.label_group(BIO_TEXT, label_hash(&rival_key(self.bin, "BIO")));
    }
}

impl ScreenLogic for BioLogic {
    fn start(&mut self, cx: &mut Cx) {
        self.vertical = cx.vertical_look();
        post_opening(cx, &[RIVAL_OPENED_PLAIN, RIVAL_OPENED_CAREER, RIVAL_BIO_CONTENT]);
        if let Some(bin) = current_bin() {
            self.bin = start_bin(bin);
        }
        if self.asked != 0 {
            self.bin = start_bin(self.asked);
        }
        self.fill(cx);
    }

    fn tick(&mut self, cx: &mut Cx, _dt: f32) {
        self.vertical = cx.vertical_look();
    }

    fn vertical(&self) -> bool {
        self.vertical
    }

    fn list(&self, assets: &UiAssets) -> Option<MenuList> {
        if !self.vertical {
            return None;
        }
        let row = career_rows().into_iter().find(|row| row.bin == self.bin);
        let current = start_bin(current_bin().unwrap_or(FIRST_RIVAL_STAGE));
        let status = row.as_ref().map_or(RivalStatus::Locked, |row| status_of(row, current));
        let bounty = row.map_or(0, |row| row.bounty_needed);
        let title = game_text(assets, "TITLE_BLACKLIST_NUMBER").replace("%d", &self.bin.to_string());
        Some(MenuList {
            title,
            message: game_text(assets, &rival_key(self.bin, "BIO")),
            entries: vec![rival_entry(assets, self.bin, status, bounty)],
            selected: 0,
            first: 0,
            hints: Vec::new(),
            style: MenuStyle::Rival,
            nudge: None,
            scroll: 0.0,
            aspect: DEFAULT_ASPECT,
        })
    }

    fn pick_row(&mut self, _cx: &mut Cx, _row: usize, _pick: Pick) {}

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            PAD_ACCEPT | PAD_BACK => switch(cx, screen::MAIN_MENU_SUB),
            _ => {}
        }
    }
}

/// The logic for `name` (lower case) when it is one of this module's screens.
pub fn make(name: &str, args: Args) -> Option<Box<dyn ScreenLogic>> {
    if name == screen::SAFE_HOUSE_RIVAL_CHALLENGE.to_ascii_lowercase() {
        return Some(Box::new(ChallengeLogic::new()));
    }
    if name == screen::SAFE_HOUSE_RIVAL_BIO.to_ascii_lowercase() {
        return Some(Box::new(BioLogic::new(args)));
    }
    None
}

#[cfg(test)]
#[path = "blacklist_tests.rs"]
mod tests;
