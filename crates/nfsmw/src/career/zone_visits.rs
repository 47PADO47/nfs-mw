//! The zones the career remembers across sessions (`docs/specs/career-safehouse-2.md` section 9): the respawn marker of
//! the last visited zone of any kind and of the last visited safe house, kept in the gameplay blob of the save, as the
//! original keeps them. A loaded career starts the world at its last safe house from them, not at `career_start`.

use nfsmw_career::save::milestones_block::{MilestoneBlock, RespawnKeys};
use nfsmw_data::gameplay::{ActivityKind, ActivityMarker};

use super::Career;

impl Career {
    /// The safe house zone (its full name, as the world's zones are named) whose respawn marker the save holds as the
    /// last visited safe house. `None` for a save without the pair, or with a key that no safe house of the loaded data
    /// has (the original then falls back to `career_start` too).
    pub fn saved_safe_house_zone(&self) -> Option<String> {
        let keys = MilestoneBlock::from_bytes(&self.settings.gameplay_blob[..]).ok()?.respawn_keys()?;
        zone_with_key(&self.data.gameplay.markers, keys.safe_house, true).map(|marker| marker.name.clone())
    }

    /// The player came to zone `zone`: its respawn marker becomes the last visited zone and, for a safe house, the last
    /// visited safe house (`DoZoneMenuAction`). A zone the data does not know, or a blob without the pair, is left as it
    /// is.
    pub fn note_zone_visit(&mut self, zone: &str) {
        let Some(marker) = self.data.gameplay.markers.iter().find(|marker| marker.name == zone) else { return };
        let Some(key) = marker.respawn_key else { return };
        let safe_house = marker.kind == ActivityKind::SafeHouse;
        let Ok(mut block) = MilestoneBlock::from_bytes(&self.settings.gameplay_blob[..]) else { return };
        let Some(keys) = block.respawn_keys() else { return };
        let keys = visited(keys, key, safe_house);
        match block.set_respawn_keys(keys).and_then(|_| block.write_into(&mut self.settings.gameplay_blob[..])) {
            Ok(()) => {
                log::info!("career: {zone} is the last visited {}", if safe_house { "safe house" } else { "zone" })
            }
            Err(error) => log::warn!("career: the visit of {zone} is not saved: {error}"),
        }
    }
}

/// The pair after a visit of a zone whose respawn marker is `key`.
fn visited(keys: RespawnKeys, key: u32, safe_house: bool) -> RespawnKeys {
    RespawnKeys { home: key, safe_house: if safe_house { key } else { keys.safe_house } }
}

/// The zone whose respawn marker is `key`; with `safe_house`, safe houses only.
fn zone_with_key(markers: &[ActivityMarker], key: u32, safe_house: bool) -> Option<&ActivityMarker> {
    if key == 0 {
        return None;
    }
    markers
        .iter()
        .filter(|marker| !safe_house || marker.kind == ActivityKind::SafeHouse)
        .find(|marker| marker.respawn_key == Some(key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    fn marker(name: &str, kind: ActivityKind, key: u32) -> ActivityMarker {
        ActivityMarker {
            key: 1,
            name: name.to_owned(),
            kind,
            position: Vec3::ZERO,
            radius: 5.0,
            respawn: None,
            respawn_key: Some(key),
            icon_index: None,
        }
    }

    #[test]
    fn a_car_lot_visit_moves_the_last_zone_but_keeps_the_last_safe_house() {
        let keys = RespawnKeys { home: 7, safe_house: 7 };
        assert_eq!(visited(keys, 9, false), RespawnKeys { home: 9, safe_house: 7 });
        assert_eq!(visited(keys, 9, true), RespawnKeys { home: 9, safe_house: 9 });
    }

    #[test]
    fn the_saved_safe_house_is_found_by_its_respawn_key_among_safe_houses_only() {
        let markers = [
            marker("menu_gate_carlot/zone_carlot_1", ActivityKind::CarLot, 5),
            marker("menu_gate_safehouse/zone_safehouse_2", ActivityKind::SafeHouse, 6),
        ];
        assert_eq!(
            zone_with_key(&markers, 6, true).map(|m| m.name.as_str()),
            Some("menu_gate_safehouse/zone_safehouse_2")
        );
        assert_eq!(zone_with_key(&markers, 5, true), None, "a car lot is never the saved safe house");
        assert_eq!(zone_with_key(&markers, 0, true), None, "no key saved");
    }
}
