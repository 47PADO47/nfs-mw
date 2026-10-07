use crate::{install, unwrapped};

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn texture_packs_decode() {
    let Some(dir) = install() else { return };
    let files = ["CARS/BMWM3GTR/TEXTURES.BIN", "CARS/TEXTURES.BIN", "GLOBAL/GLOBALB.LZC", "FRONTEND/FRONTB.LZC"];
    for rel in files {
        let packs = blackbox_tpk::read_texture_packs(&unwrapped(&dir, rel)).unwrap_or_else(|e| panic!("{rel}: {e}"));
        assert!(!packs.is_empty(), "{rel}: no texture packs");
        for pack in &packs {
            assert_eq!(pack.version, 5, "{rel}");
            for t in &pack.textures {
                // Names are stored in 24 bytes, so only untruncated names can be checked.
                if t.name.len() < 23 {
                    assert_eq!(t.name_hash, blackbox_hash::bstring_hash(&t.name), "{rel}: {}", t.name);
                }
                assert!(t.mip(0).is_some(), "{rel}: {}: level 0 missing", t.name);
            }
            assert!(pack.failed.is_empty(), "{rel} / {}: {:?}", pack.name, pack.failed);
        }
        let total: usize = packs.iter().map(|p| p.textures.len()).sum();
        eprintln!("{rel}: {} packs, {total} textures", packs.len());
    }
}
