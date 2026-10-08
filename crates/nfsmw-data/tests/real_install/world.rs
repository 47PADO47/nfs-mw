//! The whole streamed city: every section parses, and scenery resolves the way
//! `docs/formats/maps.md` says (own tile or shared sets, never another tile).

use std::collections::HashSet;

use blackbox_solid::Solid;
use nfsmw_data::world::{DEFAULT_TRACK, WorldIndex, load_section};

use crate::install;

pub fn assert_indices_in_range(s: &Solid, context: &str) {
    for g in &s.groups {
        let range = g.first_index as usize..(g.first_index + g.num_indices) as usize;
        let max = s.indices[range].iter().map(|&i| u32::from(i) + g.base_vertex).max().unwrap_or(0);
        assert!(g.num_indices == 0 || (max as usize) < s.vertices.len(), "{context}: {}: index out of range", s.name);
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR); reads the whole 533 MB stream"]
fn every_world_section_parses_and_scenery_resolves() {
    let Some(dir) = install() else { return };
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    assert_eq!(index.sections.len(), 720);
    let mut file = dir.open_file(&index.stream_file).unwrap();

    let (mut solids, mut textures, mut instances) = (0, 0, 0);
    let mut shared_solids = HashSet::new();
    let mut tiles = Vec::new();
    for (i, section) in index.sections.iter().enumerate() {
        let data = load_section(&mut file, i, section).unwrap_or_else(|e| panic!("{}: {e:#}", section.name));
        for s in &data.solids {
            assert_indices_in_range(s, &section.name);
        }
        solids += data.solids.len();
        textures += data.textures.len();
        instances += data.scenery.iter().map(|s| s.instances.len()).sum::<usize>();
        if section.is_spatial() {
            tiles.push(data);
        } else {
            shared_solids.extend(data.solids.iter().map(|s| s.name_hash));
        }
    }
    eprintln!("{solids} solids, {textures} textures, {instances} scenery instances");
    assert_eq!(solids, 20_377);
    assert_eq!(instances, 77_783);

    let (mut resolved, mut missing) = (0, 0);
    for tile in &tiles {
        let own: HashSet<u32> = tile.solids.iter().map(|s| s.name_hash).collect();
        for section in &tile.scenery {
            for inst in &section.instances {
                match section.info_of(inst).and_then(|info| info.best_solid()) {
                    Some(k) if own.contains(&k) || shared_solids.contains(&k) => resolved += 1,
                    _ => missing += 1,
                }
            }
        }
    }
    eprintln!("tile instances: {resolved} resolved, {missing} not");
    assert!(missing * 1000 < resolved, "too many unresolved instances: {missing}");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR); reads the whole 533 MB stream"]
fn collision_packs_come_with_map_tiles() {
    let Some(dir) = install() else { return };
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    assert!(index.collision_grid.is_some(), "the track file has a collision grid");
    let mut file = dir.open_file(&index.stream_file).unwrap();
    // A tile carries the pack of the collision section it covers. The two numberings differ
    // (tile A41 holds pack 101; some tiles hold a pack of their own number), so only the totals are checked.
    let mut packs = HashSet::new();
    for (i, section) in index.sections.iter().enumerate() {
        let data = load_section(&mut file, i, section).unwrap();
        for pack in &data.collision {
            assert!(packs.insert(pack.section), "pack {} is in two sections", pack.section);
        }
        if !section.is_spatial() {
            assert!(data.collision.is_empty(), "{}", section.name);
        }
    }
    assert_eq!(packs.len(), 390);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR); reads the whole 533 MB stream"]
fn prop_bounds_sit_where_their_scenery_is() {
    use blackbox_attrib::Database;
    use glam::{Mat4, Vec3};
    use nfsmw_data::world::{PropCatalog, PropKind};

    let Some(dir) = install() else { return };
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    assert_eq!(index.prop_bounds.len(), 405);
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
    let catalog = PropCatalog::new(index.prop_bounds.clone(), &db);
    let mut file = dir.open_file(&index.stream_file).unwrap();

    let (mut checked, mut matched_names, mut far) = (0, std::collections::HashSet::new(), 0);
    let (mut light, mut rigid) = (0, 0);
    for (i, section) in index.sections.iter().enumerate() {
        let data = load_section(&mut file, i, section).unwrap();
        for sec in &data.scenery {
            for inst in &sec.instances {
                let Some(info) = sec.info_of(inst) else { continue };
                let Some(shape) = catalog.shape(&info.name) else { continue };
                matched_names.insert(info.name.clone());
                match shape.kind {
                    PropKind::Light { .. } => light += 1,
                    PropKind::Rigid => rigid += 1,
                }
                let m = Mat4::from_cols_array_2d(&inst.matrix_columns());
                // The prop's box centre in the world must lie within the instance's own box, enlarged a little.
                let (lo, hi) = (Vec3::from(inst.bbox_min) - 1.5, Vec3::from(inst.bbox_max) + 1.5);
                for b in &shape.boxes {
                    let c = m.transform_point3(b.centre);
                    checked += 1;
                    if c.cmplt(lo).any() || c.cmpgt(hi).any() {
                        far += 1;
                    }
                }
            }
        }
    }
    eprintln!(
        "{} named props, {checked} boxes checked, {far} outside their scenery; {light} light and {rigid} rigid instances",
        matched_names.len()
    );
    assert!(matched_names.len() >= 200, "{} props matched", matched_names.len());
    assert!(far * 20 < checked, "{far} of {checked} prop boxes are not where their scenery is");
    assert!(light > 1000 && rigid > 1000);
}

/// The scenery-group barriers (road blocks that are off in free roam) are 18 % of all barriers: a vehicle's
/// ray goes through them once the group mask is set, and still stops at the others.
#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn group_barriers_are_left_out_of_vehicle_queries() {
    use blackbox_collision::{CollisionWorld, GROUP_EXCLUSION, HitKind, RayOptions};

    let Some(dir) = install() else { return };
    let index = nfsmw_data::world::WorldIndex::open(&dir, "L2RA").unwrap();
    let packs = blackbox_collision::read_collision_packs(&dir.read("TRACKS/STREAML2RA.BUN").unwrap()).unwrap();
    let mut world = CollisionWorld::new(index.collision_grid.clone());
    for pack in packs.iter().cloned() {
        world.insert(pack);
    }
    let everything = RayOptions::default();
    let vehicle = RayOptions { exclude: u32::from(GROUP_EXCLUSION), ..everything };
    let (mut grouped_hits, mut passed, mut checked) = (0, 0, 0);
    for pack in &packs {
        for (i, inst) in pack.instances.iter().enumerate().filter(|(_, inst)| inst.group != 0) {
            let Some(bar) = pack.article_of(i).and_then(|a| a.barriers.first()) else { continue };
            // A segment across the middle of the barrier, a metre either side of it.
            let (a, b) = (inst.to_world(bar.p0), inst.to_world(bar.p1));
            let mid = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0, (a[2] + b[2]) / 2.0];
            let n = bar.normal();
            let n = inst.dir_to_world(n);
            let from = [mid[0] + n[0], mid[1], mid[2] + n[2]];
            let to = [mid[0] - n[0], mid[1], mid[2] - n[2]];
            checked += 1;
            let Some(hit) = world.ray_cast(from, to, &everything) else { continue };
            if hit.kind != HitKind::Barrier || hit.section != pack.section || hit.instance != i {
                continue;
            }
            grouped_hits += 1;
            let after = world.ray_cast(from, to, &vehicle);
            passed += usize::from(after.is_none_or(|h| h.section != pack.section || h.instance != i));
        }
    }
    assert!(checked > 500 && grouped_hits > 300, "{checked} checked, {grouped_hits} hit");
    assert_eq!(passed, grouped_hits, "every grouped barrier is passable with the vehicle mask");
}
