//! A census of the pursuit breakers the installed track places, by streaming section (installed data).

use super::*;
use blackbox_scenery::{layout::MOST_WANTED, read_object_spawners, read_scenery_sections};
use game_install::GameDir;
use nfsmw_data::world::{DEFAULT_TRACK, PropCatalog, WorldIndex};

/// Every placement of an event engine that opens a cop zone, with its section: `(section, engine, physics position)`.
fn breaker_placements(dir: &GameDir, index: &WorldIndex, data: &PhysicsData) -> Vec<(String, String, [f32; 3])> {
    let catalog =
        PropCatalog::new(index.prop_bounds.clone(), data.database()).with_hierarchies(index.model_hierarchies.clone());
    let mut out = Vec::new();
    for section in &index.sections {
        let bytes = dir.read_range(&index.stream_file, u64::from(section.file_offset), section.size as usize).unwrap();
        let scenery = read_scenery_sections(&bytes, &MOST_WANTED).unwrap();
        let mut hooks = std::collections::HashMap::new();
        for sc in &scenery {
            for hook in &sc.override_hooks {
                let instance = &sc.instances[usize::from(hook.instance_index)];
                hooks.insert(u32::from(hook.override_index), (instance, sc.info_of(instance).unwrap()));
            }
        }
        for spawn in read_object_spawners(&bytes, &MOST_WANTED.spawners).unwrap() {
            let Some(&(_, info)) = hooks.get(&spawn.override_index) else { continue };
            let Some(shape) = catalog.for_spawner(&info.name, &spawn) else { continue };
            let Some(dynamics) = shape.dynamics.as_ref() else { continue };
            let Some(sequence) = data.sequence(dynamics.event_sequencer) else { continue };
            if !sequence.breaker_profile().is_breaker() {
                continue;
            }
            let engine = sequence.package.engines[sequence.engine].name.clone();
            out.push((section.name.clone(), engine, spawn.position));
        }
    }
    out
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_track_places_breakers_in_many_sections() {
    let Some(path) = std::env::var_os("NFSMW_GAME_DIR") else { return };
    let dir = GameDir::open(path).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let mut data = PhysicsData::load(&dir).unwrap();
    data.set_sequences(index.sequences.clone());
    let placements = breaker_placements(&dir, &index, &data);
    let mut by_engine = std::collections::BTreeMap::<&str, usize>::new();
    let mut sections = std::collections::BTreeMap::<&str, Vec<&str>>::new();
    for (section, engine, _) in &placements {
        *by_engine.entry(engine).or_default() += 1;
        sections.entry(section).or_default().push(engine);
    }
    eprintln!("{} breaker placements in {} sections: {by_engine:?}", placements.len(), sections.len());
    for (section, engines) in &sections {
        eprintln!("  {section}: {engines:?}");
    }
    // Nine gas stations, three donut/tyre shop roofs, four gazebos, two sailboats: the set pieces of the city.
    assert_eq!(by_engine["gasstationroof_seq"], 9);
    assert_eq!(by_engine["donutshoproof_seq"], 3);
    assert_eq!(by_engine["gazebo_seq"], 4);
    assert_eq!(by_engine["sailboat_seq"], 2);
    assert!(placements.len() >= 100, "{} placements", placements.len());
    assert!(sections.len() >= 25, "{} sections", sections.len());
}
