//! Shared references must retain authored pixels/blending instead of opaque white fallback.
use super::*;
use blackbox_gfx::BlendMode;

#[test]
#[ignore = "requires installed stage/global/front-end packs; set NFSMW_GAME_DIR"]
fn stage_shared_materials_resolve_transparent_sky_and_grey_on_both_platforms() {
    let dir = GameDir::open(std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR").unwrap())).unwrap();
    for stage in [Stage::Crib, Stage::SafeHouse] {
        let data = RoomData::read(&dir, stage).unwrap();
        let textures: HashMap<_, _> =
            data.packs.iter().flat_map(|pack| &pack.textures).map(|texture| (texture.name_hash, texture)).collect();
        for solid in &data.solids {
            for group in &solid.groups {
                let Some(hash) = group.diffuse_texture(solid) else { continue };
                if super::super::room_materials::KNOWN_UNRESOLVED.contains(&hash) {
                    continue;
                }
                assert!(textures.contains_key(&hash), "{stage:?} {} references unresolved {hash:#x}", solid.name);
            }
        }
        if let Some(transparent) = textures.get(&0x6699_4F4A) {
            assert_eq!(transparent.name, "DEFAULTALPHA");
            assert_eq!(blend_mode(Some(transparent)), BlendMode::Additive);
            let pixels = blackbox_tpk::decode_mip_rgba8(transparent, 0).unwrap();
            assert!(
                pixels.as_chunks::<4>().0.iter().all(|p| p[3] == 0),
                "placeholder geometry cannot occlude the room"
            );
        }
        let grey = textures.get(&0x0016_3C76).unwrap();
        assert_eq!(grey.name, "GREY");
        assert_eq!(blend_mode(Some(grey)), BlendMode::Opaque);
    }
}
#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_car_lot_draws_the_room_without_emitter_proxy_meshes() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
    let stage = super::Stage::CarLot;
    let file = dir.read(stage.file()).unwrap();
    let all = blackbox_solid::read_solids(&file).unwrap();
    let room = super::RoomData::read(&dir, stage).unwrap();
    assert_eq!(all.len() - room.solids.len(), 4);
    assert_eq!(
        room.solids.iter().map(|solid| solid.name.as_str()).collect::<Vec<_>>(),
        ["BACKDROP_CAST_SHADOW_MAP", "BACKDROP_CURRGEN", "BACKDROP_CRIB", "OBJECT01"]
    );
    assert!(!room.placements().is_empty(), "the car-lot particles still have authored placements");
}
