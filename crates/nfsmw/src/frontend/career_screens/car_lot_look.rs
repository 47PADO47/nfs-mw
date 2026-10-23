//! What the car lot shows around its cars: the room's car for a row, and the package's sample widgets put right.

use nfsmw_career::carlot;

use super::car_lot::{Item, Key, set_text, show};
use crate::career::Career;
use crate::frontend::logic::Cx;
use crate::frontend::look::{CarLook, CarRecordRef};

/// The debug label and the manufacturer logos of the carousel, which the package fills with a sample car; the lot shows the
/// car's name instead. The counter's numbers: the car's place in the list (right aligned, left of the slash) and the list's
/// length (two copies of each, one per layer).
const DEBUG_LABEL: u32 = 0x000F_04B1;
const MAKER_LOGOS: [u32; 2] = [0x000D_E35C, 0x000D_E22C];
const PLACE_NUMBERS: [u32; 2] = [0x000E_9624, 0x000E_9604];
const TOTAL_NUMBERS: [u32; 2] = [0x000E_9629, 0x000E_9616];

/// Puts the package's sample widgets right: the manufacturer logos and the debug label are hidden (the car's name is
/// shown instead), and the counter reads the shown car's place in a list of `len` cars.
pub(super) fn clean_package(cx: &mut Cx, len: usize, cursor: usize) {
    for guid in MAKER_LOGOS.into_iter().chain([DEBUG_LABEL]) {
        show(cx, guid, false);
    }
    let place = if len == 0 { 0 } else { cursor + 1 };
    for guid in PLACE_NUMBERS {
        set_text(cx, guid, place.to_string());
    }
    for guid in TOTAL_NUMBERS {
        set_text(cx, guid, len.to_string());
    }
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
