//! The traffic patterns read from the install match the tables in `docs/specs/ai-traffic-spawning.md` §4.

use blackbox_attrib::Database;
use blackbox_roads::{TrackZones, from_zone_space, to_zone_space};
use glam::Vec2;
use nfsmw_data::traffic::{TrafficPattern, pattern_hash, patterns};
use nfsmw_data::world::{DEFAULT_TRACK, WorldIndex};

use crate::install;

fn all() -> Option<Vec<TrafficPattern>> {
    let dir = install()?;
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).expect("attributes.bin");
    Some(patterns(&db))
}

fn cars(patterns: &[TrafficPattern], name: &str) -> Vec<(String, f32, u32, u32)> {
    let pattern = patterns.iter().find(|p| p.name == name).unwrap_or_else(|| panic!("no pattern {name}"));
    pattern.vehicles.iter().map(|v| (v.name.clone(), v.rate, v.max_instances, v.percent)).collect()
}

fn expect(rows: &[(&str, f32, u32, u32)]) -> Vec<(String, f32, u32, u32)> {
    rows.iter().map(|&(n, r, m, p)| (n.to_owned(), r, m, p)).collect()
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_ten_patterns_share_speeds_and_spawn_time() {
    let Some(all) = all() else { return };
    assert_eq!(all.len(), 10);
    for p in &all {
        assert_eq!((p.speed_street_mph, p.speed_highway_mph, p.spawn_time), (35.0, 55.0, 4.0), "{}", p.name);
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_car_lists_match_the_documented_table() {
    let Some(all) = all() else { return };
    let common =
        [("trafha", 3.0, 0, 40), ("trafpickupa", 3.0, 0, 40), ("trafnews", 10.0, 1, 0), ("traftaxi", 6.0, 0, 20)];
    for name in ["default", "drag", "industrial"] {
        assert_eq!(cars(&all, name), expect(&common), "{name}");
    }
    assert_eq!(
        cars(&all, "downtown"),
        expect(&[
            ("traftaxi", 3.0, 0, 25),
            ("trafgarb", 40.0, 1, 10),
            ("trafnews", 15.0, 1, 10),
            ("traffire", 60.0, 1, 10),
            ("traf4dseda", 6.0, 0, 25),
            ("trafcourt", 6.0, 0, 20),
        ])
    );
    assert_eq!(
        cars(&all, "coastalnorth"),
        expect(&[
            ("trafstwag", 3.0, 0, 30),
            ("trafpickupa", 3.0, 0, 30),
            ("trafcemtr", 60.0, 1, 10),
            ("trafficcoup", 6.0, 0, 20),
            ("traftaxi", 15.0, 1, 10),
        ])
    );
    assert_eq!(
        cars(&all, "coastalsouth"),
        expect(&[
            ("traftaxi", 6.0, 0, 25),
            ("trafficcoup", 3.0, 0, 20),
            ("trafgarb", 60.0, 1, 10),
            ("trafpizza", 6.0, 0, 20),
            ("trafpickupa", 20.0, 1, 10),
            ("traf4dseda", 6.0, 0, 25),
        ])
    );
    assert_eq!(
        cars(&all, "collegenorth"),
        expect(&[
            ("traftaxi", 3.0, 0, 40),
            ("trafha", 3.0, 0, 40),
            ("trafvanb", 10.0, 1, 0),
            ("trafstwag", 6.0, 0, 20),
            ("trafminivan", 3.0, 0, 0),
        ])
    );
    assert_eq!(
        cars(&all, "collegesouth"),
        expect(&[
            ("trafminivan", 3.0, 0, 30),
            ("trafpizza", 3.0, 0, 30),
            ("trafficcoup", 6.0, 0, 30),
            ("trafnews", 3.0, 0, 10),
            ("trafstwag", 6.0, 0, 30),
        ])
    );
    assert_eq!(
        cars(&all, "cityhighway"),
        expect(&[
            ("semib", 30.0, 1, 10),
            ("semicrate", 30.0, 1, 10),
            ("trafcourt", 10.0, 0, 20),
            ("traf4dsedc", 6.0, 0, 20),
            ("trafdmptr", 30.0, 1, 10),
            ("trafnews", 20.0, 1, 20),
        ])
    );
    assert_eq!(
        cars(&all, "collegehighway"),
        expect(&[
            ("semilog", 30.0, 1, 10),
            ("semicon", 30.0, 1, 10),
            ("trafminivan", 10.0, 1, 20),
            ("traf4dseda", 6.0, 0, 20),
            ("trafpickupa", 3.0, 0, 10),
            ("trafficcoup", 3.0, 0, 20),
        ])
    );
}

/// `Data[0]` of a traffic-pattern zone, the kind `docs/formats/road-network.md` calls TrafficPattern.
const TRAFFIC_PATTERN_ZONE: u32 = 9;

fn zones() -> Option<TrackZones> {
    let dir = install()?;
    WorldIndex::open(&dir, DEFAULT_TRACK).unwrap().track_zones
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_pattern_hash_reproduces_the_zone_data() {
    assert_eq!(pattern_hash("collegesouth") as i32, 1837492109);
    assert_eq!(pattern_hash("downtown") as i32, 1398340799);
    assert_eq!(pattern_hash("collegenorth") as i32, 1831559237);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn every_pattern_zone_names_a_pattern_and_the_boxes_match_the_spec() {
    let (Some(all), Some(zones)) = (all(), zones()) else { return };
    assert_eq!(zones.zones.len(), 705);
    let pattern_zones: Vec<_> = zones.zones.iter().filter(|z| z.kind == TRAFFIC_PATTERN_ZONE).collect();
    assert_eq!(pattern_zones.len(), 11);
    let names: Vec<&str> = pattern_zones
        .iter()
        .map(|z| {
            let hash = z.data[0] as u32;
            all.iter()
                .find(|p| pattern_hash(&p.name) == hash)
                .unwrap_or_else(|| panic!("no pattern for {hash}"))
                .name
                .as_str()
        })
        .collect();
    let expected = [
        "collegesouth",
        "downtown",
        "collegenorth",
        "collegehighway",
        "collegehighway",
        "collegehighway",
        "collegehighway",
        "cityhighway",
        "cityhighway",
        "coastalsouth",
        "coastalnorth",
    ];
    assert_eq!(names, expected);
    // docs/specs/ai-traffic-spawning.md section 3: polygon points and bounding boxes (whole units).
    let boxes = [
        (14, [-173.0, 1875.0, 1968.0, 3061.0]),
        (10, [421.0, -655.0, 2447.0, 785.0]),
        (12, [7.0, 2970.0, 2798.0, 4720.0]),
        (7, [-658.0, 1498.0, 1479.0, 2569.0]),
        (8, [-777.0, 2125.0, 129.0, 4582.0]),
        (7, [-184.0, 4138.0, 2202.0, 4819.0]),
        (12, [1166.0, 1509.0, 2829.0, 3931.0]),
        (5, [-287.0, -780.0, 942.0, 1087.0]),
        (18, [43.0, -1290.0, 2764.0, 1096.0]),
        (10, [1207.0, -499.0, 5186.0, 2176.0]),
        (10, [1009.0, 1271.0, 4587.0, 4016.0]),
    ];
    for (zone, (points, b)) in pattern_zones.iter().zip(boxes) {
        assert_eq!(zone.polygon.len(), points);
        let got = [zone.bbox_min.x, zone.bbox_min.y, zone.bbox_max.x, zone.bbox_max.y];
        assert!(got.iter().zip(b).all(|(g, e)| (g - e).abs() < 1.0), "{got:?} vs {b:?}");
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn some_point_resolves_to_each_selectable_pattern() {
    let Some(zones) = zones() else { return };
    let selectable =
        ["collegesouth", "downtown", "collegenorth", "collegehighway", "cityhighway", "coastalsouth", "coastalnorth"];
    // The first zone in file order wins, so scan the box of each zone for a point it wins.
    for name in selectable {
        let hash = pattern_hash(name) as i32;
        let found = zones.zones.iter().filter(|z| z.kind == TRAFFIC_PATTERN_ZONE && z.data[0] == hash).any(|z| {
            let step = (z.bbox_max - z.bbox_min) / 40.0;
            (0..=40).flat_map(|i| (0..=40).map(move |j| (i, j))).any(|(i, j)| {
                let p = z.bbox_min + step * Vec2::new(i as f32, j as f32);
                zones.first_of_kind_at(TRAFFIC_PATTERN_ZONE, p).is_some_and(|w| w.data[0] == hash)
            })
        });
        assert!(found, "no point resolves to {name}");
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_downtown_box_resolves_to_downtown_through_physics_space() {
    let Some(zones) = zones() else { return };
    let hash = pattern_hash("downtown") as i32;
    let downtown = zones.zones.iter().find(|z| z.kind == TRAFFIC_PATTERN_ZONE && z.data[0] == hash).unwrap();
    let inside = downtown.polygon.iter().fold(Vec2::ZERO, |a, &p| a + p) / downtown.polygon.len() as f32;
    // The vertex average of a convex-ish polygon; fall back to the box centre if it is outside.
    let centre = (downtown.bbox_min + downtown.bbox_max) / 2.0;
    let point = [inside, centre].into_iter().find(|&p| downtown.contains(p)).expect("a point in downtown");
    let physics = from_zone_space(point, 0.0);
    assert_eq!(to_zone_space(physics), point);
    let found = zones.first_of_kind_at(TRAFFIC_PATTERN_ZONE, to_zone_space(physics)).expect("a zone");
    assert_eq!(found.data[0], pattern_hash("downtown") as i32);
}
