//! What the car lot shows around its cars: the room's car for a row, and the package's sample widgets put right.

use nfsmw_career::carlot;
use nfsmw_career::carlot::stats::{self, CarStats};
use nfsmw_career::carlot::{LotCar, LotState};
use nfsmw_career::economy::sale_proceeds;
use nfsmw_career::stable::Stable;

use super::car_lot::{Item, Key, set_text, show};
use super::car_lot_bars::{ACCELERATION_STRIPS, HANDLING_STRIPS, TOP_SPEED_STRIPS, set_bar};
use crate::career::Career;
use crate::frontend::infractions::money;
use crate::frontend::logic::Cx;
use crate::frontend::look::{CarLook, CarRecordRef};
use crate::frontend::quick_play::fill::logo_textures;
use crate::frontend::quick_play::pool::{self, CarPool, PoolCar};

/// The objects of `Car_Select.fng` the lot draws, by name hash (the custom race's car select draws the same package,
/// `custom_race/car_select.rs`).
const MAKER_LOGO: u32 = 0x3E01_AD1D;
const MODEL_LOGO: u32 = 0xB05D_D708;
const NUMBER: u32 = 0x6F25_A248;
const NUMBER_OF: u32 = 0xB203_7BDC;
const LOCKED_GROUP: u32 = 0xD0F7_C7CC;
/// The car name strings (the model-name picture names the car instead; one shares its name with the debug label) and the heat
/// meter, which belongs to the career's own car select.
const HIDDEN: [u32; 3] = [0xD6D3_2016, 0x79D6_E45C, 0xBDE6_E3BA];
/// The package's own Back, Accept and Showcase buttons, which the footer's hints replace (they answer a click as an Accept).
const OWN_BUTTONS: [u32; 3] = [0xC0A3_2823, 0x73DC_B662, 0x4B89_0CDB];
/// Posted to the package when the lot opens: the `Funds:` and `Cost:` lines and the Showcase button appear (the groups answer it
/// with their own scripts).
pub(super) const LOT_MODE: u32 = 0x5415_E304;
/// The car name strings the package fills with sample text, and the "Showcase" string the footer's hint replaces: blanked as
/// well as hidden, since the package's own scripts show a hidden string again.
const DEBUG_LABEL: u32 = 0x000F_04B1;
const BLANKED: [u32; 3] = [0xD6D3_2016, 0x79D6_E45C, 0xC5E1_D8A4];
const SCRIPT_SHOW: u32 = 0x001C_A7C0;
const SCRIPT_HIDE: u32 = 0x0016_A259;
/// The `Funds:`/`Cost:` values and the Sell button.
const COST_VALUE: u32 = 0x000F_90AB;
const FUNDS_VALUE: u32 = 0x000F_90A7;

/// What one frame of the lot shows.
pub(super) struct Shown<'a> {
    /// How many cars the tab lists, and which one is shown.
    pub len: usize,
    pub cursor: usize,
    pub row: Option<&'a Item>,
    pub cash: u32,
}

/// The car of the shared car pool a row stands for (the pool holds the drivable cars with their folder, manufacturer and
/// measured stats, which Quick Play and the custom race show too).
pub(super) fn pool_car(name: &str) -> Option<(PoolCar, std::sync::Arc<CarPool>)> {
    let pool = pool::cars()?;
    let car = pool.cars.iter().find(|car| car.name.eq_ignore_ascii_case(name))?.clone();
    Some((car, pool))
}

/// Draws the package around the shown car: the counter, the manufacturer logo and model picture in place of the name, the
/// bars, the lock notice, and the cost and funds lines; the heat meter of the career's car select stays hidden.
pub(super) fn draw(cx: &mut Cx, shown: &Shown<'_>) {
    for name in HIDDEN {
        cx.hide(name, true);
    }
    for name in OWN_BUTTONS {
        cx.hide(name, true);
    }
    for name in BLANKED {
        cx.text_object(name, "");
    }
    // The debug label shares the car name strings' name hash, which finds only the first of them.
    show(cx, DEBUG_LABEL, false);
    let place = if shown.len == 0 { 0 } else { shown.cursor + 1 };
    cx.text_group(NUMBER, &place.to_string());
    cx.text_group(NUMBER_OF, &shown.len.to_string());
    let car = shown.row.and_then(|row| pool_car(&row.name));
    match &car {
        Some((car, _)) => {
            let (maker, model) = logo_textures(cx.assets, car);
            for (name, texture) in [(MAKER_LOGO, maker), (MODEL_LOGO, model)] {
                if let Some(o) = cx.object(name) {
                    cx.rt.set_hidden(o, false);
                    cx.rt.set_texture(o, texture);
                }
            }
        }
        None => {
            for name in [MAKER_LOGO, MODEL_LOGO] {
                cx.hide(name, true);
            }
        }
    }
    let [top, handling, acceleration] = match &car {
        Some((PoolCar { stats: Some(own), .. }, pool)) => stats::bar_fractions(own, &pool.population),
        _ => [0.0; 3],
    };
    set_bar(cx, &TOP_SPEED_STRIPS, top);
    set_bar(cx, &HANDLING_STRIPS, handling);
    set_bar(cx, &ACCELERATION_STRIPS, acceleration);
    let locked = shown.row.is_some_and(|row| row.locked);
    cx.script(LOCKED_GROUP, if locked { SCRIPT_SHOW } else { SCRIPT_HIDE });
    set_text(cx, COST_VALUE, shown.row.map(|row| money(row.price)).unwrap_or_default());
    set_text(cx, FUNDS_VALUE, money(shown.cash));
}

/// The car a row shows in the room: a lot car by its catalogue record, an owned car by its record in the stable.
pub(super) fn item_look(career: &Career, item: &Item) -> Option<CarLook> {
    match item.key {
        Key::Lot(key) => {
            let car = carlot::by_frontend_key(&career.data.cars, key)?;
            let record =
                CarRecordRef { handle: car.handle, frontend_key: car.frontend_key, vehicle_key: car.vehicle_key };
            Some(CarLook { car: car.name.clone(), preset: None, record: Some(record) })
        }
        Key::Owned(handle) => Some(CarLook::of_handle(career, handle)),
    }
}

/// The car the lot's Showcase shows and where the lot was left, kept for the Showcase screen and for the lot's return.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ShowcaseCar {
    /// The catalogue name of the car (the logos are named after it).
    pub name: String,
    pub look: CarLook,
    pub tab: super::car_lot::Tab,
    pub cursor: usize,
}

static SHOWCASE: std::sync::Mutex<Option<ShowcaseCar>> = std::sync::Mutex::new(None);

/// Remembers the car of the lot's Showcase.
pub(super) fn set_showcase(car: ShowcaseCar) {
    *SHOWCASE.lock().unwrap_or_else(|e| e.into_inner()) = Some(car);
}

/// The car of the lot's Showcase, when the lot sent one.
pub(crate) fn showcased() -> Option<ShowcaseCar> {
    SHOWCASE.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Where the lot was left for the Showcase, taken once when the lot opens again.
pub(super) fn take_return() -> Option<(super::car_lot::Tab, usize)> {
    let car = SHOWCASE.lock().unwrap_or_else(|e| e.into_inner()).take()?;
    Some((car.tab, car.cursor))
}

/// The logos of a showcased car on the Showcase package, by the catalogue name.
pub(crate) fn showcase_logos(assets: &crate::ui::UiAssets, name: &str) -> Option<(u32, u32)> {
    let (car, _) = pool_car(name)?;
    Some(logo_textures(assets, &car))
}

/// The lot's cars of `cars` at `stage` as rows: the offered ones in catalogue order, locked ones included, without the cars the
/// world cannot drive. The install's add-on cars with no physics (`levin`, `trueno`...) cannot start a world, and the shared
/// car pool (`quick_play::pool`) lists the drivable ones; until the pool is read every row stays.
pub(super) fn lot_rows(cars: &[LotCar], stage: u8) -> Vec<Item> {
    let rows: Vec<Item> = carlot::listed(cars, stage)
        .into_iter()
        .map(|(car, state)| Item {
            key: Key::Lot(car.frontend_key),
            name: car.name.clone(),
            price: car.price,
            locked: state == LotState::Locked,
        })
        .collect();
    let Some(pool) = pool::cars().filter(|pool| !pool.cars.is_empty()) else { return rows };
    rows.into_iter().filter(|row| pool.cars.iter().any(|car| car.name.eq_ignore_ascii_case(&row.name))).collect()
}

/// The career's owned cars that the catalogue names, as rows priced at what a sale gives.
pub(super) fn owned_rows(stable: &Stable, cars: &[LotCar]) -> Vec<Item> {
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

/// The three bar fractions of a row, `[TOP SPEED, HANDLING, ACCELERATION]`: the car's measured stats among the shared pool's
/// (the same bars Quick Play and the custom race show), else among the lot's own. Empty bars for no row or a car with no stats.
pub(super) fn shown_fractions(career: &Career, item: Option<&Item>) -> [f32; 3] {
    let Some(item) = item else { return [0.0; 3] };
    if let Some((PoolCar { stats: Some(own), .. }, pool)) = pool_car(&item.name) {
        return stats::bar_fractions(&own, &pool.population);
    }
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
