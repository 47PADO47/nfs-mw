//! A look at what the trees variant does to the installed world: for every solid it replaces, the old and the new bounds. Needs
//! the 2005 install and the converted pack; writes a tab-separated report. Not part of the normal run.

use std::fmt::Write as _;
use std::path::PathBuf;

use blackbox_collision::CollisionWorld;
use blackbox_modding::content::parse_model_swap_profile;
use game_install::GameDir;
use nfsmw_data::world::{DEFAULT_TRACK, WorldIndex, parse_section};

use super::active::world_copy_base;
use super::tests::build;

fn env_path(name: &str) -> PathBuf {
    PathBuf::from(std::env::var_os(name).unwrap_or_else(|| panic!("set {name}")))
}

/// The lowest z of the vertices of the groups of `effect` (0 trunk, 6 leaves), `None` when it has none.
fn group_floor(solid: &blackbox_solid::Solid, effect: u32) -> Option<f32> {
    let mut low: Option<f32> = None;
    for group in solid.groups.iter().filter(|group| group.effect_id == effect) {
        low = Some(low.map_or(group.bounds_min[2], |low| low.min(group.bounds_min[2])));
    }
    low
}

#[test]
#[ignore = "needs the game (NFSMW_GAME_DIR), the pack (NFSMW_TREES_DIR) and NFSMW_TREES_REPORT"]
fn report_what_the_trees_do_to_the_installed_world() {
    let dir = GameDir::open(env_path("NFSMW_GAME_DIR")).unwrap();
    let pack_dir = env_path("NFSMW_TREES_DIR");
    let profile_text = std::fs::read_to_string(pack_dir.join("profile.toml")).unwrap();
    let profile = parse_model_swap_profile(&profile_text).unwrap();
    let models = blackbox_solid::read_solids(&std::fs::read(pack_dir.join("GEOMETRY.BIN")).unwrap()).unwrap();
    let mut pictures = Vec::new();
    for pack in blackbox_tpk::read_texture_packs(&std::fs::read(pack_dir.join("TEXTURES.BIN")).unwrap()).unwrap() {
        pictures.extend(pack.textures);
    }
    let pack = build(&profile, models, pictures);
    let listed: std::collections::HashSet<&str> = profile.swaps.iter().map(|swap| swap.solid.as_str()).collect();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let mut report = String::from(
        "section\tname\tframe\tfit\told_min\told_max\tnew_min\tnew_max\ttrunk_floor\tleaf_floor\tcentre\tangle\tlength\tspan_x\tspan_y\tpivot\tground_low\tground_high\n",
    );
    for (i, section) in index.sections.iter().enumerate() {
        let bytes = dir.read_range(&index.stream_file, u64::from(section.file_offset), section.size as usize).unwrap();
        let mut data = parse_section(i, &bytes).unwrap();
        let mut terrain = CollisionWorld::new(index.collision_grid.clone());
        for pack in &data.collision {
            terrain.insert(pack.clone());
        }
        let before = data.solids.clone();
        pack.apply(&mut data);
        for (old, new) in before.iter().zip(&data.solids) {
            let world = world_copy_base(&old.name);
            let base = world.unwrap_or(&old.name);
            if !listed.contains(base) {
                continue;
            }
            let fit = profile.swaps.iter().find(|swap| swap.solid == base).map(|swap| swap.fit);
            let ground = |x: f32, y: f32| {
                terrain.ground(-y, x, old.bounds_max[2] + 5.0, old.bounds_min[2] - 80.0).map(|hit| hit.point[1])
            };
            let (from, to) = (old.bounds_min, old.bounds_max);
            let centre = [0.5 * (from[0] + to[0]), 0.5 * (from[1] + to[1])];
            let samples = [ground(centre[0], centre[1]), ground(from[0], from[1]), ground(to[0], to[1])];
            let heights: Vec<f32> = samples.into_iter().flatten().collect();
            let low = heights.iter().copied().fold(f32::MAX, f32::min);
            let high = heights.iter().copied().fold(f32::MIN, f32::max);
            let ground_text = match heights.is_empty() {
                true => "-\t-".to_string(),
                false => format!("{low:.1}\t{high:.1}"),
            };
            let f = |v: [f32; 3]| format!("{:.1},{:.1},{:.1}", v[0], v[1], v[2]);
            let _ = writeln!(
                report,
                "{}\t{}\t{}\t{:?}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.1},{:.1}\t{:.2}\t{:.1}\t{:.1}\t{:.1}\t{:.1},{:.1},{:.1}\t{}",
                section.name,
                old.name,
                if world.is_some() { "world" } else { "local" },
                fit,
                f(old.bounds_min),
                f(old.bounds_max),
                f(new.bounds_min),
                f(new.bounds_max),
                group_floor(old, 0).map_or("-".into(), |v| format!("{v:.1}")),
                group_floor(old, 6).map_or("-".into(), |v| format!("{v:.1}")),
                centre[0],
                centre[1],
                0.0,
                (to[0] - from[0]).max(to[1] - from[1]),
                old.bounds_max[0] - old.bounds_min[0],
                old.bounds_max[1] - old.bounds_min[1],
                old.transform[12],
                old.transform[13],
                old.transform[14],
                ground_text,
            );
        }
    }
    std::fs::write(env_path("NFSMW_TREES_REPORT"), report).unwrap();
}

#[test]
#[ignore = "needs the pack (NFSMW_TREES_DIR) and NFSMW_TREES_REPORT"]
fn report_the_models_of_the_pack() {
    let pack_dir = env_path("NFSMW_TREES_DIR");
    let models = blackbox_solid::read_solids(&std::fs::read(pack_dir.join("GEOMETRY.BIN")).unwrap()).unwrap();
    let mut report = String::new();
    for model in &models {
        let _ = writeln!(
            report,
            "{} bounds {:?} {:?} vertices {}",
            model.name,
            model.bounds_min.map(|v| (v * 10.0).round() / 10.0),
            model.bounds_max.map(|v| (v * 10.0).round() / 10.0),
            model.vertices.len()
        );
        for group in &model.groups {
            let _ = writeln!(
                report,
                "    effect {} triangles {} bounds {:?} {:?}",
                group.effect_id,
                group.num_triangles,
                group.bounds_min.map(|v| (v * 10.0).round() / 10.0),
                group.bounds_max.map(|v| (v * 10.0).round() / 10.0)
            );
        }
    }
    std::fs::write(env_path("NFSMW_TREES_REPORT").with_extension("models.txt"), report).unwrap();
}

/// Every distinct solid name of the world (a world-space copy under the name it was made of): how many, how big, and the
/// effects its groups use. For choosing what else a mod could replace.
#[test]
#[ignore = "needs the game (NFSMW_GAME_DIR) and NFSMW_TREES_REPORT"]
fn report_the_solid_names_of_the_world() {
    let dir = GameDir::open(env_path("NFSMW_GAME_DIR")).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let mut names: std::collections::BTreeMap<String, (usize, usize, [f32; 3], Vec<u32>)> = Default::default();
    for (i, section) in index.sections.iter().enumerate() {
        let bytes = dir.read_range(&index.stream_file, u64::from(section.file_offset), section.size as usize).unwrap();
        let data = parse_section(i, &bytes).unwrap();
        for solid in &data.solids {
            let base = world_copy_base(&solid.name).unwrap_or(&solid.name).to_string();
            let entry = names.entry(base).or_default();
            entry.0 += 1;
            entry.1 += solid.indices.len() / 3;
            let size = [
                solid.bounds_max[0] - solid.bounds_min[0],
                solid.bounds_max[1] - solid.bounds_min[1],
                solid.bounds_max[2] - solid.bounds_min[2],
            ];
            entry.2 = size;
            for group in &solid.groups {
                if !entry.3.contains(&group.effect_id) {
                    entry.3.push(group.effect_id);
                }
            }
        }
    }
    let mut report = String::new();
    for (name, (count, triangles, size, effects)) in &names {
        let _ =
            writeln!(report, "{name}\t{count}\t{triangles}\t{:.1}x{:.1}x{:.1}\t{effects:?}", size[0], size[1], size[2]);
    }
    std::fs::write(env_path("NFSMW_TREES_REPORT").with_extension("names.tsv"), report).unwrap();
}

/// Where the game places the solids of the trees that are modelled around their own origin: the position of each instance.
#[test]
#[ignore = "needs the game (NFSMW_GAME_DIR), the pack (NFSMW_TREES_DIR) and NFSMW_TREES_REPORT"]
fn report_where_the_local_trees_are_placed() {
    let dir = GameDir::open(env_path("NFSMW_GAME_DIR")).unwrap();
    let profile_text = std::fs::read_to_string(env_path("NFSMW_TREES_DIR").join("profile.toml")).unwrap();
    let profile = parse_model_swap_profile(&profile_text).unwrap();
    let listed: std::collections::HashSet<&str> = profile.swaps.iter().map(|swap| swap.solid.as_str()).collect();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let mut report = String::from("section\tsolid\tposition\tbounds_min\tbounds_max\n");
    let mut shared = std::collections::HashMap::new();
    for i in index.shared() {
        let section = &index.sections[i];
        let bytes = dir.read_range(&index.stream_file, u64::from(section.file_offset), section.size as usize).unwrap();
        for solid in parse_section(i, &bytes).unwrap().solids {
            shared.insert(solid.name_hash, solid);
        }
    }
    for (i, section) in index.sections.iter().enumerate() {
        let bytes = dir.read_range(&index.stream_file, u64::from(section.file_offset), section.size as usize).unwrap();
        let data = parse_section(i, &bytes).unwrap();
        for scenery in &data.scenery {
            for instance in &scenery.instances {
                let Some(info) = scenery.info_of(instance) else { continue };
                for key in &info.solid_keys {
                    let Some(solid) = shared.get(key) else { continue };
                    if !listed.contains(solid.name.as_str()) {
                        continue;
                    }
                    let at = instance.position;
                    let _ = writeln!(
                        report,
                        "{}\t{}\t{:.1},{:.1},{:.1}\t{:?}\t{:?}",
                        section.name, solid.name, at[0], at[1], at[2], solid.bounds_min, solid.bounds_max
                    );
                }
            }
        }
    }
    std::fs::write(env_path("NFSMW_TREES_REPORT").with_extension("places.tsv"), report).unwrap();
}

#[test]
#[ignore = "needs the game (NFSMW_GAME_DIR) and NFSMW_NAMES_REPORT: lists lamp-like solids and large textures of the tiles near the city centre"]
fn list_the_names_near_the_centre() {
    let dir = GameDir::open(env_path("NFSMW_GAME_DIR")).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let centre = index.centre();
    let mut near: Vec<(usize, f32)> = index
        .sections
        .iter()
        .enumerate()
        .map(|(i, section)| (i, if section.is_spatial() { section.distance_to(centre[0], centre[1]) } else { 0.0 }))
        .collect();
    near.sort_by(|a, b| a.1.total_cmp(&b.1));
    let mut report = format!("centre {centre:?}\n");
    for (i, distance) in near.into_iter().take(40) {
        let section = &index.sections[i];
        let bytes = dir.read_range(&index.stream_file, u64::from(section.file_offset), section.size as usize).unwrap();
        let data = parse_section(i, &bytes).unwrap();
        let _ = writeln!(report, "section {i} {} number {} distance {distance:.0}", section.name, section.number);
        for solid in &data.solids {
            let (low, high) = (solid.bounds_min, solid.bounds_max);
            let name = solid.name.to_ascii_uppercase();
            let (width, height) = ((high[0] - low[0]).max(high[1] - low[1]), high[2] - low[2]);
            let thin_and_tall = height > 5.0 && width < 3.0 && !name.contains("_DEINST");
            if thin_and_tall || name.contains("STREETLIGHT") || name.contains("LAMP") || name.contains("POLE") {
                let _ = writeln!(report, "solid {} {low:?} {high:?}", solid.name);
            }
        }
        for texture in &data.textures {
            let upper = texture.name.to_ascii_uppercase();
            let road = ["ROAD", "ASPH", "PARK", "LOT", "STREET", "CONCRETE"].iter().any(|word| upper.contains(word));
            if texture.width >= 128 || road {
                let _ = writeln!(
                    report,
                    "texture {} {}x{} {:?} mips {}",
                    texture.name, texture.width, texture.height, texture.format, texture.mip_levels
                );
            }
        }
    }
    std::fs::write(env_path("NFSMW_NAMES_REPORT"), report).unwrap();
}
