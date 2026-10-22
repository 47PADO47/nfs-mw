//! The HUD of the world scene: the drive's state, with the traffic's and the map's blips on the minimap.

use glam::Vec3;
use nfsmw_data::gameplay::{ActivityKind, ActivityMarker, GameplayData, Placement, Race, RaceType};

use super::hud_pursuit::pursuit_board;
use super::players::LEAD;
use super::{View, WorldScene, space};
use crate::hud::{
    AUTO_SHOP, CAR_LOT, MapBlip, RACE_CIRCUIT, RACE_DRAG, RACE_KNOCKOUT, RACE_SPEEDTRAP, RACE_SPRINT, RACE_TOLLBOOTH,
    SAFE_HOUSE,
};

impl WorldScene {
    /// Whether the HUD is drawn this frame: the rule of `hud_state` and `hud_frame`, without building the frame.
    pub(super) fn hud_shown(&self) -> bool {
        if self.moment.hides_hud() || self.nis.is_playing() {
            return false;
        }
        if self.view == View::Fly {
            return false;
        }
        let Some(drive) = self.players.lead() else { return false };
        if drive.camera_looking_back(self.camera_mode) && !self.tuning.show_hud_on_look_back {
            return false;
        }
        drive.has_sim()
    }

    /// The HUD for this frame. Hidden in the fly view and while there is no car yet.
    pub(super) fn hud_frame(&self) -> Option<crate::hud::HudState> {
        let hidden = Some(crate::hud::HudState { visible: false, ..Default::default() });
        if self.view == View::Fly {
            return hidden;
        }
        // No car yet (the world is still loading): hidden, not the idle HUD a viewer shows.
        let mut hud = self.players.lead().and_then(|drive| drive.hud_state()).or(hidden);
        if let Some(state) = hud.as_mut() {
            state.skin = self.garage.applied.gauges.skin;
            state.gauge_colors = self.garage.applied.gauges.colors();
            if let Some(blip) = self.route_blip() {
                state.blips.push(blip);
            }
            // The co-op guest's car, for the world map's second mark.
            state.second_player = self.players.first_guest().map(|drive| {
                let pose = drive.pose();
                crate::scenes::world::split::map_position(pose.position, pose.rotation * Vec3::X)
            });
            if let Some(traffic) = self.traffic.as_ref() {
                traffic.map_blips(&mut state.blips);
                self.breaker_icons.blips(traffic.is_pursuing(), &mut state.blips);
                let mut meter = 0.0;
                if let Some(drive) = self.players.lead() {
                    let player = drive.player_view();
                    state.radar = traffic.radar(player.position, player.forward);
                    meter = traffic.pursuit_meter(player.position, player.speed);
                }
                state.cop_nearby = state.radar.is_some();
                state.vehicle_heat = traffic.vehicle_heat();
                state.cops_engaged = traffic.cops_engaged();
                state.cops_damaged = traffic.cops_damaged();
                state.pursuit = pursuit_board(traffic, meter);
                if traffic.arrest_active() {
                    state.radar = None;
                }
                state.busted = self.busted_meter_for(LEAD, traffic);
                state.busted_overlay = traffic.busted_overlay();
            }
            self.career.hud.apply(state);
            state.race = self.race_hud();
            state.leaders = self.race_leaders();
            state.message = self.race_message().or_else(|| self.lead_catch_up_line());
            state.drag = self.race.as_ref().is_some_and(|r| r.kind == RaceType::Drag);
            state.map_source = self.race.as_ref().map(|r| r.map.clone()).unwrap_or_default();
            self.race_blips(&mut state.blips);
            self.race_start_blips(&mut state.blips);
            // A race shows its own route only: no garages, car lots, safe houses or free-roam speed traps.
            if self.race.is_none() {
                self.speed_trap_blips(&mut state.blips);
                state.blips.extend_from_slice(&self.map_blips);
            }
            state.blips.extend_from_slice(&self.mod_blips);
        }
        hud
    }

    /// The end-of-pursuit card, drawn over the frame while it is up. Not in the fly view.
    pub(super) fn show_pursuit_end(&self, ctx: &egui::Context) {
        if self.view == View::Fly {
            return;
        }
        let Some(card) = self.traffic.as_ref().and_then(|traffic| traffic.pursuit_end_card()) else { return };
        crate::hud::show_pursuit_end(ctx, &card);
    }
}

/// The map blips of the interiors (auto shops, car lots, safe houses), in the minimap's render space. They do not
/// move, so they are made once, when the world opens.
pub(super) fn interior_blips(data: &GameplayData) -> Vec<MapBlip> {
    data.markers.iter().map(interior_blip).collect()
}

/// The race start icons with the name of the race each marks, made once when the world opens. Which of them show is
/// decided each frame (`WorldScene::race_start_blips`): none during a race, and in a career only the open ones.
pub(super) fn race_start_blips(data: &GameplayData) -> Vec<(String, MapBlip)> {
    data.races.iter().filter_map(|race| Some((race.name.clone(), race_blip(race)?))).collect()
}

impl WorldScene {
    /// Appends the icons of the races that can be started now: none while a race runs, and in a career only the races the
    /// Blacklist has opened or does not list (`docs/specs/world-markers.md` section 2).
    pub(super) fn race_start_blips(&self, out: &mut Vec<MapBlip>) {
        if self.race.is_some() {
            return;
        }
        let shown = self.race_starts.iter().filter(|(name, _)| !self.career_race_locked(name)).map(|(_, blip)| *blip);
        out.extend(shown);
    }
}

/// A race's icon is its type's, at its start, pointing along the grid. A race with no icon in the game's table has none.
fn race_blip(race: &Race) -> Option<MapBlip> {
    let style = race_style(race.race_type?)?;
    let start: Placement = race.start?;
    Some(map_blip(style, start.position))
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

/// An auto shop, car lot or safe house: a fixed blip, so its icon turns with the player's arrow.
fn interior_blip(marker: &ActivityMarker) -> MapBlip {
    let style = match marker.kind {
        ActivityKind::AutoShop => AUTO_SHOP,
        ActivityKind::CarLot => CAR_LOT,
        ActivityKind::SafeHouse => SAFE_HOUSE,
    };
    map_blip(style, marker.position)
}

/// A stored position (physics space, see `gameplay`) as a map blip in render space.
fn map_blip(style: &'static str, position: Vec3) -> MapBlip {
    let at = space::to_render(position.to_array());
    MapBlip { style, position: [at.x, at.y], active: false, heading: None }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn race(race_type: Option<RaceType>, start: Option<Placement>) -> Race {
        Race {
            key: 1,
            name: "race".to_string(),
            event: None,
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

    fn data(races: Vec<Race>, markers: Vec<ActivityMarker>) -> GameplayData {
        GameplayData { races, markers, ..Default::default() }
    }

    #[test]
    fn a_race_start_is_a_blip_of_its_race_type_in_render_space() {
        // Physics (-4404.865, 209.6, 1511.8) is render (1511.8, 4404.9, 209.6): the map's own coordinates.
        let start = Placement { position: Vec3::new(-4404.865, 209.6045, 1511.824), heading_deg: 0.0 };
        let blips = race_start_blips(&data(vec![race(Some(RaceType::Sprint), Some(start))], Vec::new()));
        assert_eq!(blips.len(), 1);
        assert_eq!(blips[0].0, "race");
        assert_eq!(blips[0].1.style, RACE_SPRINT);
        assert_eq!(blips[0].1.position, [1511.824, 4404.865]);
        assert!(!blips[0].1.active);
    }

    #[test]
    fn a_race_without_a_known_type_or_a_start_has_no_blip() {
        let start = Placement { position: Vec3::ZERO, heading_deg: 0.0 };
        let no_type = race(None, Some(start));
        let no_start = race(Some(RaceType::Drag), None);
        let challenge = race(Some(RaceType::Challenge), Some(start));
        assert!(race_start_blips(&data(vec![no_type, no_start, challenge], Vec::new())).is_empty());
    }

    #[test]
    #[ignore = "needs the game (set NFSMW_GAME_DIR)"]
    fn the_install_has_race_and_interior_blips() {
        let root = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR");
        let dir = game_install::GameDir::open(std::path::PathBuf::from(root)).unwrap();
        let data = nfsmw_data::gameplay::load(&dir).unwrap();
        let blips = interior_blips(&data);
        let races = race_start_blips(&data).len();
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
            interior_blips(&data(Vec::new(), vec![marker(ActivityKind::AutoShop), marker(ActivityKind::SafeHouse)]));
        let styles: Vec<&str> = blips.iter().map(|b| b.style).collect();
        assert_eq!(styles, [AUTO_SHOP, SAFE_HOUSE]);
        // Physics (0, 0, 10) is render (10, 0, 0).
        assert_eq!(blips[0].position, [10.0, 0.0]);
    }
}
