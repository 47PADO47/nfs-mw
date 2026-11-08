//! The trees of the real 2012 install. Ignored without `MW2012_GAME_DIR` (the 2012 install).

use blackbox_modding::content::parse_model_swap_profile;
use blackbox_solid::read_solids;
use blackbox_tpk::read_texture_packs;

use super::*;
use crate::source::Source;

fn install() -> Option<Source> {
    let root = std::env::var_os("MW2012_GAME_DIR")?;
    Source::open(std::path::Path::new(&root)).ok()
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn every_species_is_read_with_its_levels_and_both_kinds_of_mesh() {
    let Some(source) = install() else { return };
    let (models, _) = read_models(&source);
    assert_eq!(models.len(), SPECIES.len(), "a species could not be read");
    for (species, model) in &models {
        assert!(model.levels.len() >= 3, "{}: {} levels", species.key, model.levels.len());
        let finest = &model.levels[0];
        assert!(finest.parts.iter().any(|part| part.role == read::Role::Foliage), "{} has no leaves", species.key);
        for part in model.levels.iter().flat_map(|level| &level.parts) {
            assert!(model.textures.contains_key(&part.texture), "{}", species.key);
            assert!(part.triangles.iter().flatten().all(|&index| (index as usize) < part.positions.len()));
            assert!(part.positions.iter().flatten().all(|v| v.is_finite()));
        }
    }
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn the_pack_reads_back_as_solids_textures_and_a_profile_the_game_accepts() {
    let Some(source) = install() else { return };
    let (models, _) = read_models(&source);
    let pack = build::build(&models, 1024).unwrap();
    assert_eq!(pack.species.len(), SPECIES.len());
    let solids = read_solids(&pack.geometry).unwrap();
    let hashes: Vec<u32> = read_texture_packs(&pack.textures)
        .unwrap()
        .into_iter()
        .flat_map(|texture_pack| texture_pack.textures)
        .map(|texture| texture.name_hash)
        .collect();
    for solid in &solids {
        assert!(solid.texture_hashes.iter().all(|hash| hashes.contains(hash)), "{}", solid.name);
        assert!(solid.groups.iter().all(|group| matches!(group.effect_id, 0 | 6)), "{}", solid.name);
        // The tree stands up: it is taller than it is wide, or at least reaches above its base.
        assert!(solid.bounds_max[2] > 1.0, "{}: {:?}", solid.name, solid.bounds_max);
    }
    let profile = parse_model_swap_profile(&pack.profile).unwrap();
    for swap in &profile.swaps {
        assert!(solids.iter().any(|solid| solid.name == swap.model), "{} has no model {}", swap.solid, swap.model);
    }
    assert_eq!(profile.swaps.len(), 78, "{} swaps", profile.swaps.len());
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn the_trees_are_read_by_ranges_and_written_in_a_few_megabytes() {
    let Some(source) = install() else { return };
    let started = std::time::Instant::now();
    let (models, taken) = read_models(&source);
    let read_time = started.elapsed();
    assert_eq!(models.len(), SPECIES.len());
    println!("read {} KiB of {} KiB of the bundles in {read_time:?}", taken.read / 1024, taken.total / 1024);
    assert!(taken.read < 16 * 1024 * 1024, "megabytes of the bundles' hundreds, not all of them: {}", taken.read);
    let pack = build::build(&models, 1024).unwrap();
    println!(
        "wrote geometry {} KiB, textures {} KiB, in {:?} all told",
        pack.geometry.len() / 1024,
        pack.textures.len() / 1024,
        started.elapsed()
    );
    assert!(pack.geometry.len() + pack.textures.len() < 12 * 1024 * 1024);
}
