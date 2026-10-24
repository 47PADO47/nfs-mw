//! How the menu backdrop gets built without stalling the frame: its CPU work (reading and assembling the car, reading and
//! parsing the room's stage file) runs on a worker thread, and the frame thread uploads what comes back, a step a frame
//! (`MenuScene::update`): the room first, then the car. Until the car is up the backdrop draws nothing.

use std::sync::Arc;
use std::thread::JoinHandle;

use game_install::GameDir;
use nfsmw_data::car::{CarModel, LoadOptions};

use crate::frontend::look::{CarLook, Look};
use crate::frontend::room::RoomData;

/// The car the menus show: the career-start M3 GTR, a preset of the `BMWM3GTRE46` car type with its livery and
/// spoiler, as the original's menu shows it. [ours] The original shows it on a platform stage
/// (`FRONTEND/PLATFORMS/PLATFORMCRIB.BIN`, see docs/specs/frontend-menus.md); here it stands on the car viewer's floor.
const MENU_CAR: &str = "BMWM3GTRE46";
const MENU_PRESET: &str = "M3GTRCAREERSTART";

/// What the worker read: the car of the look and the room of its stage. With no car there is no room either (the
/// backdrop stays flat).
pub(super) struct Staged {
    pub car: Option<(CarModel, LoadOptions)>,
    pub room: Option<Arc<RoomData>>,
}

/// Where the backdrop's build is.
#[derive(Default)]
pub(super) enum Build {
    /// Nothing left to do: built, failed, or never started.
    #[default]
    Idle,
    /// The worker is reading.
    Reading(JoinHandle<Staged>),
    /// The room is up; the car is read and waits for its upload.
    Car(Box<CarModel>, LoadOptions),
}

impl Build {
    /// Starts reading the backdrop of `look` on a worker thread. A thread that cannot start leaves the flat backdrop.
    pub fn start(dir: GameDir, look: Look) -> Self {
        let spawned = std::thread::Builder::new().name("menu-backdrop".to_owned()).spawn(move || read(&dir, &look));
        match spawned {
            Ok(worker) => Self::Reading(worker),
            Err(e) => {
                log::warn!("the menu backdrop is not built: {e}");
                Self::Idle
            }
        }
    }

    /// Whether anything is still to be read or uploaded.
    pub fn pending(&self) -> bool {
        !matches!(self, Self::Idle)
    }
}

/// Reads the car of `look` and the room of its stage.
fn read(dir: &GameDir, look: &Look) -> Staged {
    let _trace = blackbox_trace::scope("Frontend: read the menu backdrop");
    let Some(car) = load_car(dir, look) else { return Staged { car: None, room: None } };
    let room = match RoomData::read(dir, look.stage()) {
        Ok(room) => Some(room),
        Err(e) => {
            log::warn!("the menu room is not drawn: {e:#}");
            None
        }
    };
    Staged { car: Some(car), room }
}

/// The car the look shows, assembled from the install: the main look's career-start GTR, or the look's own car. A car
/// that cannot be loaded gives way to the default car; with that missing too, `None` leaves the flat backdrop.
fn load_car(dir: &GameDir, look: &Look) -> Option<(CarModel, LoadOptions)> {
    let (car, preset) = match look {
        Look::Main => (MENU_CAR.to_owned(), Some(MENU_PRESET.to_owned())),
        Look::Car(look)
        | Look::SafeHouse(look)
        | Look::Customization(look)
        | Look::Backroom(look)
        | Look::CarLot(look)
        | Look::LotShowcase(look)
        | Look::Showcase(look) => look.resolve(dir),
    };
    let default = CarLook::default_car();
    let attempts = [(car, preset), (default.car, default.preset)];
    for (car, preset) in attempts {
        let options = LoadOptions { lod: 'A', all_parts: false, preset };
        match nfsmw_data::car::load(dir, &car, &options) {
            Ok(model) => return Some((model, options)),
            Err(e) => log::warn!("the menu backdrop has no car ({car}): {e:#}"),
        }
    }
    None
}
