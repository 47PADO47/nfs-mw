//! The HUD of the world scene: the drive's state, with the traffic's and the map's blips on the minimap.

use glam::Vec3;
use nfsmw_data::gameplay::{self, ActivityKind, ActivityMarker, GameplayData, Placement, Race, RaceType};

use super::{View, WorldScene, space};
use crate::audio::PursuitFeed;
use crate::hud::{
    AUTO_SHOP, CAR_LOT, MapBlip, PursuitHud, RACE_CIRCUIT, RACE_DRAG, RACE_KNOCKOUT, RACE_SPEEDTRAP, RACE_SPRINT,
    RACE_TOLLBOOTH, SAFE_HOUSE,
};

/// The chase as the pursuit board shows it. The evade share is the part of the evade timeout already done (the feed
/// gives what is left of it).
fn pursuit_hud(feed: PursuitFeed) -> PursuitHud {
    PursuitHud::from_evade_time(feed.secs, feed.secs_to_evade, blackbox_pursuit::evade::timeout(feed.heat))
}

impl WorldScene {
    /// The HUD for this frame. Hidden in the fly view and while there is no car yet.
    pub(super) fn hud_frame(&self) -> Option<crate::hud::HudState> {
        let hidden = Some(crate::hud::HudState { visible: false, ..Default::default() });
        if self.view == View::Fly {
            return hidden;
        }
        // No car yet (the world is still loading): hidden, not the idle HUD a viewer shows.
        let mut hud = self.drive.as_ref().and_then(|drive| drive.hud_state()).or(hidden);
        if let Some(state) = hud.as_mut() {
            if let Some(traffic) = self.traffic.as_ref() {
                traffic.map_blips(&mut state.blips);
                // The radar's green laser shows while cops chase the player.
                state.cop_nearby = traffic.is_pursuing();
                state.pursuit = traffic.pursuit_feed().map(pursuit_hud);
            }
            state.blips.extend_from_slice(&self.map_blips);
        }
        hud
    }
}

/// The map blips of the race starts and the interiors (auto shops, car lots, safe houses), in the minimap's render
/// space. They do not move, so they are made once, when the world opens. A race is shown at its start, not active: race mode is
/// not run yet.
pub(super) fn static_blips(data: &GameplayData) -> Vec<MapBlip> {
    let races = data.races.iter().filter_map(race_blip);
    let interiors = data.markers.iter().map(interior_blip);
    races.chain(interiors).collect()
}

/// A race's icon is its type's, at its start, pointing along the grid. A race with no icon in the game's table has none.
fn race_blip(race: &Race) -> Option<MapBlip> {
    let style = race_style(race.race_type?)?;
    let start: Placement = race.start?;
    let forward = space::to_render(gameplay::world_heading(start.heading_deg).to_array());
    Some(map_blip(style, start.position, [forward.x, forward.y]))
}

fn race_style(kind: RaceType) -> Option<&'static str> {
    match kind {
        RaceType::Sprint => Some(RACE_SPRINT),
        RaceType::Drag => Some(RACE_DRAG),
        RaceType::Circuit => Some(RACE_CIRCUIT),
        RaceType::Knockout => Some(RACE_KNOCKOUT),
        RaceType::Tollbooth => Some(RACE_TOLLBOOTH),
        RaceType::SpeedTrap => Some(RACE_SPEEDTRAP),
        _ => None,
    }
}

/// An auto shop, car lot or safe house. Its icon does not turn, so the heading points along the minimap's x axis.
fn interior_blip(marker: &ActivityMarker) -> MapBlip {
    let style = match marker.kind {
        ActivityKind::AutoShop => AUTO_SHOP,
        ActivityKind::CarLot => CAR_LOT,
        ActivityKind::SafeHouse => SAFE_HOUSE,
    };
    map_blip(style, marker.position, [1.0, 0.0])
}

/// A stored position (physics space, see `gameplay`) as a map blip in render space.
fn map_blip(style: &'static str, position: Vec3, heading: [f32; 2]) -> MapBlip {
    let at = space::to_render(position.to_array());
    MapBlip { style, position: [at.x, at.y], heading, active: false }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn race(race_type: Option<RaceType>, start: Option<Placement>) -> Race {
        Race {
            key: 1,
            name: "race".to_string(),
            event_icon: String::new(),
            race_type,
            region: None,
            laps: 0,
            start,
            finish: None,
            checkpoints: Vec::new(),
        }
    }

    fn data(races: Vec<Race>, markers: Vec<ActivityMarker>) -> GameplayData {
        GameplayData { races, triggers: Vec::new(), markers }
    }

    #[test]
    fn a_race_start_is_a_blip_of_its_race_type_in_render_space() {
        // Physics (-4404.865, 209.6, 1511.8) is render (1511.8, 4404.9, 209.6): the map's own coordinates.
        let start = Placement { position: Vec3::new(-4404.865, 209.6045, 1511.824), heading_deg: 0.0 };
        let blips = static_blips(&data(vec![race(Some(RaceType::Sprint), Some(start))], Vec::new()));
        assert_eq!(blips.len(), 1);
        assert_eq!(blips[0].style, RACE_SPRINT);
        assert_eq!(blips[0].position, [1511.824, 4404.865]);
        assert!((blips[0].heading[0] - 1.0).abs() < 1e-5 && blips[0].heading[1].abs() < 1e-5);
        assert!(!blips[0].active);
    }

    #[test]
    fn a_race_without_a_known_type_or_a_start_has_no_blip() {
        let start = Placement { position: Vec3::ZERO, heading_deg: 0.0 };
        let no_type = race(None, Some(start));
        let no_start = race(Some(RaceType::Drag), None);
        let challenge = race(Some(RaceType::Challenge), Some(start));
        assert!(static_blips(&data(vec![no_type, no_start, challenge], Vec::new())).is_empty());
    }

    #[test]
    #[ignore = "needs the game (set NFSMW_GAME_DIR)"]
    fn the_install_has_race_and_interior_blips() {
        let root = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR");
        let dir = game_install::GameDir::open(std::path::PathBuf::from(root)).unwrap();
        let data = nfsmw_data::gameplay::load(&dir).unwrap();
        let blips = static_blips(&data);
        let races = blips.iter().filter(|b| b.style.starts_with("race_")).count();
        let shops = blips.iter().filter(|b| b.style == AUTO_SHOP).count();
        let houses = blips.iter().filter(|b| b.style == SAFE_HOUSE).count();
        eprintln!("blips: {races} races, {shops} auto shops, {houses} safe houses");
        for blip in blips.iter().filter(|b| b.style == AUTO_SHOP || b.style == SAFE_HOUSE) {
            eprintln!("{} at {:?}", blip.style, blip.position);
        }
        assert!(shops == 8 && houses == 4, "8 auto shops and 4 safe houses in the install");
    }

    #[test]
    fn the_interiors_are_blips_of_their_own_styles() {
        let marker = |kind| ActivityMarker {
            key: 2,
            name: "zone".to_string(),
            kind,
            position: Vec3::new(0.0, 0.0, 10.0),
            radius: 6.0,
            respawn: None,
            icon_index: None,
        };
        let blips =
            static_blips(&data(Vec::new(), vec![marker(ActivityKind::AutoShop), marker(ActivityKind::SafeHouse)]));
        let styles: Vec<&str> = blips.iter().map(|b| b.style).collect();
        assert_eq!(styles, [AUTO_SHOP, SAFE_HOUSE]);
        // Physics (0, 0, 10) is render (10, 0, 0).
        assert_eq!(blips[0].position, [10.0, 0.0]);
    }
}
