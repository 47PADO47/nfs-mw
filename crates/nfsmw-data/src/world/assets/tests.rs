use super::*;

#[test]
fn selected_textures_include_frames_from_all_sections_without_unrelated_animations() {
    let anim = |hash, frames: Vec<u32>| TextureAnim {
        name: format!("animation {hash}"),
        name_hash: hash,
        frames_per_second: 10,
        frames,
    };
    // Definitions can precede the selected model and can share frames. Animation
    // dependencies follow the full loader's direct references, not animation chains.
    let anims = [anim(20, vec![21, 22]), anim(10, vec![11, 20]), anim(30, vec![31])];
    assert_eq!(selected::texture_hashes([10, 40, 10].into_iter(), &anims), HashSet::from([10, 11, 20, 40]));
    assert_eq!(selected::texture_hashes([10, 20].into_iter(), &anims), HashSet::from([10, 11, 20, 21, 22]));
    assert!(selected::texture_hashes(std::iter::empty(), &anims).is_empty());
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_selected_roadblock_assets_match_full_shared_load() {
    let path = std::env::var_os(crate::game::SPEC.env_var).expect("set NFSMW_GAME_DIR");
    let dir = GameDir::open(std::path::PathBuf::from(path)).unwrap();
    let names = ["XO_SAWHORSE_1B_00", "XO_SPIKEBELT_1B_DW_00"];
    let hashes = names.map(blackbox_hash::bstring_hash);
    let selected = SharedAssets::load_selected(&dir, super::super::DEFAULT_TRACK, &hashes).unwrap();
    assert_eq!(selected.names().len(), 2);
    let full = SharedAssets::load(&dir, super::super::DEFAULT_TRACK).unwrap();
    assert!(full.names().len() > selected.names().len());
    let bytes = |assets: &SharedAssets| -> usize {
        assets.assets.iter().flat_map(|asset| &asset.textures).map(|texture| texture.data.len()).sum()
    };
    eprintln!(
        "shared assets: selected {} models / {} texture bytes; full {} models / {} texture bytes",
        selected.assets.len(),
        bytes(&selected),
        full.assets.len(),
        bytes(&full)
    );
    assert!(bytes(&selected) < bytes(&full), "unrelated per-model texture copies are absent");
    for name in names {
        let a = selected.model(selected.find(name).expect("selected model"));
        let b = full.model(full.find(name).expect("full model"));
        assert_eq!(a.solids, b.solids);
        assert_eq!(a.textures.len(), b.textures.len());
        for (hash, texture) in &a.textures {
            assert_eq!(texture, &b.textures[hash]);
        }
        assert_eq!(selected.anims(selected.find(name).unwrap()), full.anims(full.find(name).unwrap()));
    }
}
