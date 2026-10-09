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
            input_atlas: Image { width: 0, height: 0, rgba: Vec::new(), blend: 1 },
            strings: None,
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
        assert_eq!(out.patches.len(), 2);
        assert_eq!(out.patches[0].rgba, [120, 60, 30, alpha(128), 240, 120, 60, alpha(255), 240, 120, 60, alpha(255)]);
        assert_eq!(out.patches[1].rgba, [60, 30, 15, alpha(64), 0, 0, 0, 0, 240, 120, 60, alpha(255)]);
        let meshes = &out.layer.as_ref().unwrap().meshes;
        assert_eq!(meshes.len(), 2);
        assert_eq!(meshes[0].texture, out.patches[0].id);
        assert_eq!(meshes[1].texture, out.patches[1].id);
        assert!(meshes.iter().flat_map(|m| &m.vertices).all(|v| v.color_rgba == [255, 255, 255, alpha(255)]));
        let mut cached = UiOutput::default();
        presenter.present(&tree, &assets, screen, &mut cached);
        assert!(cached.patches.is_empty(), "the same composed texture must not be uploaded every frame");
    }
}
