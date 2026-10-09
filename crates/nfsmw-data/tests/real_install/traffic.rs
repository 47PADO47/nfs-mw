//! The traffic patterns read from the install match the tables in `docs/specs/ai-traffic-spawning.md` §4.

use blackbox_attrib::Database;
use nfsmw_data::traffic::{TrafficPattern, patterns};

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
