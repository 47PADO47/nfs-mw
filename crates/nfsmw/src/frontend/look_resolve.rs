//! Finding a profile's car among the install's car types and preset rides (`CarLook::resolve`).

use blackbox_attrib::vlt_hash;
use blackbox_hash::bstring_hash;
use game_install::GameDir;
use nfsmw_data::car::{CarTables, list, pick_folder};

use super::{CarLook, DEFAULT_CAR};

/// The `CARS/` folder and preset of `look`. A look with no record keeps its own names.
pub(super) fn resolve(look: &CarLook, dir: &GameDir) -> (String, Option<String>) {
    let Some(record) = &look.record else { return (look.car.clone(), look.preset.clone()) };
    let folders = list(dir);
    let by_name = || match pick_folder(&folders, &look.car) {
        Some(folder) if !look.car.is_empty() => (folder.to_owned(), None),
        _ => (DEFAULT_CAR.to_owned(), None),
    };
    let Ok(tables) = CarTables::load(dir) else { return by_name() };
    // A preset ride's handle is the hash of its name (`career-flow.md` section 3); the preset names its car type.
    let preset = tables.presets.iter().find(|preset| bstring_hash(&preset.preset_name) == record.handle);
    if let Some(preset) = preset
        && let Some(folder) = pick_folder(&folders, &preset.car_type_name)
    {
        return (folder.to_owned(), Some(preset.preset_name.clone()));
    }
    // A stock car's vehicle key is the hash of its car type's lower-case name; some front-end keys are too.
    for key in [record.vehicle_key, record.frontend_key] {
        let car_type =
            tables.types.iter().find(|t| t.usage_type == 0 && vlt_hash(&t.type_name.to_ascii_lowercase()) == key);
        if let Some(car_type) = car_type
            && let Some(folder) = pick_folder(&folders, &car_type.type_name)
        {
            return (folder.to_owned(), None);
        }
    }
    by_name()
}
