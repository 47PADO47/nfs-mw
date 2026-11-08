//! Dumps solids of the installed 2005 world by name prefix, with the textures their groups use and where the scenery places them,
//! so that they can be looked at next to the 2012 models and chosen as targets (`docs/formats/chameleon-signs.md`). Needs the 2005 install;
//! writes into a folder. Not part of the normal run.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::PathBuf;

use game_install::GameDir;
use nfsmw_data::world::{DEFAULT_TRACK, WorldIndex, parse_section};
use serde_json::json;

use super::active::world_copy_base;

fn env_path(name: &str) -> PathBuf {
    PathBuf::from(std::env::var_os(name).unwrap_or_else(|| panic!("set {name}")))
}

/// `NFSMW_SIGNS_NAMES` is a comma separated list of name prefixes (upper case); `NFSMW_SIGNS_DUMP` the output folder.
#[test]
#[ignore = "needs the game (NFSMW_GAME_DIR), NFSMW_SIGNS_NAMES and NFSMW_SIGNS_DUMP"]
fn dump_solids_by_name_prefix() {
    let dir = GameDir::open(env_path("NFSMW_GAME_DIR")).unwrap();
    let out = env_path("NFSMW_SIGNS_DUMP");
    std::fs::create_dir_all(&out).unwrap();
    let prefixes: Vec<String> =
        std::env::var("NFSMW_SIGNS_NAMES").unwrap().split(',').map(|name| name.trim().to_ascii_uppercase()).collect();
    let wanted = |name: &str| prefixes.iter().any(|prefix| name.starts_with(prefix.as_str()));
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let mut shared = HashMap::new();
    for i in index.shared() {
        let section = &index.sections[i];
        let bytes = dir.read_range(&index.stream_file, u64::from(section.file_offset), section.size as usize).unwrap();
        for solid in parse_section(i, &bytes).unwrap().solids {
            shared.insert(solid.name_hash, solid);
        }
    }
    let mut dumped = std::collections::HashSet::new();
    let mut places = String::from("section\tsolid\tposition\trotation\n");
    let mut catalog = String::from("name\tsection\tcount\tvertices\tbounds_min\tbounds_max\tmarkers\tflags\n");
    for (i, section) in index.sections.iter().enumerate() {
        let bytes = dir.read_range(&index.stream_file, u64::from(section.file_offset), section.size as usize).unwrap();
        let data = parse_section(i, &bytes).unwrap();
        let mut here: HashMap<u32, &blackbox_solid::Solid> =
            shared.iter().map(|(hash, solid)| (*hash, solid)).collect();
        for solid in &data.solids {
            here.insert(solid.name_hash, solid);
            let base = world_copy_base(&solid.name).unwrap_or(&solid.name);
            if !wanted(base) {
                continue;
            }
            let _ = writeln!(
                catalog,
                "{}\t{}\t1\t{}\t{:?}\t{:?}\t{}\t{:#x}",
                solid.name,
                section.name,
                solid.vertices.len(),
                solid.bounds_min,
                solid.bounds_max,
                solid.markers.len(),
                solid.flags
            );
            if !dumped.insert(solid.name.clone()) {
                continue;
            }
            let mut groups = Vec::new();
            for group in &solid.groups {
                let first = group.first_index as usize;
                let triangles: Vec<u32> = solid.indices[first..first + group.num_indices as usize]
                    .iter()
                    .map(|index| u32::from(*index) + group.base_vertex)
                    .collect();
                let texture = group.diffuse_texture(solid);
                if let Some(hash) = texture {
                    if let Some(picture) = data.textures.iter().find(|t| t.name_hash == hash) {
                        if let Some(rgba) = blackbox_tpk::decode_rgba8(picture) {
                            let _ = std::fs::write(out.join(format!("{hash:08X}.rgba")), &rgba);
                            let _ = std::fs::write(
                                out.join(format!("{hash:08X}.txt")),
                                format!(
                                    "{} {} {} {}",
                                    picture.width, picture.height, picture.name, picture.compression_type
                                ),
                            );
                        }
                    }
                }
                groups.push(json!({
                    "effect": group.effect_id, "flags": group.flags, "texture": texture,
                    "triangles": triangles, "light_material": group.light_material,
                }));
            }
            let vertices: Vec<_> =
                solid.vertices.iter().map(|v| json!([v.position, v.normal, v.uv, v.color_bgra])).collect();
            let markers: Vec<_> = solid
                .markers
                .iter()
                .map(|m| json!({"hash": m.name_hash, "param": m.int_param, "at": m.translation(), "matrix": m.matrix}))
                .collect();
            let doc = json!({
                "name": solid.name, "bounds_min": solid.bounds_min, "bounds_max": solid.bounds_max,
                "transform": solid.transform, "flags": solid.flags, "polys": solid.num_polys, "density": solid.density,
                "vertices": vertices, "groups": groups, "markers": markers,
                "textures": solid.texture_hashes, "light_materials": solid.light_material_hashes,
            });
            std::fs::write(out.join(format!("{}.json", solid.name)), doc.to_string()).unwrap();
        }
        for scenery in &data.scenery {
            for instance in &scenery.instances {
                let Some(info) = scenery.info_of(instance) else { continue };
                for key in &info.solid_keys {
                    let Some(solid) = here.get(key) else { continue };
                    if !wanted(&solid.name) {
                        continue;
                    }
                    let at = instance.position;
                    let _ = writeln!(
                        places,
                        "{}\t{}\t{:.2},{:.2},{:.2}\t{:?}",
                        section.name, solid.name, at[0], at[1], at[2], instance.rotation
                    );
                }
            }
        }
    }
    std::fs::write(out.join("places.tsv"), places).unwrap();
    std::fs::write(out.join("catalog.tsv"), catalog).unwrap();
}

/// What the sets of a mod folder (`NFSMW_SIGNS_MOD`: its `replace/` and `files/`) do to the installed world: for every solid they list,
/// whether it was replaced in the sections that hold it, with the old and the new box. Writes `replaced.tsv` next to the dump.
#[test]
#[ignore = "needs the game (NFSMW_GAME_DIR), NFSMW_SIGNS_MOD and NFSMW_SIGNS_DUMP"]
fn report_what_the_sets_do_to_the_installed_world() {
    use std::sync::Arc;

    use blackbox_modding::content::parse_replace_set;

    use super::active::{Active, LoadedSet};

    let dir = GameDir::open(env_path("NFSMW_GAME_DIR")).unwrap();
    let out = env_path("NFSMW_SIGNS_DUMP");
    let mod_dir = env_path("NFSMW_SIGNS_MOD");
    let mut loaded = Vec::new();
    let mut listed: std::collections::HashSet<String> = Default::default();
    for entry in std::fs::read_dir(mod_dir.join("replace")).unwrap().flatten() {
        let path = entry.path();
        let id = path.file_stem().unwrap().to_string_lossy().into_owned();
        let set = Arc::new(parse_replace_set(&id, &std::fs::read_to_string(&path).unwrap()).unwrap());
        let geometry = std::fs::read(mod_dir.join("files").join(set.geometry.as_ref().unwrap())).unwrap();
        let models = blackbox_solid::read_solids(&geometry).unwrap();
        let pack = std::fs::read(mod_dir.join("files").join(set.textures.as_ref().unwrap())).unwrap();
        let added: Vec<_> =
            blackbox_tpk::read_texture_packs(&pack).unwrap().into_iter().flat_map(|pack| pack.textures).collect();
        for model in &set.models {
            if let blackbox_modding::content::ReplaceTarget::Name(name) = &model.targets[0] {
                listed.insert(name.clone());
            }
        }
        loaded.push(LoadedSet { owner: "test".into(), order: 0, set, models, added, replaced: Vec::new() });
    }
    let active = Active::build(loaded);
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let mut report = String::from("section\tsolid\tstatus\told_min\told_max\tnew_min\tnew_max\tmarkers\n");
    for (i, section) in index.sections.iter().enumerate() {
        let bytes = dir.read_range(&index.stream_file, u64::from(section.file_offset), section.size as usize).unwrap();
        let mut data = parse_section(i, &bytes).unwrap();
        let before = data.solids.clone();
        active.apply(&mut data);
        for (old, new) in before.iter().zip(&data.solids) {
            let base = world_copy_base(&old.name).unwrap_or(&old.name);
            if !listed.contains(base) {
                continue;
            }
            let status = if old.vertices.len() == new.vertices.len()
                && old.bounds_min == new.bounds_min
                && old.indices == new.indices
            {
                "KEPT"
            } else {
                "replaced"
            };
            let _ = writeln!(
                report,
                "{}\t{}\t{}\t{:?}\t{:?}\t{:?}\t{:?}\t{}",
                section.name,
                old.name,
                status,
                old.bounds_min,
                old.bounds_max,
                new.bounds_min,
                new.bounds_max,
                new.markers.len()
            );
        }
    }
    std::fs::write(out.join("replaced.tsv"), report).unwrap();
}
