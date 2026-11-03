use glam::Vec3;
use nfsmw_data::gameplay::{ActivityKind, ActivityMarker, GameplayData, Placement, Race, RaceType};

use super::entry_zones::*;

/// A gameplay position given in render space (the map's own x, y, z), stored as the gameplay data stores it: physics.
fn stored(render: [f32; 3]) -> Vec3 {
    Vec3::new(-render[1], render[2], render[0])
}

fn marker(kind: ActivityKind, at: Vec3, radius: f32) -> ActivityMarker {
    ActivityMarker { key: 2, name: "zone".to_string(), kind, position: at, radius, respawn: None, icon_index: None }
}

fn race(name: &str, start: Option<Placement>) -> Race {
    Race {
        key: 1,
        name: name.to_string(),
        event: None,
        event_icon: String::new(),
        race_type: Some(RaceType::Sprint),
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
fn a_sphere_holds_the_points_within_its_radius() {
    let c = Vec3::new(1.0, 2.0, 3.0);
    assert!(sphere_contains(c, 6.0, c));
    assert!(sphere_contains(c, 6.0, c + Vec3::new(6.0, 0.0, 0.0)), "the surface counts as inside");
    assert!(!sphere_contains(c, 6.0, c + Vec3::new(0.0, 6.1, 0.0)));
    assert!(!sphere_contains(c, 0.0, c), "a zone with no radius holds nothing");
}

#[test]
fn the_zone_centres_are_in_render_space_like_the_car() {
    // A stored position (physics) of (0, 0, 4) is the map point (4, 0, 0): the render x axis.
    let zones =
        EntryZones::from_data(&data(Vec::new(), vec![marker(ActivityKind::SafeHouse, stored([4.0, 0.0, 0.0]), 6.0)]));
    assert_eq!(zones.zones[0].centre, Vec3::new(4.0, 0.0, 0.0));
    assert_eq!(zones.entered(Vec3::new(5.0, 0.0, 0.0)).map(|z| z.kind), Some(ZoneKind::SafeHouse));
}

#[test]
fn the_car_in_a_zone_is_the_one_whose_centre_is_nearest() {
    let zones = EntryZones::from_data(&data(
        Vec::new(),
        vec![
            marker(ActivityKind::AutoShop, stored([0.0, 0.0, 0.0]), 10.0),
            marker(ActivityKind::SafeHouse, stored([4.0, 0.0, 0.0]), 6.0),
        ],
    ));
    let inside_both = Vec3::new(3.0, 0.0, 0.0);
    assert_eq!(zones.entered(inside_both).map(|z| z.kind), Some(ZoneKind::SafeHouse));
    assert_eq!(zones.entered(Vec3::new(-8.0, 0.0, 0.0)).map(|z| z.kind), Some(ZoneKind::AutoShop));
    assert!(zones.entered(Vec3::new(40.0, 0.0, 0.0)).is_none());
}

#[test]
fn a_car_lot_marker_is_an_entry_zone_of_its_own_kind() {
    let zones =
        EntryZones::from_data(&data(Vec::new(), vec![marker(ActivityKind::CarLot, stored([20.0, 0.0, 0.0]), 7.0)]));
    assert_eq!(zones.zones.len(), 1);
    assert_eq!(zones.zones[0].kind, ZoneKind::CarLot);
    assert_eq!(zones.zones[0].radius, 7.0);
    assert_eq!(zones.zones[0].kind.label(), "car lot");
    assert_eq!(zones.entered(Vec3::new(21.0, 0.0, 0.0)).map(|z| z.kind), Some(ZoneKind::CarLot));
}

#[test]
fn the_zones_are_the_interiors_then_the_race_starts() {
    let start = Placement { position: stored([100.0, 0.0, 0.0]), heading_deg: 0.0 };
    let zones = EntryZones::from_data(&data(
        vec![race("race_a", Some(start)), race("no_start", None)],
        vec![marker(ActivityKind::SafeHouse, Vec3::ZERO, 7.0)],
    ));
    assert_eq!(zones.zones.len(), 2, "a race with no start is not a zone");
    assert_eq!(zones.zones[0].kind, ZoneKind::SafeHouse);
    assert_eq!(zones.zones[0].radius, 7.0);
    assert_eq!(zones.zones[1].kind, ZoneKind::Race);
    assert_eq!(zones.zones[1].radius, RACE_RADIUS);
    assert_eq!(zones.zones[1].name, "race_a");
    assert_eq!(zones.entered(Vec3::new(102.0, 0.0, 0.0)).map(|z| z.kind), Some(ZoneKind::Race));
}

#[test]
fn only_the_zones_near_the_car_are_drawn() {
    let zones = EntryZones::from_data(&data(
        Vec::new(),
        vec![
            marker(ActivityKind::AutoShop, stored([10.0, 0.0, 0.0]), 6.0),
            marker(ActivityKind::AutoShop, stored([1000.0, 0.0, 0.0]), 6.0),
        ],
    ));
    assert_eq!(zones.near(Vec3::ZERO, 250.0).count(), 1);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_install_has_eight_shops_four_safe_houses_and_authored_race_entries() {
    let root = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR");
    let dir = game_install::GameDir::open(std::path::PathBuf::from(root)).unwrap();
    let data = nfsmw_data::gameplay::load(&dir).unwrap();
    let zones = EntryZones::from_data(&data);
    let count = |kind| zones.zones.iter().filter(|z| z.kind == kind).count();
    assert_eq!(count(ZoneKind::AutoShop), 8);
    assert_eq!(count(ZoneKind::SafeHouse), 4);
    assert!(!data.engages.is_empty(), "the install must expose its engage triggers");
    assert_eq!(
        count(ZoneKind::Race),
        data.races.iter().filter(|r| data.engages.iter().any(|e| e.target == r.key)).count()
    );
    for zone in zones.zones.iter().filter(|zone| zone.kind == ZoneKind::Race) {
        let race = data.race_named(&zone.name).unwrap();
        let engage = data.engages.iter().find(|entry| entry.target == race.key).unwrap();
        assert_eq!(zone.centre, super::space::to_render(engage.position.to_array()));
    }
}

#[test]
fn engage_position_replaces_grid_and_omits_races_without_an_entry() {
    let start = Placement { position: stored([100.0, 10.0, 20.0]), heading_deg: 90.0 };
    let mut missing = race("missing", Some(start));
    missing.key = 2;
    let mut data = data(vec![race("entered", Some(start)), missing], Vec::new());
    data.engages.push(nfsmw_data::gameplay::EngageMarker {
        target: 1,
        position: stored([15.0, 20.0, 30.0]),
        radius: 8.0,
    });
    let zones = EntryZones::from_data(&data);
    assert_eq!(zones.zones.len(), 1);
    assert_eq!(zones.zones[0].centre, Vec3::new(15.0, 20.0, 30.0));
    assert_eq!(zones.zones[0].radius, 8.0);
    assert!(zones.entered(Vec3::new(15.0, 20.0, 30.0)).is_some());
    assert!(zones.entered(Vec3::new(100.0, 10.0, 20.0)).is_none());
}
