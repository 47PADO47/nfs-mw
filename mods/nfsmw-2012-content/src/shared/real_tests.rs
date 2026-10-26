//! The shared sounds of the real 2012 install. Ignored without `MW2012_GAME_DIR` (the 2012 install).

use std::path::PathBuf;

use super::*;

fn install() -> Option<PathBuf> {
    std::env::var_os("MW2012_GAME_DIR").map(PathBuf::from)
}

fn open(root: &std::path::Path) -> (Source, SharedSet) {
    let source = Source::open(root).unwrap();
    let set = SharedSet::open(&source, &Config::default());
    (source, set)
}

fn seconds(shot: &Shot) -> f32 {
    (shot.wav.len() - 44) as f32 / 2.0 / 48_000.0
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn the_three_collision_graphs_and_the_scraping_graph_give_the_shared_hits_and_scrape() {
    let Some(root) = install() else { return };
    let (_, set) = open(&root);
    // The first selector of the three "Player Vehicle Collision <size>" graphs of each size.
    let counts: Vec<usize> = set.impact.iter().map(Vec::len).collect();
    assert_eq!(counts, [11, 11, 12], "small, medium, big: 4 + 4 + 3, 3 + 4 + 4 and 4 + 4 + 4 samples");
    for (class, shots) in set.impact.iter().enumerate() {
        assert!(shots.iter().all(|shot| (0.4..2.7).contains(&seconds(shot))), "class {class}");
    }
    let longest = |shots: &[Shot]| shots.iter().map(seconds).fold(0.0f32, f32::max);
    assert!(longest(&set.impact[0]) < longest(&set.impact[2]), "a light hit is shorter than a heavy one");
    // "Player Vehicle Scraping" plays three METALCAR one-shots of 0.5 to 0.9 s.
    assert_eq!(set.scrape.len(), 3);
    assert!(set.scrape.iter().all(|shot| (0.45..0.95).contains(&seconds(shot))));
    println!("impacts {counts:?}, scrape {:?}", set.scrape.iter().map(seconds).collect::<Vec<_>>());
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn a_car_whose_graph_imports_the_shared_pops_names_them_and_the_table_is_valid() {
    let Some(root) = install() else { return };
    let (source, set) = open(&root);
    let mut named = 0;
    let mut with_pops = 0;
    for file in source.cars() {
        let bytes = source.bundle_bytes(file).unwrap();
        let graphs = Graphs::new(&Bundle::parse(&bytes).unwrap());
        let table = set.table_for(&graphs);
        let parsed: toml::Table = toml::from_str(&table).unwrap();
        let shared = parsed["car_sound"]["shared"].as_table().unwrap();
        assert_eq!(shared["impact_big"].as_array().unwrap().len(), 12);
        assert_eq!(shared["scrape"].as_array().unwrap().len(), 3);
        named += 1;
        let Some(pops) = shared.get("pops") else { continue };
        with_pops += 1;
        let pops = pops.as_array().unwrap();
        assert!((3..=write::MAX_FILES).contains(&pops.len()), "{} pops", pops.len());
    }
    println!("{named} cars name the shared sounds, {with_pops} of them shared pops");
    assert!(named > 90 && with_pops > 70, "the 2012 cars import shared pops (90 of 93 cars read the pop values)");
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn the_set_writes_every_file_its_tables_name() {
    let Some(root) = install() else { return };
    let (source, set) = open(&root);
    let dir = std::env::temp_dir().join(format!("nfsmw-2012-shared-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let file = &source.cars()[0];
    let graphs = Graphs::new(&Bundle::parse(&source.bundle_bytes(file).unwrap()).unwrap());
    let table = set.table_for(&graphs);
    set.write(&dir).unwrap();
    let parsed: toml::Table = toml::from_str(&table).unwrap();
    for list in parsed["car_sound"]["shared"].as_table().unwrap().values() {
        for path in list.as_array().unwrap() {
            let written = dir.join("files").join(path.as_str().unwrap());
            assert!(written.is_file(), "{} is named and not written", written.display());
        }
    }
    std::fs::remove_dir_all(&dir).unwrap();
}
