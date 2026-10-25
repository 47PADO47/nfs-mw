//! The map scenarios: free roam with a GPS route to the safe house, two turns long, an auto shop and a discovered race
//! start. `map-compact`, `map-centered` and `map-expanded` share the state; only the map mode they run with differs
//! (`HudScenario::map_mode`).

use nfsmw_data::gameplay::RaceType;

use super::common::{CAR, at, car, career};
use crate::hud::{GPS, HudState, MapBlip, NavMarker, NavMarkerKind, NavRoute, next_turn};

/// Cruising at 96 km/h in 3rd on the way to the safe house: 300 m on, left, 220 m on, right, and 400 m to the end.
pub fn route(viewport: [f32; 2]) -> HudState {
    // The route's points, and the junctions where one road meets the next (the turns).
    let path = vec![CAR, at(300.0, 0.0), at(300.0, 220.0), at(700.0, 220.0)];
    let junctions = [1, 2];
    let target = at(700.0, 220.0);
    let route = NavRoute { next_turn: next_turn(&path, &junctions, CAR), path, target };
    let safe_house = NavMarker { kind: NavMarkerKind::SafeHouse, position: target, name: None, discovered: true };
    let shop = NavMarker { kind: NavMarkerKind::AutoShop, position: at(150.0, -260.0), name: None, discovered: true };
    let race = NavMarker {
        kind: NavMarkerKind::Race(RaceType::Sprint),
        position: at(-180.0, 140.0),
        name: Some("fixture".to_owned()),
        discovered: true,
    };
    HudState {
        route: Some(route),
        markers: vec![safe_house, shop, race],
        blips: vec![MapBlip { style: GPS, position: target, active: false, heading: None }],
        ..career(car(96.0, 3, 4300.0, viewport))
    }
}
