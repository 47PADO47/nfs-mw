//! The parts of opening a world that belong to the mods' worlds: which map packs the world has, and the clock and weather its manifest
//! asks for (`docs/specs/mod-content-replacement-worlds.md`).

use std::collections::HashSet;

use game_install::GameDir;

use super::*;
use crate::modding::content::scenery::MapPack;
use crate::modding::content::worlds::World;

/// The map packs of the world being opened. A standalone world has its own (and its patches'); the city has every mod's `scenery/`
/// packs, an alternative map adds its own, and the objects the replacement sets in effect place go in the city and its alternative
/// maps, nowhere else. `installed` holds the section numbers of the install's index.
pub(super) fn map_packs(
    dir: &GameDir,
    standalone: Option<&World>,
    alternative: Option<&World>,
    installed: &HashSet<i16>,
) -> Vec<MapPack> {
    if let Some(world) = standalone {
        return crate::modding::content::worlds::packs(dir, world, installed);
    }
    let taken_by = |packs: &[MapPack]| -> HashSet<i16> {
        installed.iter().copied().chain(packs.iter().map(|pack| pack.section.number)).collect()
    };
    let mut packs = crate::modding::content::scenery::packs(dir, crate::modding::content::mod_dirs(), installed);
    if let Some(map) = alternative {
        let more = crate::modding::content::worlds::packs(dir, map, &taken_by(&packs));
        packs.extend(more);
    }
    let added = crate::modding::content::replace::added_packs(dir, &taken_by(&packs));
    packs.extend(added);
    packs
}

impl WorldScene {
    /// Set the clock and the weather a world's manifest asks for (`environment`): the hour (held when it says so) and the rain. What
    /// the manifest leaves out stays the game's.
    pub(super) fn apply_world_environment(&mut self, environment: blackbox_modding::content::WorldEnvironment) {
        if let Some(hour) = environment.hour {
            let now = self.environment_state();
            self.configure_environment(crate::viewer::Environment { hour, ..now });
            self.day.set_held(environment.freeze);
        }
        if let Some(rain) = environment.rain {
            self.rain.configure(rain > 0.0, rain);
        }
    }
}
