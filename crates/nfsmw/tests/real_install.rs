//! Tests against the user's own install. They need game data, so they are
//! `#[ignore]`d; run them with
//!
//! ```sh
//! NFSMW_GAME_DIR="D:/Need For Speed Most Wanted Black Edition" cargo test -p nfsmw -- --ignored
//! ```
//!
//! Without `NFSMW_GAME_DIR` they pass without checking anything.

use nfsmw_install::GameDir;

fn install() -> Option<GameDir> {
    let dir = std::env::var_os(nfsmw_install::ENV_VAR)?;
    Some(GameDir::open(std::path::PathBuf::from(dir)).expect("NFSMW_GAME_DIR is set but cannot be indexed"))
}

fn unwrapped(dir: &GameDir, rel: &str) -> Vec<u8> {
    let raw = dir.read(rel).unwrap_or_else(|e| panic!("{rel}: {e}"));
    nfsmw_compress::unwrap(&raw).unwrap_or_else(|e| panic!("{rel}: {e}")).into_owned()
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn install_validates() {
    let Some(dir) = install() else { return };
    let v = dir.validate();
    assert!(v.is_usable(), "missing: {:?}", v.missing);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn every_car_geometry_parses() {
    let Some(dir) = install() else { return };
    let mut cars = 0;
    let mut solids = 0;
    for car in dir.subdirectories("CARS") {
        let rel = format!("CARS/{car}/GEOMETRY.BIN");
        if !dir.exists(&rel) {
            continue;
        }
        let data = unwrapped(&dir, &rel);
        let parsed = nfsmw_geometry::read_solids(&data).unwrap_or_else(|e| panic!("{rel}: {e}"));
        assert!(!parsed.is_empty(), "{rel}: no solids");
        for s in &parsed {
            assert_eq!(s.name_hash, nfsmw_hash::bstring_hash(&s.name), "{rel}: {}", s.name);
            for g in &s.groups {
                let range = g.first_index as usize..(g.first_index + g.num_indices) as usize;
                assert!(
                    s.indices[range].iter().all(|&i| usize::from(i) < s.vertices.len()),
                    "{rel}: {}: index out of range",
                    s.name
                );
            }
        }
        cars += 1;
        solids += parsed.len();
    }
    eprintln!("{cars} cars, {solids} solids");
    assert!(cars >= 50);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn bmw_m3_gtr_matches_documented_numbers() {
    let Some(dir) = install() else { return };
    let data = unwrapped(&dir, "CARS/BMWM3GTR/GEOMETRY.BIN");
    let solids = nfsmw_geometry::read_solids(&data).unwrap();
    // docs/formats/models.md: 97 solids, all with 36-byte vertices.
    assert_eq!(solids.len(), 97);
    assert!(solids.iter().all(|s| s.vertex_stride == 36));
    let base = solids.iter().find(|s| s.name == "BMWM3GTR_BASE_A").unwrap();
    assert_eq!(base.groups.len(), 7);
    assert_eq!(base.vertices.len(), 345);
    assert_eq!(base.indices.len(), 1062);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn texture_packs_decode() {
    let Some(dir) = install() else { return };
    let files = ["CARS/BMWM3GTR/TEXTURES.BIN", "CARS/TEXTURES.BIN", "GLOBAL/GLOBALB.LZC", "FRONTEND/FRONTB.LZC"];
    for rel in files {
        let data = unwrapped(&dir, rel);
        let packs = nfsmw_texture::read_texture_packs(&data).unwrap_or_else(|e| panic!("{rel}: {e}"));
        assert!(!packs.is_empty(), "{rel}: no texture packs");
        for pack in &packs {
            for t in &pack.textures {
                // Names are stored in 24 bytes, so only untruncated names can be checked.
                if t.name.len() < 23 {
                    assert_eq!(t.name_hash, nfsmw_hash::bstring_hash(&t.name), "{rel}: {}", t.name);
                }
                let level0 = nfsmw_texture::mip_level_size(t.format, t.width, t.height, 0);
                assert!(t.data.len() >= level0, "{rel}: {}: {} < {level0}", t.name, t.data.len());
            }
            let failed: Vec<_> = pack.failed.iter().filter(|(_, why)| !why.contains("not implemented")).collect();
            assert!(failed.is_empty(), "{rel} / {}: {failed:?}", pack.name);
        }
        let total: usize = packs.iter().map(|p| p.textures.len()).sum();
        eprintln!("{rel}: {} packs, {total} textures", packs.len());
    }
}
