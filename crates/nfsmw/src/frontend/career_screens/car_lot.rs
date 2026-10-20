//! The career car lot (`Car_Select.fng`, `docs/specs/career-car-lot.md`): the cars for sale at the career's stage (Lot
//! tab) and the player's own cars to sell (Owned tab), with the purchase and sale rules of `nfsmw_career::carlot`.
//!
//! Opened from the safe house menu (My Cars) or from a car lot zone in the world: both arrive with `Args::safehouse`.
//! Back returns to the safe house menu (`MainMenu_Sub.fng`) in both cases, since the screen cannot tell them apart.
//!
//! The screen shows one car at a time, as the original's carousel does: the car's name and price, the player's cash,
//! the locked notice for a greyed car, the tab title and the Accept label. Left and Right step through the tab's cars
//! (the package plays the arrow animations itself), Up and Down switch between the Lot and Owned tabs, Accept asks about
//! the shown car, Back leaves for the safe house menu. A purchase or sale is asked with a confirmation whose yes answer
//! is a [`CarLotAction`]; the front end passes it to [`apply`] when the player says yes.

use blackbox_feng::ids::{PAD_ACCEPT, PAD_BACK, PAD_DOWN, PAD_LEFT, PAD_RIGHT, PAD_UP};
use blackbox_text::label_hash;
use nfsmw_career::carlot::sale::MAX_SALE_STAGE;
use nfsmw_career::carlot::stats::{self, CarStats};
use nfsmw_career::carlot::{self, Delivery, Limits, LotCar, LotState, PurchaseError, PurchasePrompt, SaleError};
use nfsmw_career::economy::sale_proceeds;
use nfsmw_career::settings::CareerSettings;
use nfsmw_career::stable::{CUSTOMIZATION_SLOTS, Stable};

use crate::career::{Career, CareerHandle};
use crate::frontend::confirm::{Alert, Confirm};
use crate::frontend::ids::screen;
use crate::frontend::infractions::money;
use crate::frontend::logic::{Args, Command, Cx, ScreenLogic};
use crate::ui::Text;

/// The objects of `Car_Select.fng` the screen fills (guids from `nfsmw dump-screen Car_Select.fng`): the car card's
/// name, the `Cost:` value, the `Funds:` value, the "This Car is Locked" notice, the Sell button, the Accept button's
/// label and the header's title text.
const CAR_NAME: u32 = 0x000E_283B;
const PRICE: u32 = 0x000F_90AB;
const CASH: u32 = 0x000F_90A7;
const LOCKED: u32 = 0x000F_BDEB;
const SELL_BUTTON: u32 = 0x0010_5E06;
const ACCEPT: u32 = 0x000E_2B7E;
const TAB_TITLE: u32 = 0x000F_18D7;

/// One LED strip of a stat bar: its image GUID, its centre x inside the bar's group, its size in pixels and its uv width.
/// The strips are `LED_main.tga` at 32 pixels per uv unit (`[exe][verified]`, `Car_Select.fng`). Both strips of a bar
/// start at the same left edge (x = -60), so the shorter first strip overlays the start of the full one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Strip {
    pub guid: u32,
    pub centre_x: f32,
    pub width: f32,
    pub uv_width: f32,
}

/// The strips of the TOP SPEED bar (group `0xF1737`), the HANDLING bar (`0xF173F`) and the ACCELERATION bar (`0xF172F`).
const TOP_SPEED_STRIPS: [Strip; 2] = [
    Strip { guid: 0x000F_1738, centre_x: -20.0, width: 80.0, uv_width: 2.5 },
    Strip { guid: 0x000F_1739, centre_x: 20.0, width: 160.0, uv_width: 5.0 },
];
const HANDLING_STRIPS: [Strip; 2] = [
    Strip { guid: 0x000F_1740, centre_x: 0.0, width: 120.0, uv_width: 3.75 },
    Strip { guid: 0x000F_1741, centre_x: 20.0, width: 160.0, uv_width: 5.0 },
];
const ACCELERATION_STRIPS: [Strip; 2] = [
    Strip { guid: 0x000F_1730, centre_x: -40.0, width: 40.0, uv_width: 1.25 },
    Strip { guid: 0x000F_1731, centre_x: 20.0, width: 160.0, uv_width: 5.0 },
];

/// What one strip shows for a fraction: its lit uv (the `u1` of its texture rectangle), its new width and centre x, and
/// whether it is shown at all.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StripFill {
    pub guid: u32,
    pub lit_uv: f32,
    pub width: f32,
    pub centre_x: f32,
    pub shown: bool,
}

/// Splits a bar's fraction over its strips. The bar is as wide as its widest strip (`total` uv); the lit part is
/// `fraction * total` uv from the left edge, and every strip lights that part up to its own uv width (the strips share
/// the left edge). A lit strip keeps its pixels per uv and its left edge, so its width and centre move with the lit part.
/// A strip with nothing lit is hidden.
pub fn strip_fills(fraction: f32, strips: &[Strip]) -> Vec<StripFill> {
    let total = strips.iter().map(|strip| strip.uv_width).fold(0.0, f32::max);
    let lit_total = fraction.clamp(0.0, 1.0) * total;
    strips.iter().map(|strip| strip_fill(lit_total, strip)).collect()
}

fn strip_fill(lit_total: f32, strip: &Strip) -> StripFill {
    let lit_uv = lit_total.clamp(0.0, strip.uv_width);
    let width = lit_uv * strip.width / strip.uv_width;
    let left = strip.centre_x - strip.width / 2.0;
    StripFill { guid: strip.guid, lit_uv, width, centre_x: left + width / 2.0, shown: lit_uv > 0.0 }
}

/// Draws one bar: each strip's uv rectangle, width and position for `fraction`.
pub(crate) fn set_bar(cx: &mut Cx, strips: &[Strip; 2], fraction: f32) {
    for fill in strip_fills(fraction, strips) {
        let Some(object) = cx.guid(fill.guid) else { continue };
        cx.rt.set_hidden(object, !fill.shown);
        if !fill.shown {
            continue;
        }
        cx.set_uv(fill.guid, [0.0, 0.0, fill.lit_uv, 1.0]);
        let Some(size) = cx.rt.size(object) else { continue };
        cx.rt.set_size_xy(object, fill.width, size.y);
        let Some(position) = cx.rt.position(object) else { continue };
        cx.rt.set_position_xy(object, fill.centre_x, position.y);
    }
}

/// The language table labels the tab titles and the Accept label show (`Car Lot`, `Career`, `Buy Car`, `Accept`).
/// The owned tab shows `Career` (`0x5415C3F1`, the label the package's own header object carries). `My Cars`
/// (`MAIN_CUSTOMIZE`, `0x1AFD5BE6`) is the quick-race garage, not the career's owned cars (career-car-lot.md section 7).
const LOT_TITLE: u32 = 0xB9D7_1959;
const OWNED_TITLE: u32 = 0x5415_C3F1;
const BUY_LABEL: u32 = 0x7010_BBF2;
const ACCEPT_LABEL: u32 = 0x2B65_A216;

/// The two tabs of the screen: the cars for sale, and the player's own cars.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Lot,
    Owned,
}

impl Tab {
    /// The other tab.
    pub const fn other(self) -> Tab {
        match self {
            Tab::Lot => Tab::Owned,
            Tab::Owned => Tab::Lot,
        }
    }

    /// The language label of the tab's title.
    pub const fn title(self) -> u32 {
        match self {
            Tab::Lot => LOT_TITLE,
            Tab::Owned => OWNED_TITLE,
        }
    }
}

/// What a confirmed question does to the career. Carried by `Command::CarLot` (`flow_career.rs` applies it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarLotAction {
    /// Buy the lot car with this front-end key (`LotCar::frontend_key`).
    Buy { frontend_key: u32, delivery: Delivery },
    /// Sell the owned car with this handle.
    Sell { handle: u32 },
}

/// The key a row is found by when its question is answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Lot(u32),
    Owned(u32),
}

/// One row of the list: the car's shown name, its price (the price to pay, or the half the sale gives), and whether it
/// is greyed (listed but not yet available).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub key: Key,
    pub name: String,
    pub price: u32,
    pub locked: bool,
}

/// What Accept does with a row: nothing, a notice, or a question.
#[derive(Debug, Clone, PartialEq)]
pub enum Start {
    Nothing,
    Refused(Text),
    Ask(Confirm),
}

/// A game label by its English key (`blackbox_text::label_hash`).
fn label(key: &str) -> Text {
    Text::Game(label_hash(key))
}

fn show(cx: &mut Cx, guid: u32, shown: bool) {
    let Some(object) = cx.guid(guid) else { return };
    cx.rt.set_hidden(object, !shown);
}

fn set_text(cx: &mut Cx, guid: u32, text: String) {
    let Some(object) = cx.guid(guid) else { return };
    cx.rt.set_text(object, text);
}

/// Shows the language table's text for `label` on the object `guid` (nothing when the table has no such label).
fn set_label(cx: &mut Cx, guid: u32, label_hash: u32) {
    let text = Text::Game(label_hash).resolve(cx.assets).unwrap_or_default();
    set_text(cx, guid, text);
}

/// The rows of `tab` at the career's stage. Lot rows are the offered cars in catalogue order, locked ones included
/// (`carlot::listed`). Owned rows are the career's owned cars that the catalogue names.
pub fn items(career: &Career, tab: Tab) -> Vec<Item> {
    match tab {
        Tab::Lot => lot_items(&career.data.cars, career.settings.stage),
        Tab::Owned => owned_items(&career.stable, &career.data.cars),
    }
}

fn lot_items(cars: &[LotCar], stage: u8) -> Vec<Item> {
    carlot::listed(cars, stage)
        .into_iter()
        .map(|(car, state)| Item {
            key: Key::Lot(car.frontend_key),
            name: car.name.clone(),
            price: car.price,
            locked: state == LotState::Locked,
        })
        .collect()
}

fn owned_items(stable: &Stable, cars: &[LotCar]) -> Vec<Item> {
    stable
        .cars
        .iter()
        .filter(|car| !car.is_free() && car.is_owned())
        .filter_map(|car| {
            let lot = carlot::by_frontend_key(cars, car.frontend_key)?;
            Some(Item {
                key: Key::Owned(car.handle),
                name: lot.name.clone(),
                price: sale_proceeds(lot.price),
                locked: false,
            })
        })
        .collect()
}

/// The three bar fractions of a row, `[TOP SPEED, HANDLING, ACCELERATION]`, from the car's stats among the lot's
/// (`stats::bar_fractions`). Empty bars for no row, a car the lot does not name, or a car with no stats.
fn shown_fractions(career: &Career, item: Option<&Item>) -> [f32; 3] {
    let Some(item) = item else { return [0.0; 3] };
    let key = match item.key {
        Key::Lot(key) => key,
        Key::Owned(handle) => {
            let Some(record) = career.stable.cars.iter().find(|car| car.handle == handle) else { return [0.0; 3] };
            record.frontend_key
        }
    };
    let Some(index) = career.data.cars.iter().position(|car| car.frontend_key == key) else { return [0.0; 3] };
    let Some(car) = career.data.stats.get(index).copied().flatten() else { return [0.0; 3] };
    let population: Vec<CarStats> = career.data.stats.iter().flatten().copied().collect();
    stats::bar_fractions(&car, &population)
}

/// The answer to Accept on row `cursor` of `tab`: nothing when the row is greyed or missing, else the notice or the
/// question the purchase or sale needs (`carlot` rules, `docs/specs/career-car-lot.md` sections 3 and 4).
pub fn start_at(career: &Career, tab: Tab, cursor: usize) -> Start {
    let Some(item) = items(career, tab).into_iter().nth(cursor) else { return Start::Nothing };
    if item.locked {
        return Start::Nothing;
    }
    match item.key {
        Key::Lot(key) => buy_start(career, key),
        Key::Owned(handle) => sell_start(career, handle),
    }
}

/// The question for buying the lot car `key`: at stage 16 a plain confirm (delivered); at 15 or lower Drive Now.
fn buy_start(career: &Career, key: u32) -> Start {
    let Some(car) = carlot::by_frontend_key(&career.data.cars, key) else { return Start::Nothing };
    if car.state_at(career.settings.stage) != LotState::Buyable {
        return Start::Nothing;
    }
    if let Some(text) = buy_refusal(&career.settings, &career.stable, car) {
        return Start::Refused(text);
    }
    let (message, delivery) = match carlot::prompt_for(career.settings.stage) {
        PurchasePrompt::Confirm => (label("CAR_PURCHASE_CONFIRM"), Delivery::Deliver),
        PurchasePrompt::DriveOrDeliver => (label("CARLOT_BUYMSG"), Delivery::DriveNow),
    };
    let action = CarLotAction::Buy { frontend_key: key, delivery };
    Start::Ask(Confirm::new(message, Command::CarLot(action)))
}

/// The notice that refuses a purchase before the question: not enough cash, or no room in the Safe House (the owned
/// cars at the limit, or no free career record). `None` when the purchase may be asked.
pub fn buy_refusal(settings: &CareerSettings, stable: &Stable, car: &LotCar) -> Option<Text> {
    if car.price > settings.cash {
        return Some(label("CAR_LOT_NO_CASH"));
    }
    let owned_full = carlot::owned_count(stable) >= Limits::SPEC.owned_cars;
    let no_record = stable.careers.iter().all(|record| !record.is_unused());
    if owned_full || no_record {
        return Some(label("CAR_LOT_NO_ROOM"));
    }
    None
}

/// The question for selling the owned car `handle`, or the notice that refuses it (`docs/specs/career-car-lot.md`
/// section 4). Only at stage 15 or lower.
fn sell_start(career: &Career, handle: u32) -> Start {
    if career.settings.stage > MAX_SALE_STAGE {
        return Start::Nothing;
    }
    let Some(record) = career.stable.career_for_handle(handle) else { return Start::Nothing };
    if !record.is_free_to_drive() {
        return Start::Refused(Text::Custom("This car is impounded, you cannot sell it.".into()));
    }
    let free_to_drive = career.stable.live_records().filter(|record| record.is_free_to_drive()).count();
    if free_to_drive <= 1 {
        return Start::Refused(Text::Custom("You cannot sell the only car left in your safe house.".into()));
    }
    let action = CarLotAction::Sell { handle };
    Start::Ask(Confirm::new(label("CS_SELL_CAR"), Command::CarLot(action)))
}

/// Carries out a confirmed action on the career (the front end calls this on yes). Refusals change nothing and come
/// back as the notice to show.
pub fn apply(career: &mut Career, action: &CarLotAction) -> Result<(), Text> {
    let data = career.data.clone();
    match *action {
        CarLotAction::Buy { frontend_key, delivery } => buy_car(career, &data.cars, frontend_key, delivery),
        CarLotAction::Sell { handle } => sell_car(career, &data.cars, handle),
    }
}

fn buy_car(career: &mut Career, cars: &[LotCar], key: u32, delivery: Delivery) -> Result<(), Text> {
    let Some(car) = carlot::by_frontend_key(cars, key) else {
        return Err(Text::Custom("That car is not in the car lot.".into()));
    };
    let Some(customization) = free_customization(&career.stable) else { return Err(label("CAR_LOT_NO_ROOM")) };
    let bought = carlot::buy(&mut career.settings, &mut career.stable, car, delivery, customization, &Limits::SPEC);
    bought.map(|_| ()).map_err(buy_text)
}

fn sell_car(career: &mut Career, cars: &[LotCar], handle: u32) -> Result<(), Text> {
    let sold = carlot::sell(&mut career.settings, &mut career.stable, cars, handle, true);
    sold.map(|_| ()).map_err(sell_text)
}

/// The notice for a refused purchase (`docs/specs/career-car-lot.md` section 3).
fn buy_text(error: PurchaseError) -> Text {
    match error {
        PurchaseError::NotEnoughCash => label("CAR_LOT_NO_CASH"),
        PurchaseError::GarageFull | PurchaseError::NoCareerSlot | PurchaseError::NoCarSlot => label("CAR_LOT_NO_ROOM"),
        PurchaseError::NotOffered => Text::Custom("This car is not for sale.".into()),
        PurchaseError::Locked => Text::Custom("This car is not available yet.".into()),
    }
}

/// The notice for a refused sale (`docs/specs/career-car-lot.md` section 4).
fn sell_text(error: SaleError) -> Text {
    match error {
        SaleError::NotAllowed => Text::Custom("You cannot sell a car here.".into()),
        SaleError::NoCareerRecord | SaleError::NoPrice => Text::Custom("That car is not in your safe house.".into()),
        SaleError::Impounded => Text::Custom("This car is impounded, you cannot sell it.".into()),
        SaleError::LastCar => Text::Custom("You cannot sell the only car left in your safe house.".into()),
    }
}

/// The first customization record no car uses (`[unconfirmed]`: the spec takes a free record; which one is the port's
/// choice). `None` when all are in use.
pub fn free_customization(stable: &Stable) -> Option<u8> {
    let used = |index: u8| stable.cars.iter().any(|car| !car.is_free() && car.customization_index == index);
    (0..CUSTOMIZATION_SLOTS).filter_map(|slot| u8::try_from(slot).ok()).find(|index| !used(*index))
}

/// One step of the highlight through `len` rows, clamped to the list (no wrap). An empty list keeps 0.
pub fn step(cursor: usize, len: usize, down: bool) -> usize {
    if len == 0 {
        return 0;
    }
    let last = len - 1;
    if down {
        return (cursor + 1).min(last);
    }
    cursor.saturating_sub(1)
}

/// The car lot screen. The rows are read from the career each time they are needed, so a purchase or sale shows as soon
/// as the question is answered.
pub struct CarLotLogic {
    tab: Tab,
    cursor: usize,
}

impl CarLotLogic {
    fn new() -> Self {
        Self { tab: Tab::Lot, cursor: 0 }
    }

    /// The rows of the current tab, or none outside career mode.
    fn rows(&self) -> Vec<Item> {
        CareerHandle::shared().with(|career| items(career, self.tab)).unwrap_or_default()
    }

    /// Keeps the highlight on a row after the list changed.
    fn clamp(&mut self) {
        let len = self.rows().len();
        self.cursor = self.cursor.min(len.saturating_sub(1));
    }

    /// Draws the shown car: its name and price, the player's cash, the locked notice, the tab title, the Accept label and
    /// the Sell button (owned cars only). An empty list shows no car.
    fn fill(&self, cx: &mut Cx) {
        let rows = self.rows();
        let cash = CareerHandle::shared().with(|career| career.settings.cash).unwrap_or(0);
        let row = rows.get(self.cursor);
        set_text(cx, CAR_NAME, row.map(|item| item.name.clone()).unwrap_or_default());
        set_text(cx, PRICE, row.map(|item| money(item.price)).unwrap_or_default());
        set_text(cx, CASH, money(cash));
        show(cx, LOCKED, row.is_some_and(|item| item.locked));
        show(cx, SELL_BUTTON, self.tab == Tab::Owned);
        set_label(cx, TAB_TITLE, self.tab.title());
        let accept = match self.tab {
            Tab::Lot => BUY_LABEL,
            Tab::Owned => ACCEPT_LABEL,
        };
        set_label(cx, ACCEPT, accept);
        let fractions = CareerHandle::shared().with(|career| shown_fractions(career, row)).unwrap_or([0.0; 3]);
        set_bar(cx, &TOP_SPEED_STRIPS, fractions[0]);
        set_bar(cx, &HANDLING_STRIPS, fractions[1]);
        set_bar(cx, &ACCELERATION_STRIPS, fractions[2]);
    }

    /// Moves the shown car one step through the tab's list (`forward` is the next car).
    fn move_car(&mut self, cx: &mut Cx, forward: bool) {
        self.cursor = step(self.cursor, self.rows().len(), forward);
        self.fill(cx);
    }

    /// Switches between the Lot and Owned tabs, starting at the tab's first car.
    fn switch_tab(&mut self, cx: &mut Cx) {
        self.tab = self.tab.other();
        self.cursor = 0;
        self.fill(cx);
    }

    fn accept(&self, cx: &mut Cx) {
        let start =
            CareerHandle::shared().with(|career| start_at(career, self.tab, self.cursor)).unwrap_or(Start::Nothing);
        match start {
            Start::Nothing => {}
            Start::Refused(text) => cx.send(Command::Alert(Alert::new(text))),
            Start::Ask(confirm) => cx.send(Command::Confirm(confirm)),
        }
    }
}

impl ScreenLogic for CarLotLogic {
    fn start(&mut self, cx: &mut Cx) {
        self.fill(cx);
    }

    fn resume(&mut self, cx: &mut Cx) {
        self.clamp();
        self.fill(cx);
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            PAD_LEFT => self.move_car(cx, false),
            PAD_RIGHT => self.move_car(cx, true),
            PAD_UP | PAD_DOWN => self.switch_tab(cx),
            PAD_ACCEPT => self.accept(cx),
            PAD_BACK => {
                let back = Args { safehouse: true, ..Args::default() };
                cx.send(Command::Switch(screen::MAIN_MENU_SUB.to_string(), back));
            }
            _ => {}
        }
    }
}

/// The logic for `name` (lower case) when it is the car lot screen.
pub fn make(name: &str, _args: Args) -> Option<Box<dyn ScreenLogic>> {
    if name != screen::CAR_SELECT.to_ascii_lowercase() {
        return None;
    }
    Some(Box::new(CarLotLogic::new()))
}

#[cfg(test)]
#[path = "car_lot_tests.rs"]
mod tests;
