//! Batch 10 regressions use the installed controls packages and normal pad dispatch.
use super::*;
use crate::frontend::controls_layout;
use crate::input::{Bindings, InputDevice};

#[test]
fn a_click_focuses_the_visible_button_under_the_pointer() {
    use glam::Vec3;
    let Some(mut h) = Harness::open(screen::MAIN_MENU, Args::default()) else { return };
    h.wait(1.0);
    let tree = h.screens.trees().pop().unwrap();
    let visible = |b: &blackbox_feng::ObjectRef| tree.nodes.iter().any(|n| n.index == b.index && n.visible);
    let button = h.screens.top_buttons().into_iter().find(visible).expect("the main menu shows a button");
    let node = tree.nodes.iter().find(|n| n.index == button.index).unwrap();
    let centre = node.world.transform_point3(Vec3::ZERO).truncate();
    assert!(h.screens.focus_button_at(centre), "the centre of a visible button is on it");
    assert_eq!(h.screens.focused_button(), Some(button));
    assert!(!h.screens.focus_button_at(glam::Vec2::new(-600.0, 0.0)), "far off every button");
}

#[test]
#[ignore = "requires installed controls packages"]
fn batch10_controls_navigation_clears_departed_field_highlights() {
    for (name, pause) in [(screen::CONTROLS, false), (screen::PAUSE_CONTROLS, true)] {
        let mut h = Harness::open(name, Args { pause, ..Args::default() }).unwrap();
        h.wait(1.0);
        let layout = controls_layout::for_package(name);
        let baseline = h.screens.trees().pop().unwrap();
        let neutral = baseline.nodes.iter().find(|n| n.guid == layout.secondary[7]).unwrap().world_colour;
        for bits in [pad::DOWN, pad::DOWN, pad::RIGHT, pad::UP, pad::LEFT, pad::DOWN, pad::RIGHT] {
            h.press(bits);
            h.wait(0.2);
            let tree = h.screens.trees().pop().unwrap();
            let fields: Vec<_> = tree
                .nodes
                .iter()
                .filter(|n| layout.primary.contains(&n.guid) || layout.secondary.contains(&n.guid))
                .collect();
            assert_eq!(fields.len(), 16);
            let active: Vec<_> = fields.iter().filter(|n| n.world_colour != neutral).map(|n| n.guid).collect();
            assert_eq!(active.len(), 1, "{name}: only the current field stays highlighted: {active:x?}");
            let focused = h.screens.focused_button().unwrap();
            assert_eq!(active[0], tree.nodes[focused.index].guid, "highlight follows the actual remap target");
        }
        let tree = h.screens.trees().pop().unwrap();
        let target = tree.nodes.iter().find(|n| n.guid == layout.secondary[6]).unwrap();
        assert!(h.screens.focus_button_at(target.world.transform_point3(glam::Vec3::ZERO).truncate()));
        h.run(0, 3);
        let tree = h.screens.trees().pop().unwrap();
        let active: Vec<_> = tree
            .nodes
            .iter()
            .filter(|n| {
                (layout.primary.contains(&n.guid) || layout.secondary.contains(&n.guid)) && n.world_colour != neutral
            })
            .collect();
        assert_eq!(active.len(), 1, "pointer focus also clears departed highlights");
        assert_eq!(active[0].guid, layout.secondary[6]);
        h.commands.clear();
        h.press(pad::ACCEPT);
        let remaps: Vec<_> = h.commands.iter().filter(|c| matches!(c, Command::Remap(..))).collect();
        assert_eq!(remaps, vec![&Command::Remap(crate::frontend::controls_screen::ROWS[6], 1)]);
    }
}

#[test]
#[ignore = "requires installed controls packages"]
fn batch10_pause_controls_footer_has_bound_hints_without_native_pc_back() {
    let mut h = Harness::open(screen::PAUSE_CONTROLS, Args { pause: true, ..Args::default() }).unwrap();
    h.wait(1.0);
    let raw = h.screens.trees().pop().unwrap();
    for device in [InputDevice::Xbox, InputDevice::Keyboard, InputDevice::Xbox] {
        let shown = h.screens.presented_trees(&Bindings::default(), device).pop().unwrap();
        for text in ["Back", "Select", "Column", "Remap"] {
            assert!(
                shown.nodes[raw.nodes.len()..].iter().any(|n| n.visible && n.text.as_deref() == Some(text)),
                "missing {text}"
            );
        }
        for n in &raw.nodes {
            if matches!(n.kind, blackbox_feng::NodeKind::Image { texture: 0x1bb3_c514, .. }) {
                assert!(!shown.nodes[n.index].visible, "native PC Back must not overlap replacement hints");
            }
        }
    }
    h.press(pad::BACK);
    h.wait(1.0);
    assert_eq!(h.screens.top(), Some(screen::PAUSE_MENU));
}

#[test]
#[ignore = "requires installed controls packages"]
fn batch12_controller_native_tabs_show_bindings_and_retain_remap_targets() {
    for (name, pause) in [(screen::CONTROLS, false), (screen::PAUSE_CONTROLS, true)] {
        let mut h = Harness::open(name, Args { pause, ..Args::default() }).unwrap();
        h.screens.set_key_texts(crate::frontend::controls_pages::texts(&Bindings::default(), None, None));
        h.wait(1.0);
        let title = |screens: &Screens| {
            screens
                .trees()
                .pop()
                .unwrap()
                .nodes
                .iter()
                .find(|node| node.name_hash == 0x4e11_cf90)
                .unwrap()
                .text
                .clone()
                .unwrap()
        };
        assert_eq!(title(&h.screens), "Keyboard Configuration");
        h.press(pad::RTRIGGER);
        assert_eq!(title(&h.screens), "Controller Configuration");
        let layout = controls_layout::for_package(name);
        let tree = h.screens.trees().pop().unwrap();
        assert_eq!(
            tree.nodes.iter().find(|node| node.guid == layout.primary_text[0]).unwrap().text.as_deref(),
            Some("RT")
        );
        h.press(pad::DOWN);
        h.press(pad::DOWN);
        h.commands.clear();
        h.press(pad::ACCEPT);
        assert!(h.commands.contains(&Command::RemapController(crate::frontend::controls_screen::ROWS[2], 0)));
        h.press(pad::LTRIGGER);
        assert_eq!(title(&h.screens), "Keyboard Configuration");
        h.commands.clear();
        h.press(pad::ACCEPT);
        assert!(h.commands.contains(&Command::Remap(crate::frontend::controls_screen::ROWS[2], 0)));
    }
}

#[test]
#[ignore = "requires installed controls packages"]
fn batch12_controller_native_header_arrows_switch_pages_with_pointer() {
    for (name, pause) in [(screen::CONTROLS, false), (screen::PAUSE_CONTROLS, true)] {
        let mut h = Harness::open(name, Args { pause, ..Args::default() }).unwrap();
        h.screens.set_key_texts(crate::frontend::controls_pages::texts(&Bindings::default(), None, None));
        h.wait(1.0);
        for (hash, expected) in [(0x9345_e4a7, "Controller Configuration"), (0xd1cd_a594, "Keyboard Configuration")] {
            let tree = h.screens.trees().pop().unwrap();
            let arrow = tree.nodes.iter().find(|node| node.name_hash == hash && node.visible).unwrap();
            let point = arrow.world.transform_point3(glam::Vec3::ZERO).truncate();
            let mut env =
                Env { settings: &mut h.settings, changed: &mut h.changed, caps: test_caps::native(), mods: None };
            assert!(h.screens.click(point, &mut env).is_some());
            h.run(0, 2);
            let tree = h.screens.trees().pop().unwrap();
            assert_eq!(
                tree.nodes.iter().find(|node| node.name_hash == 0x4e11_cf90).unwrap().text.as_deref(),
                Some(expected)
            );
        }
    }
}
