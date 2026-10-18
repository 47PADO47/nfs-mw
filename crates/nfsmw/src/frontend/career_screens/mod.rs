//! The career screens (`docs/specs/career-*.md`): the safe house menu, the Blacklist, the rap sheet, the save
//! screens, the region unlock, the car lot and the SMS inbox. They read and change the career through
//! `CareerHandle::shared()` (the screens are made by name and get no constructor data).

pub mod blacklist;
pub mod car_lot;
pub mod rap_sheet;
pub mod region_unlock;
pub mod safehouse_menu;
pub mod save;
pub mod sms;

use super::logic::{Args, ScreenLogic};

/// The logic of the career screen called `name` (lower case), or `None` when `name` is no career screen.
pub fn make(name: &str, args: Args) -> Option<Box<dyn ScreenLogic>> {
    blacklist::make(name, args)
        .or_else(|| car_lot::make(name, args))
        .or_else(|| rap_sheet::make(name, args))
        .or_else(|| region_unlock::make(name, args))
        .or_else(|| save::make(name, args))
        .or_else(|| sms::make(name, args))
}
