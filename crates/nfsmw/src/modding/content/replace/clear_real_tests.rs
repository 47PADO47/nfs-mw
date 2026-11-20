//! What keeping the copies off the objects next to them does to the installed 2005 world: how many replacements were made smaller. Needs the
//! 2005 install and the folder of the 2012 mod (`NFSMW_SIGNS_MOD`, its `replace/` and `files/`). Not part of the normal run.

use std::path::PathBuf;
use std::sync::Arc;

use blackbox_modding::content::parse_replace_set;
use game_install::GameDir;
use nfsmw_data::world::{DEFAULT_TRACK, WorldIndex, parse_section};

use super::active::{Active, LoadedSet};

fn env_path(name: &str) -> PathBuf {
    PathBuf::from(std::env::var_os(name).unwrap_or_else(|| panic!("set {name}")))
}

/// The bushes set of the mod, with the clearance the mod's manifest asks for (an older conversion's manifest does not: it is forced on here).
fn bushes(mod_dir: &std::path::Path) -> Active {
    let path = mod_dir.join("replace").join("bushes-mw2012.toml");
    let mut set = parse_replace_set("bushes-mw2012", &std::fs::read_to_string(path).unwrap()).unwrap();
    set.limits.clearance = true;
    let set = Arc::new(set);
    let geometry = std::fs::read(mod_dir.join("files").join(set.geometry.as_ref().unwrap())).unwrap();
    let models = blackbox_solid::read_solids(&geometry).unwrap();
    let pack = std::fs::read(mod_dir.join("files").join(set.textures.as_ref().unwrap())).unwrap();
    let added: Vec<_> =
        blackbox_tpk::read_texture_packs(&pack).unwrap().into_iter().flat_map(|pack| pack.textures).collect();
    Active::build(vec![LoadedSet { owner: "test".into(), order: 0, set, models, added, replaced: Vec::new() }])
}

/// Every section of the world is rewritten with the bushes set (the shared ones first: their rails are the tiles' neighbours) and the
/// tally of what clearance did is printed: how many copies were looked at, made smaller, held at the old width and left out.
#[test]
#[ignore = "needs the game (NFSMW_GAME_DIR) and NFSMW_SIGNS_MOD"]
fn report_how_many_bushes_are_made_smaller_to_clear_their_neighbours() {
    let dir = GameDir::open(env_path("NFSMW_GAME_DIR")).unwrap();
    let active = bushes(&env_path("NFSMW_SIGNS_MOD"));
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let shared: Vec<usize> = index.shared().collect();
    let others = (0..index.sections.len()).filter(|i| !shared.contains(i));
    let mut sections = 0;
    for i in shared.iter().copied().chain(others) {
        let section = &index.sections[i];
        let bytes = dir.read_range(&index.stream_file, u64::from(section.file_offset), section.size as usize).unwrap();
        let mut data = parse_section(i, &bytes).unwrap();
        active.apply(&mut data);
        sections += 1;
    }
    let tally = active.clearance();
    eprintln!(
        "bushes: {sections} sections, {} copies looked at near an object: {} made smaller, {} of them held at the old width, {} left out",
        tally.copies, tally.shrunk, tally.held, tally.skipped
    );
    assert!(
        tally.copies > 0,
        "no copy of the bushes had a wall, rail, barrier or building near it: the objects are not found"
    );
}
