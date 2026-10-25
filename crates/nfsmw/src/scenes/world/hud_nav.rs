//! The places on the map for the Custom HUD (`hud/state/nav.rs`): the race starts, safe houses, auto shops and car lots,
//! by the same rules as their map blips (`hud.rs`): none during a race, the first safe house alone in the prologue, and
//! a race the Blacklist has not opened marked as not discovered.

use nfsmw_data::gameplay::{ActivityKind, ActivityMarker, Race};

use super::hud::FIRST_SAFE_HOUSE;
use super::{WorldScene, space};
use crate::hud::{NavMarker, NavMarkerKind};

impl WorldScene {
    /// The places the map shows this frame, with their names and whether the player can enter them.
    pub(super) fn nav_markers(&self) -> Vec<NavMarker> {
        let Some(data) = self.gameplay.as_ref() else { return Vec::new() };
        if self.race.is_some() {
            return Vec::new();
        }
        if self.in_prologue() {
            return data.markers.iter().filter(|marker| marker.name == FIRST_SAFE_HOUSE).map(place_marker).collect();
        }
        self.refresh_race_locks();
        let races = data.races.iter().filter_map(|race| race_marker(race, !self.career_race_locked(&race.name)));
        races.chain(data.markers.iter().map(place_marker)).collect()
    }
}

/// A race start's marker: its type at its start, named by its career event id. A race without a start or a type has
/// none, as it has no blip.
fn race_marker(race: &Race, open: bool) -> Option<NavMarker> {
    let start = race.start?;
    let at = space::to_render(start.position.to_array());
    Some(NavMarker {
        kind: NavMarkerKind::Race(race.race_type?),
        position: [at.x, at.y],
        name: Some(crate::career::race_event_id(race)),
        discovered: open,
    })
}

/// An auto shop, car lot or safe house: always on the map.
fn place_marker(marker: &ActivityMarker) -> NavMarker {
    let kind = match marker.kind {
        ActivityKind::AutoShop => NavMarkerKind::AutoShop,
        ActivityKind::CarLot => NavMarkerKind::CarLot,
        ActivityKind::SafeHouse => NavMarkerKind::SafeHouse,
    };
    let at = space::to_render(marker.position.to_array());
    NavMarker { kind, position: [at.x, at.y], name: None, discovered: true }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;
    use nfsmw_data::gameplay::{Placement, RaceType};

    fn race(race_type: Option<RaceType>, start: Option<Placement>) -> Race {
        Race {
            key: 1,
            name: "race_bin_10/10_2_1_sprint".to_string(),
            event: Some("10.2.1".to_string()),
            event_icon: String::new(),
            race_type,
            region: None,
            laps: 0,
            start,
            finish: None,
            checkpoints: Vec::new(),
            barriers: Vec::new(),
            params: Default::default(),
            story: Default::default(),
        }
    }

    #[test]
    fn a_race_marker_has_its_type_event_id_and_render_space_place() {
        // Physics (-4404.865, 209.6, 1511.8) is render (1511.8, 4404.9, 209.6).
        let start = Placement { position: Vec3::new(-4404.865, 209.6, 1511.8), heading_deg: 0.0 };
        let marker = race_marker(&race(Some(RaceType::Sprint), Some(start)), false).unwrap();
        assert_eq!(marker.kind, NavMarkerKind::Race(RaceType::Sprint));
        assert_eq!(marker.position, [1511.8, 4404.865]);
        assert_eq!(marker.name.as_deref(), Some("10.2.1"));
        assert!(!marker.discovered, "a locked race is not discovered");
    }

    #[test]
    fn a_race_without_a_type_or_a_start_has_no_marker() {
        let start = Placement { position: Vec3::ZERO, heading_deg: 0.0 };
        assert_eq!(race_marker(&race(None, Some(start)), true), None);
        assert_eq!(race_marker(&race(Some(RaceType::Drag), None), true), None);
    }

    #[test]
    fn a_place_is_named_by_its_kind_and_always_discovered() {
        let marker = ActivityMarker {
            key: 2,
            name: "zone".to_string(),
            kind: ActivityKind::SafeHouse,
            position: Vec3::new(0.0, 0.0, 10.0),
            radius: 6.0,
            respawn: None,
            icon_index: None,
        };
        let place = place_marker(&marker);
        assert_eq!((place.kind, place.position, place.discovered), (NavMarkerKind::SafeHouse, [10.0, 0.0], true));
        assert_eq!(place.name, None);
    }
}
