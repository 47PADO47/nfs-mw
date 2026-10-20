//! The install's data behind the roadblocks, the spike strips and the helicopter's spawn rules. These need the game
//! (`NFSMW_GAME_DIR`) and are ignored without it.

use blackbox_attrib::{CollectionRef, Database, Value};
use blackbox_pursuit::roadblock::request::odds;
use game_install::GameDir;
use nfsmw_data::world::{DEFAULT_TRACK, PropCatalog, SharedAssets, WorldIndex};

use super::props::{Physics, SAWHORSE_MODEL, SPIKE_MODEL};
use crate::scenes::world::ai::police::heli::SEARCH_SPAWN_CHANCE;
use crate::settings::helicopter::vanilla;

fn install() -> Option<(GameDir, Database)> {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR")?).ok()?;
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").ok()?).ok()?;
    Some((dir, db))
}

/// The `pursuitlevels` collection of heat level `heat` (1 to 10) in the `heattable` of `pursuitescalation`.
fn level(db: &Database, heat: usize) -> CollectionRef<'_> {
    let escalation = db.collections_of("pursuitescalation").find(|c| c.name() == Some("default")).unwrap();
    let Some(Value::Array(rows)) = escalation.get("heattable") else { panic!("no heattable") };
    db.resolve(rows[heat - 1].as_ref_spec().unwrap()).unwrap()
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_roadblock_props_have_bounds_and_models() {
    let Some((dir, db)) = install() else { return };
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let catalog = PropCatalog::new(index.prop_bounds.clone(), &db);
    let physics = Physics::from_catalog(&catalog);
    let [along, across, up] = physics.strip_half.expect("the strip has a collision box");
    assert!((along - 0.378).abs() < 0.001 && (across - 3.265).abs() < 0.001 && (up - 0.254).abs() < 0.001);
    let (bounds, spec) = physics.sawhorse.expect("the sawhorse has bounds and dynamics");
    assert!(bounds.nodes.len() >= 4, "a body and its rail and legs");
    assert_eq!(spec.mass, 100.0, "the `sawhorse` smackable's mass");
    let shared = SharedAssets::load(&dir, DEFAULT_TRACK).unwrap();
    for name in [SPIKE_MODEL, SAWHORSE_MODEL] {
        assert!(shared.find(name).is_some(), "{name} is in the track's shared scenery");
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_level_rows_match_the_roadblock_and_helicopter_tables() {
    let Some((_, db)) = install() else { return };
    let mut wrong = Vec::new();
    for heat in 1..=10usize {
        let row = level(&db, heat);
        let want = odds(heat as u32);
        let f = |name: &str| row.get_f32(name).unwrap_or(f32::NAN);
        let checks = [
            ("roadblockprobability", want.probability),
            ("roadblockspikechance", want.spike_chance),
            ("SearchModeRoadblockChance", want.search_chance),
            ("SearchModeRoadblockRadius", want.search_radius),
            ("roadblockhelichance", 0.0),
            ("SearchModeHeliSpawnChance", SEARCH_SPAWN_CHANCE[heat - 1]),
            ("HeliFuelTime", vanilla::FUEL_HEAT[heat - 1]),
            ("TimeBetweenHeliActive", vanilla::RESPAWN_HEAT[heat - 1]),
            ("heliLOSdistance", vanilla::LINE_OF_SIGHT_HEAT[heat - 1]),
        ];
        for (name, expected) in checks {
            if f(name) != expected {
                wrong.push(format!("heat {heat} {name}: install {} table {expected}", f(name)));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("; "));
}
