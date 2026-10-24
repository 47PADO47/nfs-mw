//! What the menu backdrop shows (docs/specs/career-parity-menus.md, section 7; docs/specs/menu-backdrop-looks.md). The main
//! menu and the option screens stand on the career-start GTR in the stage room; the career menu shows the profile's last
//! driven car in the same room; the safe house menu puts that car in the safe house interior. A screen names its look
//! (`factory::look_for`), the screen stack reports the look of the topmost screen that has one (`Screens::look`), and the
//! front end swaps the backdrop when it changes (`Frontend::sync_backdrop`).

use nfsmw_career::settings::{STARTER_CAR_HANDLE, STARTER_PRESET_NAME};

use crate::career::{Career, CareerHandle};

#[path = "look_resolve.rs"]
mod resolve;

/// The car type folder of the car the default look and the career start show: the BMW M3 E46 with the GTR body kit and
/// the big rear wing, in its stock red paint. [verified: `nfsmw view-car BMWM3GTRE46`]
pub const DEFAULT_CAR: &str = "BMWM3GTRE46";

/// A car the backdrop shows: the car type folder under `CARS/` and the preset it is built with, if any.
///
/// A look made from a profile's car also keeps what the save says about the car (`record`), because naming the folder
/// needs the install's tables (`resolve`, run when the backdrop is built), which `of_profile` does not read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CarLook {
    pub car: String,
    pub preset: Option<String>,
    /// The profile's car record, when the look comes from one that `car` and `preset` do not already name.
    pub record: Option<CarRecordRef>,
}

/// What a profile's car record holds: enough to find the car among the install's car types and preset rides.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CarRecordRef {
    /// The record's handle: a small slot number, or the name hash of a preset ride.
    pub handle: u32,
    /// The record's front-end collection key (`career-car-lot.md` section 1).
    pub frontend_key: u32,
    /// The record's vehicle collection key: the hash of the car type's name for a stock car.
    pub vehicle_key: u32,
}

/// The backdrop of a menu.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Look {
    /// The career-start GTR in the stage room: the main menu, the options, the Quick Race menus, the alias manager.
    #[default]
    Main,
    /// A car in the stage room: the career menu (the profile's car) and the Quick Play brief (the car it picked).
    Car(CarLook),
    /// The career car in the safe house interior: the safe house menu and the screens it opens.
    SafeHouse(CarLook),
    /// A car in the car lot's own room: the lot zones' car select and the first car's purchase.
    CarLot(CarLook),
    /// The Showcase of the car lot: the lot's room, with the camera the arrow keys turn (`career_screens/car_lot.rs`).
    LotShowcase(CarLook),
    /// A car in the stage room that the arrow keys turn and the zoom keys bring nearer: the Showcase of the custom race's car
    /// select (`Showcase.fng`, docs/specs/custom-race.md section 6).
    Showcase(CarLook),
}

impl Look {
    /// A look from the command line (`view-screen --look KIND --car CAR[:PRESET]`): `main`, `car`, `safehouse` or `showcase`. The car
    /// of the last two is `car` when given (a `CARS/` folder and an optional preset), else [`CarLook::default_car`].
    pub fn from_cli(kind: &str, car: Option<&str>) -> anyhow::Result<Look> {
        let car = match car.map(|car| car.split_once(':').unwrap_or((car, ""))) {
            Some((folder, "")) => CarLook::named(folder, None),
            Some((folder, preset)) => CarLook::named(folder, Some(preset)),
            None => CarLook::default_car(),
        };
        match kind.to_ascii_lowercase().as_str() {
            "main" => Ok(Look::Main),
            "car" => Ok(Look::Car(car)),
            "safehouse" => Ok(Look::SafeHouse(car)),
            "carlot" => Ok(Look::CarLot(car)),
            "showcase" => Ok(Look::Showcase(car)),
            "lotshowcase" => Ok(Look::LotShowcase(car)),
            other => anyhow::bail!("unknown look {other:?} (main, car, safehouse, carlot, showcase, lotshowcase)"),
        }
    }
}

impl CarLook {
    /// A car and preset by name, with no record behind them.
    pub fn named(car: &str, preset: Option<&str>) -> CarLook {
        CarLook { car: car.to_owned(), preset: preset.map(str::to_owned), record: None }
    }

    /// The car of the loaded profile: the last car it drove (the current car of its settings), or [`CarLook::default_car`]
    /// when no profile is loaded or its current car is not in its stable. Reads only the loaded profile, no files.
    pub fn of_profile(profile: &CareerHandle) -> CarLook {
        profile.with_loaded(|career| CarLook::of_career(career)).unwrap_or_else(CarLook::default_car)
    }

    /// The car of `career`'s current-car handle.
    fn of_career(career: &Career) -> CarLook {
        CarLook::of_handle(career, career.settings.car_handle)
    }

    /// The car of the record with `handle` in `career`'s stable (a co-op player's car, `flow_coop.rs`).
    pub fn of_handle(career: &Career, handle: u32) -> CarLook {
        // A new career starts on the preset ride `M3GTRCAREERSTART`; its record may not be in the stable yet.
        if handle == STARTER_CAR_HANDLE {
            return CarLook::career_start();
        }
        let Some(record) = career.stable.car_by_handle(handle) else { return CarLook::default_car() };
        // The lot's name for the record's front-end collection is a folder name for most cars (`tt`, `gti`, `cobaltss`).
        let name =
            career.data.cars.iter().find(|car| car.frontend_key == record.frontend_key).map(|car| car.name.clone());
        CarLook {
            car: name.unwrap_or_default(),
            preset: None,
            record: Some(CarRecordRef {
                handle: record.handle,
                frontend_key: record.frontend_key,
                vehicle_key: record.vehicle_key,
            }),
        }
    }

    /// The car a profile with no usable current car, or no profile, shows: the red BMW M3 E46 GTR.
    pub fn default_car() -> CarLook {
        CarLook::named(DEFAULT_CAR, None)
    }

    /// The car a new career starts with: the `M3GTRCAREERSTART` preset of the E46 GTR, white with a wing.
    pub fn career_start() -> CarLook {
        CarLook::named(DEFAULT_CAR, Some(STARTER_PRESET_NAME))
    }

    /// The car and preset to load from `dir`: the look's own when it names them, else found from the profile's car record
    /// among the install's preset rides and car types. Falls back to the default car when nothing matches.
    pub fn resolve(&self, dir: &game_install::GameDir) -> (String, Option<String>) {
        resolve::resolve(self, dir)
    }
}

#[cfg(test)]
#[path = "look_tests.rs"]
mod tests;
