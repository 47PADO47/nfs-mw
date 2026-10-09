//! The replacement hints must stay registered to the installed packages throughout transitions.

use blackbox_feng::{NodeKind, UiTree, font::TextStyle};
use glam::Vec3;

use super::*;
use crate::input::{Bindings, InputDevice};

const FOOTERS: [(&str, bool, u32, u32); 5] = [
    (screen::MAIN_MENU, false, 0x17B1_F254, 0xC6AF_DD7E),
    (screen::MAIN_MENU_SUB, false, 0x17B1_F254, 0xC6AF_DD7E),
    (screen::OPTIONS, false, 0x84BC_F2B5, 0x3D3B_C1AC),
    (screen::PAUSE_MENU, true, 0x2FE7_4244, 0x81B4_FAD0),
    (screen::PAUSE_OPTIONS, true, 0xB3A6_9E89, 0x81B4_FAD0),
];

fn assets() -> UiAssets {
    let path = std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR to run installed-menu tests");
    let dir = GameDir::open(std::path::PathBuf::from(path)).unwrap();
    UiAssets::load(&dir).unwrap()
}

fn assert_entrance(h: &Harness, group: u32) {
    let raw = h.screens.trees().pop().unwrap();
    let root = raw.nodes.iter().find(|n| n.name_hash == group).unwrap();
    assert_eq!(root.world_colour[3], 0, "incoming footer must start at its entrance pose");
    if h.screens.top() == Some(screen::PAUSE_MENU) {
        let glow = raw.nodes.iter().find(|n| n.name_hash == super::super::ids::ICON_SELECTION_GLOW).unwrap();
        assert_eq!(glow.world_colour[3], 0, "selection glow must not flash before its entrance fade");
    }
    for device in [InputDevice::Xbox, InputDevice::Keyboard] {
        let shown = h.screens.presented_trees(&Bindings::default(), device).pop().unwrap();
        assert!(shown.nodes[raw.nodes.len()..].iter().all(|n| !n.visible));
    }
}

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn incoming_screens_never_present_the_stored_end_pose() {
    for (name, pause, group, _) in FOOTERS {
        let mut h = Harness::open(name, Args { pause, category: Category::Gameplay, ..Args::default() }).unwrap();
        assert_entrance(&h, group);
        if name == screen::PAUSE_MENU {
            h.wait(1.0);
            let tree = h.screens.trees().pop().unwrap();
            let glow = tree.nodes.iter().find(|n| n.name_hash == super::super::ids::ICON_SELECTION_GLOW).unwrap();
            assert!(glow.visible && glow.world_colour[3] > 0, "authored glow animation must still run");
        }
    }
    for (menu, rows, pause, menu_group, rows_group) in [
        (screen::MAIN_MENU_SUB, screen::OPTIONS, false, 0x17B1_F254, 0x84BC_F2B5),
        (screen::PAUSE_MENU, screen::PAUSE_OPTIONS, true, 0x2FE7_4244, 0xB3A6_9E89),
    ] {
        let mut h = Harness::open(menu, Args { pause, options: true, ..Args::default() }).unwrap();
        h.wait(1.0);
        for (button, target, group) in [(pad::ACCEPT, rows, rows_group), (pad::BACK, menu, menu_group)] {
            h.run(button, 1);
            for _ in 0..180 {
                h.run(0, 1);
                if h.screens.top() == Some(target) {
                    break;
                }
            }
            assert_eq!(h.screens.top(), Some(target));
            // Inspect the switch frame itself, before advancing the incoming screen at all.
            assert_entrance(&h, group);
            h.wait(1.0);
        }
    }
}

fn assert_footer(raw: &UiTree, shown: &UiTree, group: u32, background: u32, assets: &UiAssets) {
    let root = raw.nodes.iter().find(|n| n.name_hash == group).unwrap();
    let bar = raw.nodes.iter().find(|n| n.parent == Some(root.index) && n.name_hash == background).unwrap();
    let inverse = root.world.inverse();
    let center = bar.local_position;
    let half = bar.size.abs() * 0.5;
    let hints = &shown.nodes[raw.nodes.len()..];
    assert!(!hints.is_empty());
    for n in hints {
        assert_eq!(n.parent, Some(root.index));
        assert_eq!(n.visible, root.visible && n.world_colour[3] != 0 && n.z > 0.0);
        assert_eq!(shown.draw_order.contains(&n.index), n.visible);
        let mut corners = Vec::new();
        match n.kind {
            NodeKind::Image { .. } => {
                for (x, y) in [(-0.5, -0.5), (0.5, -0.5), (-0.5, 0.5), (0.5, 0.5)] {
                    corners.push(Vec3::new(x, y, 0.0));
                }
            }
            NodeKind::Text { font, justification, leading, max_width } => {
                assert_eq!(n.world_colour[3], root.world_colour[3]);
                let f = assets.font(font).unwrap();
                let layout = f.layout(
                    n.text.as_deref().unwrap(),
                    TextStyle { justification, leading, max_width },
                    assets.texture_size(f.texture_hash).unwrap(),
                );
                for q in layout.quads {
                    corners.extend([Vec3::new(q.x0, q.y0, 0.0), Vec3::new(q.x1, q.y1, 0.0)]);
                }
            }
            _ => panic!("unexpected prompt node"),
        }
        for corner in corners {
            let p = (inverse * n.world).transform_point3(corner);
            assert!((p.x - center.x).abs() <= half.x + 0.01, "horizontal overflow: {:?} {p:?}", n.text);
            assert!((p.y - center.y).abs() <= half.y + 0.01, "vertical overflow: {:?} {p:?}", n.text);
        }
    }
}

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn hints_fit_native_footers_and_share_their_enter_and_leave_animations() {
    let assets = assets();
    for (name, pause, group, background) in FOOTERS {
        let mut h = Harness::open(name, Args { pause, category: Category::Gameplay, ..Args::default() }).unwrap();
        let mut faded = false;
        let mut settled = false;
        let mut leaving = false;
        for frame in 0..150 {
            h.run(0, 1);
            if frame == 90 {
                h.press(pad::ACCEPT);
            }
            if h.screens.top() != Some(name) {
                break;
            }
            let raw = h.screens.trees().pop().unwrap();
            let root = raw.nodes.iter().find(|n| n.name_hash == group).unwrap();
            faded |= root.world_colour[3] < 255;
            settled |= root.world_colour[3] == 255 && frame > 30;
            leaving |= root.world_colour[3] < 255 && frame >= 90;
            for device in [InputDevice::Xbox, InputDevice::Keyboard] {
                let shown = h.screens.presented_trees(&Bindings::default(), device).pop().unwrap();
                assert_footer(&raw, &shown, group, background, &assets);
            }
        }
        assert!(faded && settled, "{name} must exercise both transition and settled frames");
        assert!(leaving, "{name} must exercise the outgoing footer animation");
    }
}

#[test]
#[ignore = "requires installed menu packages and fonts; set NFSMW_GAME_DIR"]
fn custom_keycaps_fit_and_inherit_footer_transforms_and_visibility() {
    let assets = assets();
    let mut h = Harness::open(screen::PAUSE_OPTIONS, Args { pause: true, ..Args::default() }).unwrap();
    h.wait(1.0);
    let raw = h.screens.trees().pop().unwrap();
    let mut bindings = Bindings::default();
    bindings.bind(crate::input::Action::MenuBack, "key:Backspace", false).unwrap();
    for (visible, depth) in [(true, 0.0), (false, 0.0), (true, -900.0)] {
        let mut raw = raw.clone();
        let root = raw.nodes.iter_mut().find(|n| n.name_hash == 0xB3A6_9E89).unwrap();
        root.world = glam::Mat4::from_translation(Vec3::new(0.0, 0.0, depth))
            * glam::Mat4::from_rotation_z(0.1)
            * glam::Mat4::from_scale(Vec3::new(0.8, 0.9, 1.0))
            * root.world;
        root.world_colour[3] = 127;
        root.visible = visible;
        let mut shown = raw.clone();
        super::super::prompts::decorate(&mut shown, screen::PAUSE_OPTIONS, &bindings, InputDevice::Keyboard, &assets);
        assert_footer(&raw, &shown, 0xB3A6_9E89, 0x81B4_FAD0, &assets);
        assert!(shown.nodes[raw.nodes.len()..].iter().any(|n| n.text.as_deref() == Some("Backspace")));
        let insets: Vec<_> =
            shown.nodes[raw.nodes.len()..].iter().filter(|n| n.world_colour[..3] == [0, 0, 0]).collect();
        assert_eq!(insets.len(), 5);
        assert!(insets.iter().all(|n| n.world_colour[3] < 127));
    }
}
