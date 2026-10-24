//! Controllers for the authored array, cart and tuning widgets.

mod cart;
mod dialog;
mod manager;
mod tuning;

use super::{GarageUi, Page};
use crate::frontend::logic::ScreenLogic;

pub(super) fn logic(handle: GarageUi, page: Page) -> Box<dyn ScreenLogic> {
    match page {
        Page::Manager | Page::AddCar => Box::new(manager::Manager::new(handle, page)),
        Page::Cart { .. } => Box::new(cart::Cart::new(handle, page)),
        Page::Tuning => Box::new(tuning::Tuning::new(handle)),
        _ => Box::new(dialog::Dialog::new(handle, page)),
    }
}
