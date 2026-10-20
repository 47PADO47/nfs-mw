//! What the menu backdrop shows (docs/specs/career-parity-menus.md, section 7). The main menu and the option screens stand on
//! the career-start GTR in the stage room; the career menu shows the profile's last driven car in the same room; the safe
//! house menu puts that car in the safe house interior. A screen names its look (`factory::look_for`), the screen stack
//! reports the look of the topmost screen that has one (`Screens::look`), and the front end swaps the backdrop when it changes
//! (`Frontend::sync_backdrop`).

use crate::career::CareerHandle;

/// A car the backdrop shows: the car type folder under `CARS/` and the preset it is built with, if any.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CarLook {
    pub car: String,
    pub preset: Option<String>,
}

/// The backdrop of a menu.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Look {
    /// The career-start GTR in the stage room: the main menu, the options, the Quick Race menus, the alias manager.
    #[default]
    Main,
    /// The career car in the stage room: the career menu.
    Career(CarLook),
    /// The career car in the safe house interior: the safe house menu and the screens it opens.
    SafeHouse(CarLook),
}

impl CarLook {
    /// The car of the loaded profile: the last car it drove, or the starter car of a career with none.
    pub fn of_profile(_profile: &CareerHandle) -> CarLook {
        CarLook::starter()
    }

    /// The car a career starts with: the red BMW M3 (docs/specs/career-parity-menus.md, section 7).
    pub fn starter() -> CarLook {
        CarLook { car: "BMWM3GTR".to_owned(), preset: None }
    }
}
