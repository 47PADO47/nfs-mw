use super::source::{hashes, mappings};
use super::*;

use blackbox_tpk::{AlphaUsage, PixelFormat};

/// A fresh folder under the temp folder, for one test.
fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nfsmw-texture-packs-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A square DXT1 DDS file with a full mip chain, every byte `fill`.
fn dds(side: u32, fill: u8) -> Vec<u8> {
    let mut levels = 0u32;
    let mut body = 0;
    let mut size = side;
    while size >= 1 {
        body += (size.div_ceil(4) * size.div_ceil(4) * 8) as usize;
        levels += 1;
        if size == 1 {
            break;
        }
        size /= 2;
    }
    let mut out = vec![0u8; 128];
    out[..4].copy_from_slice(b"DDS ");
    out[4..8].copy_from_slice(&124u32.to_le_bytes());
    out[12..16].copy_from_slice(&side.to_le_bytes());
    out[16..20].copy_from_slice(&side.to_le_bytes());
    out[28..32].copy_from_slice(&levels.to_le_bytes());
    out[80..84].copy_from_slice(&4u32.to_le_bytes());
    out[84..88].copy_from_slice(b"DXT1");
    out.extend(std::iter::repeat_n(fill, body));
    out
}

fn texture(name: &str) -> Texture {
    Texture {
        name: name.to_owned(),
        name_hash: blackbox_hash::bstring_hash(name),
        width: 4,
        height: 4,
        mip_levels: 1,
        format: PixelFormat::Dxt1,
        compression_type: 0x22,
        alpha_usage: AlphaUsage::None,
        alpha_sorting: false,
        alpha_blend: 0,
        flags: 0,
        tilable_uv: 0,
        data: vec![0; 8],
        palette: Vec::new(),
    }
}

fn spec(id: &str, root: &Path, priority: i32) -> SourceSpec {
    SourceSpec { id: id.to_owned(), root: root.to_owned(), priority }
}

#[test]
fn a_game_id_is_a_hash_or_a_name() {
    assert_eq!(hashes("0x17c498c9"), [0x17C4_98C9]);
    let name = blackbox_hash::bstring_hash("ARC_BROWNBRICKWALL");
    assert!(hashes("arc_brownbrickwall").contains(&name));
}

#[test]
fn a_pack_info_maps_ids_to_files_under_its_root() {
    let dir = folder("info");
    std::fs::create_dir_all(dir.join("RESOURCES")).unwrap();
    let info = dir.join("TexturePackInfo.json");
    let text = r#"{ "rootDirectory": "RESOURCES", "textureMappings": [
        { "gameId": "0x2b3f2355", "texturePath": "a.dds" },
        { "gameId": "ROAD", "texturePath": "sub\\b.dds" }, ] }"#;
    std::fs::write(&info, text).unwrap();
    let found = mappings(&info);
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(found[0], ("0x2b3f2355".to_owned(), dir.join("RESOURCES/a.dds")));
    assert_eq!(found[1], ("ROAD".to_owned(), dir.join("RESOURCES/sub/b.dds")));
}

#[test]
fn the_highest_source_wins_and_a_higher_mod_is_never_undone() {
    let dir = folder("priority");
    let (low, high, pack) = (dir.join("low/textures"), dir.join("high/textures"), dir.join("pack"));
    for (folder, fill) in [(&low, 1), (&high, 2), (&pack, 3)] {
        std::fs::create_dir_all(folder).unwrap();
        std::fs::write(folder.join("ROAD.dds"), dds(8, fill)).unwrap();
    }
    std::fs::write(pack.join("WALL.dds"), dds(8, 3)).unwrap();
    let specs = vec![spec("settings", &pack, SETTINGS_PRIORITY), spec("high", &high, 50), spec("low", &low, 10)];
    let mods =
        vec![ModFolder { dir: dir.join("low"), priority: 10 }, ModFolder { dir: dir.join("high"), priority: 50 }];
    let packs = TexturePacks::new(specs, mods, 0);
    assert_eq!((packs.len(), packs.shadowed()), (2, 2));

    // A file of the install: the highest source of each texture.
    let mut install = [texture("ROAD"), texture("WALL"), texture("SKY")];
    assert_eq!(packs.apply_above(packs.floor(&dir.join("install/STREAML2RA.BUN")), &mut install), 2);
    assert_eq!((install[0].width, install[0].data[0]), (8, 2));
    assert_eq!(install[1].data[0], 3);
    assert_eq!(install[2].width, 4);

    // A file the low mod serves: the high mod still replaces it, the settings' folder does not.
    let mut served = [texture("ROAD"), texture("WALL")];
    assert_eq!(packs.floor(&dir.join("low/files/TRACKS/STREAML2RA.BUN")), Some(10));
    assert_eq!(packs.apply_above(Some(10), &mut served), 1);
    assert_eq!((served[0].data[0], served[1].width), (2, 4));
    assert_eq!(packs.held_back(), 1);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_size_cap_keeps_the_stored_levels_that_fit() {
    let dir = folder("cap");
    std::fs::write(dir.join("ROAD.dds"), dds(64, 7)).unwrap();
    let packs = TexturePacks::new(vec![spec("settings", &dir, SETTINGS_PRIORITY)], Vec::new(), 16);
    let mut textures = [texture("ROAD")];
    assert_eq!(packs.apply_above(None, &mut textures), 1);
    assert_eq!((textures[0].width, textures[0].height, textures[0].mip_levels), (16, 16, 5));
    assert_eq!(packs.sources()[0].replaced(), 1);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_mod_folder_matches_as_given_and_as_resolved() {
    let dir = folder("resolved");
    std::fs::create_dir_all(dir.join("mod/files")).unwrap();
    // A roundabout spelling of the folder: the install's layers hand out the resolved one.
    let given = dir.join("mod").join("files").join("..");
    let packs = TexturePacks::new(Vec::new(), vec![ModFolder { dir: given.clone(), priority: 7 }], 0);
    let resolved = dir.join("mod").canonicalize().unwrap();
    assert_eq!(packs.floor(&resolved.join("files/TRACKS/STREAML2RA.BUN")), Some(7));
    assert_eq!(packs.floor(&given.join("SCENERY/A.BUN")), Some(7));
    assert_eq!(packs.floor(&dir.join("other/STREAML2RA.BUN")), None);
    std::fs::remove_dir_all(&dir).unwrap();
}
