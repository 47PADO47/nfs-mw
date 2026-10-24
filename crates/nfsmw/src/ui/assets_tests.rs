use blackbox_feng::{NodeKind, UiNode, UiTree};
use blackbox_tpk::{AlphaUsage, PixelFormat};
use glam::{Mat4, Quat, Vec3};

use super::*;
use crate::gui::UiOutput;
use crate::ui::present::{BlackboxPresenter, Screen};

#[test]
#[ignore = "requires an unmodified base PC install; set NFSMW_GAME_DIR"]
fn stock_selection_glow_has_alpha_coverage_despite_white_rgb() {
    let path = std::env::var("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR to an unmodified base PC install");
    let dir = GameDir::open(path).unwrap();
    let assets = UiAssets::load(&dir).unwrap();
    let t = assets.texture(fe_hash_upper("IconSelection_Glow")).unwrap();
    assert_eq!(t.alpha_blend, 2);
    let image = assets.image(t.name_hash).unwrap();
    assert_eq!(&image.rgba[..4], &[255, 255, 255, 0]);
    assert!(image.rgba.as_chunks::<4>().0.iter().any(|p| p[3] > 0));
}

#[test]
fn rival_pictures_load_from_rivals_bin() {
    let Some(path) = std::env::var_os("NFSMW_GAME_DIR") else { return };
    let dir = GameDir::open(path).unwrap();
    let assets = UiAssets::load(&dir).unwrap();
    let picture = assets.texture(fe_hash_upper("RIVAL_04_BG")).expect("RIVAL_04_BG from GLOBAL/RIVALS.BIN");
    assert_eq!((picture.width, picture.height), (512, 512));
}

#[test]
#[ignore = "needs the installed game"]
fn installed_minimap_roads_preserve_additive_colour_for_hdr_ui() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let assets = UiAssets::load(&dir).unwrap();
    let name = blackbox_minimap::tile_name(nfsmw_data::minimap::FULL_MAP, 0);
    let hash = fe_hash_upper(&name);
    assert_eq!(assets.texture(hash).expect("installed minimap tile").alpha_blend, 2);
    let mut tile = node(0, None);
    tile.kind = NodeKind::Image {
        texture: hash,
        uv: [0.0, 0.0, 1.0, 1.0],
        mask: None,
        mask_rotation: [0.5, 0.5, 0.0],
        mask_uv: [0.0, 0.0, 1.0, 1.0],
    };
    let tree = UiTree { nodes: vec![tile], draw_order: vec![0] };
    let screen = Screen { width: 640.0, height: 480.0, pixels_per_point: 1.0 };
    let mut out = UiOutput::default();
    BlackboxPresenter::default().present(&tree, &assets, screen, &mut out);
    let patch = out.patches.first().expect("minimap texture upload");
    assert!(patch.rgba.as_chunks::<4>().0.iter().all(|pixel| pixel[3] == 0));
    assert!(patch.rgba.as_chunks::<4>().0.iter().any(|pixel| pixel[..3].iter().any(|v| *v > 0)));
}

fn texture(hash: u32, pixels: [[u8; 4]; 3], blend: u8) -> Texture {
    Texture {
        name: String::new(),
        name_hash: hash,
        width: 3,
        height: 1,
        mip_levels: 1,
        format: PixelFormat::Argb8888,
        compression_type: 32,
        alpha_usage: AlphaUsage::None,
        alpha_sorting: true,
        alpha_blend: blend,
        flags: 0,
        data: pixels.into_iter().flat_map(|[r, g, b, a]| [b, g, r, a]).collect(),
        palette: Vec::new(),
    }
}

fn node(index: usize, mask: Option<u32>) -> UiNode {
    UiNode {
        index,
        guid: index as u32 + 1,
        name_hash: 0,
        parent: None,
        kind: NodeKind::Image {
            texture: 1,
            uv: [0.0, 0.0, 1.0, 1.0],
            mask,
            mask_rotation: [0.5, 0.5, 0.0],
            mask_uv: [0.0, 0.0, 1.0, 1.0],
        },
        text: None,
        local_position: Vec3::ZERO,
        position: Vec3::ZERO,
        local_pivot: Vec3::ZERO,
        local_rotation: Quat::IDENTITY,
        size: Vec3::ONE,
        colour: [255; 4],
        world_colour: [255; 4],
        world: Mat4::IDENTITY,
        z: 1.0,
        visible: true,
        clip: None,
    }
}

#[test]
fn masked_and_unmasked_presenter_uploads_apply_alpha_coverage_once() {
    for blend in [1, 2] {
        let picture = texture(1, [[240, 120, 60, 128], [240, 120, 60, 255], [240, 120, 60, 255]], blend);
        let mask = texture(2, [[255, 255, 255, 128], [255, 255, 255, 0], [255; 4]], 1);
        let assets = UiAssets {
            fonts: HashMap::new(),
            textures: HashMap::from([(1, picture), (2, mask)]),
            aliases: HashMap::new(),
            font_native: HashMap::new(),
            custom: CustomFontSet::default(),
            late: LateTextures::default(),
            input_atlas: Image { width: 0, height: 0, rgba: Vec::new(), blend: 1 },
            discord_mark: Image { width: 0, height: 0, rgba: Vec::new(), blend: 1 },
            mod_mark: Image { width: 0, height: 0, rgba: Vec::new(), blend: 1 },
            language_fonts: HashMap::new(),
        };
        let tree = UiTree { nodes: vec![node(0, None), node(1, Some(2))], draw_order: vec![0, 1] };
        let screen = Screen { width: 640.0, height: 480.0, pixels_per_point: 1.0 };
        let mut presenter = BlackboxPresenter::default();
        let mut out = UiOutput::default();
        presenter.present(&tree, &assets, screen, &mut out);
        let alpha = |coverage| match blend {
            2 => 0,
            _ => coverage,
        };
        assert_eq!(out.patches.len(), 3);
        assert_eq!(out.patches[0].rgba, [120, 60, 30, alpha(128), 240, 120, 60, alpha(255), 240, 120, 60, alpha(255)]);
        // Masked pictures and masks stay straight; the GPU composes and premultiplies once.
        assert_eq!(out.patches[1].rgba, [240, 120, 60, 128, 240, 120, 60, 255, 240, 120, 60, 255]);
        assert_eq!(out.patches[2].rgba, [255, 255, 255, 128, 255, 255, 255, 0, 255, 255, 255, 255]);
        let layer = out.layer.as_ref().unwrap();
        let command = layer.compositions[0];
        assert_eq!(command.picture, out.patches[1].id);
        assert_eq!(command.mask, out.patches[2].id);
        assert_eq!(command.additive, blend == 2);
        let meshes = &layer.meshes;
        assert_eq!(meshes.len(), 2);
        assert_eq!(meshes[0].texture, out.patches[0].id);
        assert_eq!(meshes[1].texture, command.output);
        assert!(meshes.iter().flat_map(|m| &m.vertices).all(|v| v.color_rgba == [255, 255, 255, alpha(255)]));
        let mut cached = UiOutput::default();
        presenter.present(&tree, &assets, screen, &mut cached);
        assert!(cached.patches.is_empty(), "the same composed texture must not be uploaded every frame");
        assert!(cached.layer.as_ref().unwrap().compositions.is_empty());
        let mut moving = tree.clone();
        if let NodeKind::Image { mask_uv, .. } = &mut moving.nodes[1].kind {
            *mask_uv = [0.25, -0.5, 1.25, 0.5];
        }
        let mut changed = UiOutput::default();
        presenter.present(&moving, &assets, screen, &mut changed);
        assert!(changed.patches.is_empty(), "moving masks must not rebuild or upload CPU pixels");
        assert_eq!(changed.layer.unwrap().compositions[0].output, command.output);
    }
}

#[test]
fn the_table_beats_an_install_copy_that_a_mod_replaced_and_the_install_fills_the_gap() {
    // A modded install holds a 2048 x 2048 FONT_MW_BODY where the stock texture is 256 x 256.
    assert_eq!(native_size(Some((2048, 2048)), Some((256, 256))), Some((256, 256)));
    assert_eq!(native_size(None, Some((128, 64))), Some((128, 64)));
    assert_eq!(native_size(Some((64, 64)), None), Some((64, 64)));
    assert_eq!(native_size(None, None), None);
}

#[test]
fn the_table_is_found_by_font_name_in_any_case() {
    assert_eq!(table_size(fe_hash_upper("FONT_MW_BODY")), Some((256, 256)));
    assert_eq!(table_size(fe_hash_upper("font_mw_body")), Some((256, 256)));
    assert_eq!(table_size(fe_hash_upper("not_a_font")), None);
}

#[test]
#[ignore = "requires an unmodified base PC install; set NFSMW_GAME_DIR"]
fn the_verified_table_rows_match_the_install_and_stock_fonts_keep_their_size() {
    let path = std::env::var("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR to an unmodified base PC install");
    let dir = GameDir::open(path).unwrap();
    let assets = UiAssets::load(&dir).unwrap();
    for (name, size) in
        [("FONT_MW_BODY", (256, 256)), ("FONT_MW_TITLE", (512, 512)), ("FONT_MW_CUSTOM_NUMBERS", (128, 64))]
    {
        let hash = fe_hash_upper(name);
        assert_eq!(assets.font_native.get(&hash), Some(&size), "{name}: the install's stock size");
        assert_eq!(
            assets.texture_size(hash),
            assets.texture(hash).map(|t| (t.width, t.height)),
            "{name}: stock output"
        );
    }
}

#[test]
fn repeating_images_keep_their_extent_and_cache_a_separate_wrapped_texture() {
    let assets = UiAssets {
        fonts: HashMap::new(),
        textures: HashMap::from([(1, texture(1, [[255; 4], [128; 4], [64; 4]], 1))]),
        aliases: HashMap::new(),
        font_native: HashMap::new(),
        custom: CustomFontSet::default(),
        late: LateTextures::default(),
        input_atlas: Image { width: 0, height: 0, rgba: Vec::new(), blend: 1 },
        discord_mark: Image { width: 0, height: 0, rgba: Vec::new(), blend: 1 },
        mod_mark: Image { width: 0, height: 0, rgba: Vec::new(), blend: 1 },
        language_fonts: HashMap::new(),
    };
    let mut repeating = node(0, None);
    let NodeKind::Image { uv, .. } = &mut repeating.kind else { unreachable!() };
    *uv = [0.0, 0.0, 2.5, 1.5];
    let tree = UiTree { nodes: vec![repeating, node(1, None)], draw_order: vec![0, 1] };
    let screen = Screen { width: 640.0, height: 480.0, pixels_per_point: 1.0 };
    let mut presenter = BlackboxPresenter::default();
    let mut out = UiOutput::default();
    presenter.present(&tree, &assets, screen, &mut out);
    assert_eq!(out.patches.len(), 2);
    assert_ne!(out.patches[0].id, out.patches[1].id);
    assert_eq!(out.patches[0].size, [5, 3], "one wrapped texel on each side");
    assert_eq!(out.patches[1].size, [3, 1], "ordinary atlas sampling stays unchanged");
    let meshes = &out.layer.as_ref().unwrap().meshes;
    assert_eq!(meshes.len(), 2);
    assert_eq!(meshes[0].vertices.len(), 24, "six fractional tiles");
    assert_eq!(meshes[1].vertices.len(), 4);
    for (axis, centre) in [(0, 320.0), (1, 240.0)] {
        let lo = meshes[0].vertices.iter().map(|v| v.position[axis]).reduce(f32::min).unwrap();
        let hi = meshes[0].vertices.iter().map(|v| v.position[axis]).reduce(f32::max).unwrap();
        assert_eq!([lo, hi], [centre - 0.5, centre + 0.5]);
    }
    let mut cached = UiOutput::default();
    presenter.present(&tree, &assets, screen, &mut cached);
    assert!(cached.patches.is_empty());
}

#[test]
#[ignore = "requires installed PC menu packages; set NFSMW_GAME_DIR"]
fn dialog_and_main_menu_stripes_author_repeating_uvs() {
    let path = std::env::var("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR");
    let dir = GameDir::open(path).unwrap();
    let catalog = crate::ui::Catalog::load(&dir, &crate::ui::SCREEN_FILES);
    for (name, guid, expected) in [
        ("Dialog.fng", 0x000f_70a9, [0.0, 0.0, 16.8125, 6.1875]),
        ("MainMenu.fng", 0x000e_a550, [0.0, 0.0, 8.90625, 1.0]),
    ] {
        let package = catalog.find(name).unwrap();
        let object = package.objects.iter().find(|o| o.guid == guid).unwrap();
        assert_eq!(object.data.uv(), expected, "{name}");
        let resource = &package.resources[object.resource.unwrap()];
        assert_eq!(resource.handle, fe_hash_upper("BASESTRIPE"));
    }
}

#[test]
fn every_loading_tip_has_its_picture_and_text() {
    let Some(path) = std::env::var_os("NFSMW_GAME_DIR") else { return };
    let dir = GameDir::open(path).unwrap();
    let assets = UiAssets::load(&dir).unwrap();
    let strings = nfsmw_data::languages::table(&dir, nfsmw_data::languages::ENGLISH_FILE).expect("the English table");
    for key in crate::ui::tips::TIPS {
        let picture = fe_hash_upper(&crate::ui::tips::picture_name(key));
        assert!(assets.texture(picture).is_some(), "TIP_{key} from GLOBAL/DYNTEX.BIN");
        for label in [crate::ui::tips::header_label(key), crate::ui::tips::description_label(key)] {
            assert!(strings.get(blackbox_text::label_hash(&label)).is_some(), "{label} in the language table");
        }
    }
}

#[test]
fn late_textures_are_found_after_the_stock_ones_and_can_be_dropped() {
    let mut assets = UiAssets::empty();
    let stock = texture(1, [[1, 2, 3, 255]; 3], 1);
    assets.textures.insert(1, stock);
    assert!(assets.texture(2).is_none());
    assets.add_textures([texture(1, [[9, 9, 9, 255]; 3], 2), texture(2, [[4, 5, 6, 255]; 3], 2)]);
    assert_eq!(assets.late_count(), 2);
    assert_eq!(assets.texture(2).map(|t| t.alpha_blend), Some(2), "a late texture is found");
    assert_eq!(assets.resolve(2), 2);
    assert_eq!(assets.texture(1).map(|t| t.alpha_blend), Some(1), "the stock texture of the same name wins");
    assert!(assets.image(2).is_some(), "a late texture decodes like a stock one");
    assets.remove_textures(&[1, 2]);
    assert_eq!(assets.late_count(), 0);
    assert!(assets.texture(2).is_none());
    assert!(assets.texture(1).is_some(), "the stock texture stays");
}

#[test]
#[ignore = "needs the installed game"]
fn installed_photo_backdrops_are_in_the_ui_assets_with_a_transparent_window() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let assets = UiAssets::load(&dir).unwrap();
    for name in ["PHOTOFINISH_SPRINT", "PHOTOFINISH_RIVAL", "PHOTOFINISH_DRAG", "PHOTOFINISH_TOOBOOTH"] {
        let texture = assets.texture(fe_hash_upper(name)).unwrap_or_else(|| panic!("{name} is in the assets"));
        assert_eq!((texture.width, texture.height), (1024, 512), "{name}");
        let image = assets.image(fe_hash_upper(name)).expect("decodes");
        // The polaroid's picture is cut out of the backdrop: a pixel in the middle of it is clear, one in a corner is not.
        let alpha = |x: usize, y: usize| image.rgba[(y * image.width as usize + x) * 4 + 3];
        assert_eq!(alpha(500, 250), 0, "{name}: the window is transparent");
        assert_eq!(alpha(10, 10), 255, "{name}: the backdrop is opaque");
    }
}
