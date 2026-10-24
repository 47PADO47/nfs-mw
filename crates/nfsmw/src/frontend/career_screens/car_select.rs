//! The career car select (`Car_Select.fng` in career select mode, `docs/specs/busted-aftermath.md` sections 3 to 6): the
//! owned cars one at a time with their strike pips, impound stamp, bounty or release cost, and heat meter.
//!
//! It is opened by the bust (after the ticket, with an animation) and by the safe house menu's Car Select. Moving to a car
//! makes it the current car; Accept on a free car goes to the safe house menu; Accept on an impounded car asks to release
//! it; Back is refused while the car the player came with is impounded. The rules are in
//! `nfsmw_career::economy::aftermath`; this file draws them and turns the answers into [`CarSelectAction`]s.

use blackbox_feng::ids::{PAD_ACCEPT, PAD_BACK, PAD_BUTTON0, PAD_LEFT, PAD_RIGHT};
use nfsmw_career::economy::aftermath::{
    CarSelectEntry, ReleaseDialog, ReleaseWay, StrikeMarker, back_is_blocked, car_select_info, release_dialog,
    strike_marker,
};
use nfsmw_career::economy::apply_game_over_check;
use nfsmw_career::economy::markers::MarkerKind;
use nfsmw_career::stable::CareerRecord;

use super::car_lot::{
    ACCELERATION_STRIPS, CAR_NAME, HANDLING_STRIPS, Item, Key, SELL_BUTTON, Start, TOP_SPEED_STRIPS, Tab, items,
    sell_start, set_bar, set_text, show, shown_fractions,
};
use super::car_select_view::{LABEL_RELEASE, LABEL_USE_MARKER, draw_car, hide_header, hide_lot_lines, set_title};
use crate::career::{Career, CareerHandle};
use crate::frontend::confirm::{Alert, Confirm};
use crate::frontend::ids::screen;
use crate::frontend::infractions::money;
use crate::frontend::logic::QuitTo;
use crate::frontend::logic::{Args, Command, Cx, ScreenLogic, Sub};
use crate::ui::Text;

/// What the career car select asks the front end to do (`Command::CareerCar`, `flow_bust.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarSelectAction {
    /// Open the screen with this entry animation (after the ticket, or from the safe house menu).
    Open(CarSelectEntry),
    /// Release the impounded car with this handle.
    Release { handle: u32, marker: bool },
    /// Use an Extra Impound Strike marker on the car with this handle.
    UseStrikeMarker { handle: u32 },
}

const TEXT_MUST_CHOOSE: u32 = 0x6309_31b6;
const TEXT_GAME_OVER: u32 = 0x164b_ed94;
const TEXT_CANNOT_RELEASE: u32 = 0xe96f_a0c5;
const TEXT_MARKER_RELEASE: u32 = 0xed4d_d591;
const TEXT_FEE_QUESTION: u32 = 0xcad5_722e;
const TEXT_ADD_STRIKE: u32 = 0x0ceb_da20;
const TEXT_STRIKE_MAX: u32 = 0xbcae_8539;

/// The pad button 4 and 5 messages (`PAD_BUTTON0` + n): the strike marker and the sale.
const PAD_STRIKE_MARKER: u32 = PAD_BUTTON0 + 4;
const PAD_SELL: u32 = PAD_BUTTON0 + 5;

/// The record of the car with this handle.
fn record_of(career: &Career, handle: u32) -> Option<&CareerRecord> {
    career.stable.career_for_handle(handle)
}

/// The price of the car with this handle (the release fee is a quarter of it), from the lot's catalogue. 0 when the car is
/// not in it.
pub fn cost_of(career: &Career, handle: u32) -> u32 {
    let Some(car) = career.stable.car_by_handle(handle) else { return 0 };
    career.data.cars.iter().find(|lot| lot.frontend_key == car.frontend_key).map_or(0, |lot| lot.price)
}

/// The rows the screen lists. The plain list keeps every row. Player 2's list of a co-op career leaves out player 1's
/// current car and the impounded cars; with no other car free to drive, player 2 shares player 1's car, so co-op can
/// start with a one-car garage (`docs/specs/coop-career.md`, section 3.1 item 3).
pub fn choices(rows: Vec<Item>, career: &Career, coop: bool) -> Vec<Item> {
    if !coop {
        return rows;
    }
    let current = career.settings.car_handle;
    let free = |handle: u32| record_of(career, handle).is_some_and(|r| r.is_free_to_drive());
    let others: Vec<Item> = rows
        .iter()
        .filter(|item| matches!(item.key, Key::Owned(handle) if handle != current && free(handle)))
        .cloned()
        .collect();
    if !others.is_empty() {
        return others;
    }
    rows.into_iter().filter(|item| item.key == Key::Owned(current)).collect()
}

/// A number that changes whenever something the screen draws changes, so a tick can redraw after an action.
fn signature(career: &Career, rows: &[Item], shown: Option<u32>) -> u64 {
    let mut sig = u64::from(career.settings.cash);
    sig = sig.rotate_left(7) ^ u64::from(career.settings.car_handle);
    sig = sig.rotate_left(7) ^ rows.len() as u64;
    for kind in [MarkerKind::JailFree, MarkerKind::ExtraStrike, MarkerKind::ImpoundRelease] {
        sig = sig.rotate_left(5) ^ u64::from(career.markers.count(kind));
    }
    if let Some(record) = shown.and_then(|handle| record_of(career, handle)) {
        sig = sig.rotate_left(5) ^ u64::from(record.state);
        sig = sig.rotate_left(5) ^ record.strike_limit as u64;
        sig = sig.rotate_left(5) ^ record.strikes as u64;
        sig = sig.rotate_left(5) ^ u64::from(record.heat.to_bits());
    }
    sig
}

/// The career car select.
pub struct CareerCarSelectLogic {
    entry: CarSelectEntry,
    cursor: usize,
    /// The car the player came with: Back returns to it, and is refused while it is impounded.
    original: u32,
    /// The signature of what was drawn last.
    drawn: Option<u64>,
    /// The game-over check ran (its notice can only be raised from a tick: the commands of `start` are not run).
    checked: bool,
    /// Player 2's pick in a co-op career: the car goes to the co-op flow and player 1's current car stays as it is.
    coop: bool,
}

impl CareerCarSelectLogic {
    /// The screen with an entry animation, on the current car.
    pub fn new(entry: CarSelectEntry) -> Self {
        Self { entry, cursor: 0, original: 0, drawn: None, checked: false, coop: false }
    }

    /// Player 2's car select of a co-op career (`Args::coop`).
    pub fn coop() -> Self {
        Self { coop: true, ..Self::new(CarSelectEntry::Plain) }
    }

    /// Runs `f` on the career this screen shows: the played one, or in co-op the loaded profile, since player 2 picks
    /// from the career menu before the career resumes.
    fn read<R>(&self, f: impl FnOnce(&mut Career) -> R) -> Option<R> {
        let handle = CareerHandle::shared();
        match self.coop {
            true => handle.with_loaded(f),
            false => handle.with(f),
        }
    }

    /// The rows this screen lists, in the career's order.
    fn list(&self, career: &Career) -> Vec<Item> {
        choices(items(career, Tab::Owned), career, self.coop)
    }

    fn rows(&self) -> Vec<Item> {
        self.read(|career| self.list(career)).unwrap_or_default()
    }

    /// The handle of the shown row.
    fn shown(&self) -> Option<u32> {
        match self.rows().get(self.cursor)?.key {
            Key::Owned(handle) => Some(handle),
            Key::Lot(_) => None,
        }
    }

    /// Draws everything for the shown car.
    fn fill(&mut self, cx: &mut Cx) {
        let drawn = self.read(|career| self.draw(cx, career));
        self.drawn = drawn;
    }

    fn draw(&self, cx: &mut Cx, career: &Career) -> u64 {
        let rows = self.list(career);
        let row = rows.get(self.cursor);
        let handle = row.and_then(|item| match item.key {
            Key::Owned(handle) => Some(handle),
            Key::Lot(_) => None,
        });
        hide_lot_lines(cx);
        set_title(cx);
        set_text(cx, CAR_NAME, row.map(|item| item.name.clone()).unwrap_or_default());
        show(cx, SELL_BUTTON, true);
        let fractions = shown_fractions(career, row);
        set_bar(cx, &TOP_SPEED_STRIPS, fractions[0]);
        set_bar(cx, &HANDLING_STRIPS, fractions[1]);
        set_bar(cx, &ACCELERATION_STRIPS, fractions[2]);
        let record = handle.and_then(|handle| record_of(career, handle).copied());
        match (handle, record) {
            (Some(handle), Some(record)) => self.draw_header(cx, career, &record, cost_of(career, handle)),
            _ => hide_header(cx),
        }
        signature(career, &rows, handle)
    }

    fn draw_header(&self, cx: &mut Cx, career: &Career, record: &CareerRecord, cost: u32) {
        let info = car_select_info(record, cost);
        let strike_markers = career.markers.count(MarkerKind::ExtraStrike);
        let release_markers = career.markers.count(MarkerKind::ImpoundRelease);
        draw_car(cx, &info, self.entry, career.settings.cash, strike_markers, release_markers);
    }

    /// Moves the shown car one step (no wrap) and makes it the current car.
    fn move_car(&mut self, cx: &mut Cx, forward: bool) {
        let len = self.rows().len();
        let cursor = super::car_lot::step(self.cursor, len, forward);
        if cursor == self.cursor {
            return;
        }
        self.cursor = cursor;
        self.entry = CarSelectEntry::Plain;
        self.choose_shown();
        self.fill(cx);
    }

    /// The shown car becomes the career's current car (`SetCurrentCar`). Player 2's pick leaves player 1's car alone.
    fn choose_shown(&self) {
        if self.coop {
            return;
        }
        let Some(handle) = self.shown() else { return };
        CareerHandle::shared().with(|career| career.settings.car_handle = handle);
    }

    fn accept(&mut self, cx: &mut Cx) {
        let Some(handle) = self.shown() else { return };
        if self.coop {
            self.accept_coop(cx, handle);
            return;
        }
        let command = CareerHandle::shared().with(|career| accept_command(career, handle, cx)).flatten();
        match command {
            Some(command) => cx.send(command),
            None => cx.send(Command::Switch(screen::MAIN_MENU_SUB.to_string(), safehouse_args())),
        }
    }

    /// Player 2 seats the car with this handle in the co-op flow, with the gearbox the settings hold (the screen asks none).
    fn accept_coop(&self, cx: &mut Cx, handle: u32) {
        let transmission = cx.settings.transmission;
        cx.send(Command::CoopSecondCar { handle, transmission });
    }

    fn back(&mut self, cx: &mut Cx) {
        if self.coop {
            cx.send(Command::Switch(screen::MAIN_MENU_SUB.to_string(), career_menu_args()));
            return;
        }
        let blocked = CareerHandle::shared()
            .with(|career| record_of(career, self.original).is_some_and(back_is_blocked))
            .unwrap_or(false);
        if blocked {
            cx.send(Command::Alert(notice(Text::Game(TEXT_MUST_CHOOSE))));
            return;
        }
        let original = self.original;
        CareerHandle::shared().with(|career| {
            if record_of(career, original).is_some() {
                career.settings.car_handle = original;
            }
        });
        cx.send(Command::Switch(screen::MAIN_MENU_SUB.to_string(), safehouse_args()));
    }

    fn strike_marker(&self, cx: &mut Cx) {
        let Some(handle) = self.shown() else { return };
        let command = CareerHandle::shared()
            .with(|career| {
                let record = record_of(career, handle)?;
                match strike_marker(record, &career.markers) {
                    StrikeMarker::Offer => Some(Command::Confirm(Confirm::new(
                        Text::Game(TEXT_ADD_STRIKE),
                        Command::CareerCar(CarSelectAction::UseStrikeMarker { handle }),
                    ))),
                    StrikeMarker::AtMaximum => Some(Command::Alert(notice(Text::Game(TEXT_STRIKE_MAX)))),
                    StrikeMarker::Nothing => None,
                }
            })
            .flatten();
        if let Some(command) = command {
            cx.send(command);
        }
    }

    fn sell(&self, cx: &mut Cx) {
        let Some(handle) = self.shown() else { return };
        let start = CareerHandle::shared().with(|career| sell_start(career, handle)).unwrap_or(Start::Nothing);
        match start {
            Start::Nothing => {}
            Start::Refused(text) => cx.send(Command::Alert(notice(text))),
            Start::Ask(confirm) => cx.send(Command::Confirm(confirm)),
        }
    }

    /// The game-over check of the car select: no free car and no Impound Release marker ends the career.
    fn check_game_over(&self, cx: &mut Cx) {
        let over = CareerHandle::shared()
            .with(|career| {
                let release = career.markers.count(MarkerKind::ImpoundRelease) > 0;
                !career.stable.cars.is_empty() && apply_game_over_check(&mut career.settings, &career.stable, release)
            })
            .unwrap_or(false);
        if !over {
            return;
        }
        let alert = Alert::then(Text::Game(TEXT_GAME_OVER), Command::QuitTo(QuitTo::Main));
        cx.send(Command::Alert(Alert { title: Text::Literal(String::new()), ..alert }));
    }
}

/// The notice with an empty title, as the original's one-button dialogs are.
fn notice(message: Text) -> Alert {
    Alert { title: Text::Literal(String::new()), message, on_close: None }
}

fn safehouse_args() -> Args {
    Args { safehouse: true, ..Args::default() }
}

/// The career main menu (Resume Career, Start New Career, Load), as the career flow opens it.
fn career_menu_args() -> Args {
    Args { sub: Sub::Career, ..Args::default() }
}

/// What Accept does on the car with this handle: the release choice for an impounded car, nothing (leave) for a free one.
fn accept_command(career: &Career, handle: u32, cx: &Cx) -> Option<Command> {
    let record = record_of(career, handle)?;
    if record.is_free_to_drive() {
        return None;
    }
    let cost = cost_of(career, handle);
    let fee = nfsmw_career::stable::impound::release_fee(cost);
    let release = |marker: bool| Command::CareerCar(CarSelectAction::Release { handle, marker });
    let fee_question = || {
        let template = Text::Game(TEXT_FEE_QUESTION).resolve().unwrap_or_default();
        Confirm::new(Text::Literal(template.replace("%$d", &money(fee))), release(false))
    };
    Some(match release_dialog(record, &career.settings, cost, &career.markers) {
        ReleaseDialog::Cannot | ReleaseDialog::Choose { cash: false, marker: false } => {
            Command::Alert(notice(Text::Game(TEXT_CANNOT_RELEASE)))
        }
        ReleaseDialog::Choose { cash: true, marker: false } => Command::Confirm(fee_question()),
        ReleaseDialog::Choose { cash: false, marker: true } => {
            let mut ask = Confirm::new(Text::Game(TEXT_MARKER_RELEASE), release(true));
            ask.yes = Text::Game(LABEL_USE_MARKER);
            Command::Confirm(ask)
        }
        ReleaseDialog::Choose { cash: true, marker: true } => {
            let mut ask = Confirm::new(Text::Game(TEXT_MARKER_RELEASE), release(true));
            ask.yes = Text::Game(LABEL_USE_MARKER);
            ask.no = Text::Game(LABEL_RELEASE);
            Command::Confirm(ask.or_else(Command::Confirm(fee_question())))
        }
    })
}

/// Applies a release the player confirmed. The notice to show when it is refused.
pub fn release(career: &mut Career, handle: u32, marker: bool) -> Result<(), Text> {
    let cost = cost_of(career, handle);
    let way = if marker { ReleaseWay::Marker } else { ReleaseWay::Cash };
    let Some(record) = career.stable.career_for_handle_mut(handle) else {
        return Err(Text::Literal("That car is not in your safe house.".into()));
    };
    nfsmw_career::economy::aftermath::release(way, &mut career.settings, record, cost, &mut career.markers)
        .map_err(|_| Text::Game(TEXT_CANNOT_RELEASE))
}

/// Applies an Extra Impound Strike marker the player confirmed.
pub fn use_strike_marker(career: &mut Career, handle: u32) -> Result<(), Text> {
    let Some(record) = career.stable.career_for_handle_mut(handle) else {
        return Err(Text::Literal("That car is not in your safe house.".into()));
    };
    match nfsmw_career::economy::aftermath::use_strike_marker(record, &mut career.markers) {
        true => Ok(()),
        false => Err(Text::Game(TEXT_STRIKE_MAX)),
    }
}

impl ScreenLogic for CareerCarSelectLogic {
    fn start(&mut self, cx: &mut Cx) {
        let (original, cursor) = self
            .read(|career| {
                let current = career.settings.car_handle;
                let rows = self.list(career);
                let cursor = rows.iter().position(|item| item.key == Key::Owned(current)).unwrap_or(0);
                (current, cursor)
            })
            .unwrap_or_default();
        self.original = original;
        self.cursor = cursor;
        self.choose_shown();
        self.fill(cx);
        // The entry animation plays once: later redraws show the car as it stands.
        self.entry = CarSelectEntry::Plain;
    }

    fn resume(&mut self, cx: &mut Cx) {
        let len = self.rows().len();
        self.cursor = self.cursor.min(len.saturating_sub(1));
        self.fill(cx);
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            PAD_LEFT => self.move_car(cx, false),
            PAD_RIGHT => self.move_car(cx, true),
            PAD_ACCEPT => self.accept(cx),
            PAD_BACK => self.back(cx),
            PAD_STRIKE_MARKER => self.strike_marker(cx),
            PAD_SELL => self.sell(cx),
            _ => {}
        }
    }

    /// An action (a release, a sale, a marker) changes the career after the question closed: the next frame redraws.
    fn tick(&mut self, cx: &mut Cx, _dt: f32) {
        if !self.checked {
            self.checked = true;
            self.check_game_over(cx);
        }
        let now = self.read(|career| {
            let rows = self.list(career);
            let shown = rows.get(self.cursor).and_then(|item| match item.key {
                Key::Owned(handle) => Some(handle),
                Key::Lot(_) => None,
            });
            signature(career, &rows, shown)
        });
        if now.is_some() && now != self.drawn {
            let len = self.rows().len();
            self.cursor = self.cursor.min(len.saturating_sub(1));
            self.fill(cx);
        }
    }
}

#[cfg(test)]
#[path = "car_select_tests.rs"]
mod tests;
