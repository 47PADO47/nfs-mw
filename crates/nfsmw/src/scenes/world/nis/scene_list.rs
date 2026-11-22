//! The vehicle a scene's special car list gives a police channel (`nfsmw_data::world::scene_cars`): a fixed list of
//! eight per list code, `cop1` first, the code from the install's scene candidate table. Spec: `nis-player.md` section 7.

use nfsmw_data::world::moment::scene_hash;
use nfsmw_data::world::scene_cars;

use super::cars::CAR_SLOTS;
use crate::scenes::world::WorldScene;

impl WorldScene {
    /// The list vehicle of car slot `slot` of the playing scene (slots 8 to 15 are `cop1`..`cop8`): `None` when the
    /// slot is not a police channel or the scene has no list, `Some(None)` when its list makes no car there.
    pub(super) fn nis_listed_vehicle(&self, slot: usize) -> Option<Option<&'static str>> {
        let name = self.nis.playing_name()?;
        listed_vehicle(&self.dir, name, slot)
    }
}

/// [`WorldScene::nis_listed_vehicle`] for the scene `scene` of the install `dir`.
pub(super) fn listed_vehicle(dir: &game_install::GameDir, scene: &str, slot: usize) -> Option<Option<&'static str>> {
    let index = slot.checked_sub(CAR_SLOTS / 2).filter(|&i| i < CAR_SLOTS / 2)?;
    let data = nfsmw_data::world::moment::load(dir).ok()?;
    Some(scene_cars::list(data.car_list_of(scene_hash(scene))?)?[index])
}
