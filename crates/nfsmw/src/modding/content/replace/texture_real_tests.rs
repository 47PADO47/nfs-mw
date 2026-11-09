//! Where the textures a set replaces live in the installed 2005 world: in the sections of the shared sets (`V`, `X`, `Y`: loaded once when the
//! world opens) or in map tiles (streamed as the player drives). Needs the 2005 install and the folder of the 2012 mod
//! (`NFSMW_SIGNS_MOD`, its `replace/`). Not part of the normal run.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use blackbox_modding::content::parse_replace_set;
use game_install::GameDir;
use nfsmw_data::world::{DEFAULT_TRACK, WorldIndex, parse_section};

use super::active::target_hash;

fn env_path(name: &str) -> PathBuf {
    PathBuf::from(std::env::var_os(name).unwrap_or_else(|| panic!("set {name}")))
}

/// For the texture set `road_textures-mw2012.toml` (`NFSMW_SIGNS_MOD`): how many of its textures are held by a shared section, by a tile, and by
/// both, so that it is known which of them a change made with the world open reaches by streaming the tiles again and which only through the
/// redirect of the shared textures (`residency/shared_textures.rs`).
#[test]
#[ignore = "needs the game (NFSMW_GAME_DIR) and NFSMW_SIGNS_MOD"]
fn report_where_the_replaced_textures_live() {
    let dir = GameDir::open(env_path("NFSMW_GAME_DIR")).unwrap();
    let path = env_path("NFSMW_SIGNS_MOD").join("replace").join("road_textures-mw2012.toml");
    let set = parse_replace_set("road_textures-mw2012", &std::fs::read_to_string(path).unwrap()).unwrap();
    let wanted: HashSet<u32> =
        set.texture_entries.iter().filter_map(|entry| entry.replaces.as_ref()).map(target_hash).collect();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let shared: HashSet<usize> = index.shared().collect();
    let mut where_found: HashMap<u32, (usize, usize)> = HashMap::new();
    for (i, section) in index.sections.iter().enumerate() {
        let bytes = dir.read_range(&index.stream_file, u64::from(section.file_offset), section.size as usize).unwrap();
        for texture in parse_section(i, &bytes).unwrap().textures {
            if !wanted.contains(&texture.name_hash) {
                continue;
            }
            let found = where_found.entry(texture.name_hash).or_default();
            match shared.contains(&i) {
                true => found.0 += 1,
                false => found.1 += 1,
            }
        }
    }
    let in_shared = where_found.values().filter(|(shared, _)| *shared > 0).count();
    let in_tiles = where_found.values().filter(|(_, tiles)| *tiles > 0).count();
    let only_shared = where_found.values().filter(|(shared, tiles)| *shared > 0 && *tiles == 0).count();
    eprintln!(
        "{} textures are replaced, {} are in a section of the world: {in_shared} in a shared section, {in_tiles} in a tile, {only_shared} only in a shared section",
        wanted.len(),
        where_found.len()
    );
    assert!(!where_found.is_empty(), "no replaced texture is in the world: the hashes do not match the stream");
}
