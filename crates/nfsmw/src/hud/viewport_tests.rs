//! Numeric reference placement, rotating map coherence and install-authored wide positions.

use blackbox_feng::package::{MultiDef, ObjectData, ObjectDef, ObjectKind, Package};
use blackbox_feng::{Runtime, UiTree, fe_hash_upper};
use blackbox_minimap::{Calibration, Orientation};
use glam::Vec3;

use super::bind::HudBinding;
use super::minimap::MinimapBinding;
use super::state::{HudState, MapPosition};
use super::viewport::HudViewport;
use crate::settings::HudLayout;
use crate::ui::present::Screen;

const MAP: Calibration = Calibration { origin: [0.0; 2], width: 8000.0 };

fn object(kind: ObjectKind, guid: u32, hash: u32, parent: Option<u32>, at: [f32; 2]) -> ObjectDef {
    let mut words = vec![0; 36];
    words[..4].fill(255);
    words[7] = at[0].to_bits();
    words[8] = at[1].to_bits();
    words[9] = 100.0f32.to_bits();
    words[13] = 1.0f32.to_bits();
    words[14] = 128.0f32.to_bits();
    words[15] = 128.0f32.to_bits();
    ObjectDef {
        kind,
        guid,
        name_hash: hash,
        flags: 0,
        resource: None,
        parent,
        data: ObjectData { words },
        string: None,
        multi: (kind == ObjectKind::MultiImage)
            .then(|| MultiDef { textures: [fe_hash_upper("MINIMAP_MASK"), 0, 0], ..Default::default() }),
        scripts: Vec::new(),
        responses: Vec::new(),
    }
}

fn package() -> Package {
    let mut objects = vec![
        object(ObjectKind::Group, 1, 0x1603_009e, None, [-221.0, 139.0]),
        object(ObjectKind::Group, 2, fe_hash_upper("TRACKMAPTARGETRING"), Some(1), [0.0; 2]),
        object(ObjectKind::Image, 3, fe_hash_upper("Backing"), Some(2), [0.0; 2]),
        object(ObjectKind::Group, 4, fe_hash_upper("TRACK_MAP"), None, [-221.0, 139.0]),
        object(ObjectKind::Image, 5, fe_hash_upper("PLAYERCARINDICATOR"), None, [-222.0, 139.0]),
        object(ObjectKind::Group, 10, 0x5d01_01f1, None, [221.0, 139.0]),
        object(ObjectKind::Group, 11, fe_hash_upper("GaugeCluster"), Some(10), [0.0; 2]),
        object(ObjectKind::Image, 12, fe_hash_upper("Face"), Some(11), [0.0; 2]),
        object(ObjectKind::Group, 13, 0x87c3_8e97, Some(10), [-10.0, 20.0]),
        object(ObjectKind::MultiImage, 14, 0xedfb_6d37, Some(10), [0.0; 2]),
        object(ObjectKind::Image, 15, fe_hash_upper("Centered"), None, [0.0, -50.0]),
    ];
    for i in 0..4 {
        objects.push(object(
            ObjectKind::MultiImage,
            20 + i,
            fe_hash_upper(&format!("TRACK_MAP{}", i + 1)),
            Some(4),
            [
                match i % 2 {
                    0 => -64.0,
                    _ => 64.0,
                },
                match i {
                    0 | 1 => -64.0,
                    _ => 64.0,
                },
            ],
        ));
    }
    Package {
        name: "HUD.fng".into(),
        file_name: String::new(),
        version: 0x20000,
        resources: Vec::new(),
        objects,
        button_count: 0,
        responses: Vec::new(),
        targets: Vec::new(),
    }
}

fn screen(width: f32, height: f32) -> Screen {
    Screen { width, height, pixels_per_point: 1.0 }
}

fn close(got: f32, expected: f32) {
    assert!((got - expected).abs() < 0.01, "{got} vs {expected}");
}

fn point(tree: &UiTree, name: &str, local: Vec3, s: Screen) -> Vec3 {
    let node = tree.nodes.iter().find(|n| n.name_hash == fe_hash_upper(name)).unwrap();
    let world = node.world.transform_point3(local);
    Vec3::new(s.width * 0.5 + world.x * s.scale(), s.height * 0.5 + world.y * s.scale(), world.z)
}

#[test]
fn xbox_preset_matches_the_supplied_reference_centers_at_4k() {
    let mut rt = Runtime::new();
    let id = rt.load(package());
    let mut tree = rt.tree(id);
    let viewport = HudViewport::new(&tree);
    let s = screen(3840.0, 2160.0);
    viewport.apply(&mut tree, s, HudLayout::Xbox360);
    for (name, x) in [("Backing", 508.26), ("Face", 3331.74)] {
        let center = point(&tree, name, Vec3::ZERO, s);
        close(center.x, x);
        close(center.y, 1655.46);
    }
    let centered = point(&tree, "Centered", Vec3::ZERO, s);
    close(centered.x, 1920.0);
    close(centered.y, 873.0);
}

#[test]
fn pc_wide_layout_moves_all_gauge_descendants_once_without_stretching() {
    let mut rt = Runtime::new();
    let id = rt.load(package());
    let before = rt.tree(id);
    let viewport = HudViewport::new(&before);
    let mut tree = before.clone();
    viewport.apply(&mut tree, screen(1920.0, 1080.0), HudLayout::Pc);
    for (a, b) in before.nodes.iter().zip(&tree.nodes) {
        let shift = match a.guid {
            10..=14 => 120.0,
            15 => 0.0,
            _ => -120.0,
        };
        for corner in [Vec3::ZERO, Vec3::new(-0.5, 0.5, 0.0), Vec3::new(0.5, -0.5, 0.0)] {
            let old = a.world.transform_point3(corner);
            let new = b.world.transform_point3(corner);
            close(new.x, old.x + shift);
            close(new.y, old.y);
            close(new.z, old.z);
        }
        assert_eq!(a.local_position, b.local_position);
        assert_eq!(a.kind, b.kind, "textures and mask parameters remain unchanged");
    }
}

#[test]
fn rotating_and_scrolling_map_stays_registered_with_the_backing_and_arrow() {
    let mut rt = Runtime::new();
    let id = rt.load(package());
    let map = MinimapBinding::new(&rt, id, MAP, "MAP").unwrap();
    let mut binding = HudBinding::new(&mut rt, id).with_minimap(Some(map));
    let viewport = HudViewport::new(&rt.tree(id));
    for layout in [HudLayout::Pc, HudLayout::Xbox360, HudLayout::Classic] {
        for orientation in [Orientation::North, Orientation::Heading] {
            for heading in [[0.0, 1.0], [1.0, 0.0], [-0.6, -0.8]] {
                for at in [[1250.0, 6750.0], [1500.1, 5999.9], [2000.1, 6000.1]] {
                    let state = HudState {
                        minimap: Some(MapPosition { position: at, heading, orientation }),
                        ..Default::default()
                    };
                    binding.apply(&mut rt, &state);
                    let scroll = MinimapBinding::new(&rt, id, MAP, "MAP")
                        .unwrap()
                        .view(&state, &state.minimap.unwrap())
                        .scroll();
                    let mut tree = rt.tree(id);
                    let s = screen(2048.0, 1152.0);
                    viewport.apply(&mut tree, s, layout);
                    let map_center = point(&tree, "TRACK_MAP", Vec3::new(scroll[0], scroll[1], 0.0), s);
                    let backing = point(&tree, "Backing", Vec3::ZERO, s);
                    close(map_center.x, backing.x);
                    close(map_center.y, backing.y);
                    let arrow = point(&tree, "PLAYERCARINDICATOR", Vec3::ZERO, s);
                    let scale = match layout {
                        HudLayout::Xbox360 => 0.92,
                        _ => 1.0,
                    };
                    close(arrow.x, backing.x - s.scale() * scale);
                    close(arrow.y, backing.y);
                }
            }
        }
    }
}

#[test]
fn resizing_and_switching_presets_never_changes_the_runtime_or_accumulates_offsets() {
    let mut rt = Runtime::new();
    let id = rt.load(package());
    let before = rt.tree(id);
    let viewport = HudViewport::new(&before);
    for layout in [HudLayout::Xbox360, HudLayout::Pc, HudLayout::Classic, HudLayout::Xbox360] {
        for s in [screen(1920.0, 1080.0), screen(1440.0, 1080.0), screen(2560.0, 1080.0)] {
            let mut a = rt.tree(id);
            let mut b = rt.tree(id);
            viewport.apply(&mut a, s, layout);
            viewport.apply(&mut b, s, layout);
            for (a, b) in a.nodes.iter().zip(&b.nodes) {
                assert_eq!(a.world, b.world);
            }
            if layout == HudLayout::Classic || s.width / s.height <= 4.0 / 3.0 {
                for (a, original) in a.nodes.iter().zip(&before.nodes) {
                    assert_eq!(a.world, original.world);
                }
            }
        }
    }
    for (a, b) in before.nodes.iter().zip(rt.tree(id).nodes) {
        assert_eq!(a.world, b.world);
    }
}

#[test]
fn intermediate_and_ultrawide_aspects_preserve_round_gauges_and_visible_edges() {
    let mut rt = Runtime::new();
    let id = rt.load(package());
    let viewport = HudViewport::new(&rt.tree(id));
    for layout in [HudLayout::Pc, HudLayout::Xbox360] {
        for width in [1440.0, 1728.0, 1920.0, 2560.0, 3840.0] {
            let s = screen(width, 1080.0);
            let mut tree = rt.tree(id);
            viewport.apply(&mut tree, s, layout);
            for name in ["Backing", "Face"] {
                let center = point(&tree, name, Vec3::ZERO, s);
                let left = point(&tree, name, Vec3::new(-0.5, 0.0, 0.0), s);
                let right = point(&tree, name, Vec3::new(0.5, 0.0, 0.0), s);
                let top = point(&tree, name, Vec3::new(0.0, -0.5, 0.0), s);
                assert!(left.x >= 0.0 && right.x <= width);
                close(right.x - center.x, center.y - top.y);
            }
        }
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn viewport_agrees_with_the_installs_authored_wide_gauge_and_backing_positions() {
    use crate::ui::Catalog;
    use game_install::GameDir;

    let dir = GameDir::open(std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR").unwrap())).unwrap();
    let catalog = Catalog::load(&dir, &["GLOBAL/InGameB.bun", "GLOBAL/INGAMEC.BUN"]);
    let mut rt = Runtime::new();
    let id = rt.load(catalog.find("HUD_SingleRace.fng").unwrap().clone());
    rt.update(1.0);
    let mut projected = rt.tree(id);
    let viewport = HudViewport::new(&projected);
    viewport.apply(&mut projected, screen(1920.0, 1080.0), HudLayout::Pc);
    rt.post_to_package(id, 0x62ed04ec);
    rt.update(1.0);
    rt.update(1.0);
    let native = rt.tree(id);
    for name in ["TRACKMAPTARGETRING", "TAC_Lines_7500", "3rdPersonNeedle", "SPEED_DIGIT_1", "TURBO_GROUP"] {
        let a = projected.nodes.iter().find(|n| n.name_hash == fe_hash_upper(name)).unwrap();
        let b = native.nodes.iter().find(|n| n.name_hash == fe_hash_upper(name)).unwrap();
        for corner in [Vec3::ZERO, Vec3::new(-0.5, -0.5, 0.0), Vec3::new(0.5, 0.5, 0.0)] {
            let actual = a.world.transform_point3(corner);
            let expected = b.world.transform_point3(corner);
            close(actual.x, expected.x);
            close(actual.y, expected.y);
        }
    }
}
