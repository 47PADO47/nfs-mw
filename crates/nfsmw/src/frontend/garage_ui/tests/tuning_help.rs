//! Tuning Help must fit its native line slots and remain modal during input.
use super::{Harness, Page, model};
use blackbox_feng::{NodeKind, font::TextStyle, runtime::pad};

#[test]
#[ignore = "requires installed tuning package"]
fn batch11_tuning_exit_waits_for_native_animation_and_ignores_extra_input() {
    for button in [pad::BACK, pad::ACCEPT, pad::START] {
        let mut h = Harness::open(Page::Tuning, model());
        h.run(button, 1);
        assert!(h.events.is_empty() && h.saved.is_empty(), "exit must not close/save on the first frame");
        let mut opacity = Vec::new();
        for _ in 0..6 {
            h.run(0, 1);
            let tree = h.screens.trees().pop().unwrap();
            opacity.push(tree.nodes.iter().find(|n| n.name_hash == 0xb3a6_9e89).unwrap().world_colour[3]);
        }
        // The authored footer reaches zero before the event handler finishes the complete exit.
        assert!(opacity.iter().any(|a| *a < 255), "native exit affects its footer: {opacity:?}");
        let original = h.model.draft.clone();
        for bits in [pad::RIGHT, pad::BUTTON0 << 5, pad::BACK, pad::ACCEPT] {
            h.run(bits, 1);
            h.run(0, 1);
        }
        assert_eq!(h.model.draft, original, "transient exit inputs cannot edit tuning");
        assert!(h.events.is_empty() && h.saved.is_empty());
        h.run(0, 60);
        match button {
            pad::BACK => {
                assert_eq!(h.events.iter().filter(|e| matches!(e, crate::garage::MenuEvent::Cancel)).count(), 1)
            }
            _ => assert_eq!(h.saved.len(), 1, "save only once after animation completion"),
        }
    }
}

#[test]
#[ignore = "requires installed tuning and error-dialog packages"]
fn batch11_failed_tuning_save_restores_visible_screen_and_allows_retry() {
    let mut h = Harness::open(Page::Tuning, model());
    let before = h.screens.trees().pop().unwrap();
    let help_before = before.nodes.iter().find(|n| n.name_hash == 0x6e1f_61a0).unwrap().world_colour[3];
    h.fail_save = true;
    h.press(pad::ACCEPT);
    assert_eq!(h.top(), "InGameDialog.fng");
    h.press(pad::ACCEPT);
    assert_eq!(h.top(), "Pause_Performance_Tuning.fng");
    let tree = h.screens.trees().pop().unwrap();
    let footer = tree.nodes.iter().find(|n| n.name_hash == 0xb3a6_9e89).unwrap();
    assert!(footer.visible && footer.world_colour[3] > 0, "failed save restores the footer");
    assert_eq!(
        tree.nodes.iter().find(|n| n.name_hash == 0x6e1f_61a0).unwrap().world_colour[3],
        help_before,
        "recovery does not open Help"
    );
    h.fail_save = false;
    h.press(pad::ACCEPT);
    assert_eq!(h.saved.len(), 1);
}

#[test]
#[ignore = "requires installed tuning package and fonts"]
fn batch11_tuning_help_lines_do_not_wrap_again_inside_native_slots() {
    let mut model = model();
    model.draft.performance.levels.fill(1);
    let mut h = Harness::open(Page::Tuning, model);
    for row in 0..7 {
        h.press(pad::BUTTON0 << 5);
        for scroll in 0..8 {
            let tree = h.screens.trees().pop().unwrap();
            let mut nonempty = 0;
            for hash in 0x46be_6be4..=0x46be_6bea {
                let node = tree.nodes.iter().find(|n| n.name_hash == hash).unwrap();
                let Some(text) = node.text.as_deref().filter(|t| !t.is_empty()) else { continue };
                nonempty += 1;
                let NodeKind::Text { font, justification, leading, max_width } = node.kind else {
                    panic!("line is text")
                };
                let assets = h.screens.assets();
                let font = assets.font(font).unwrap();
                let size = assets.texture_size(assets.resolve(font.texture_hash)).unwrap();
                let scale = assets.font_scale(font.hash);
                let layout = font.layout_scaled(text, TextStyle { justification, leading, max_width }, size, scale);
                assert!(
                    layout.height <= font.height() * scale + 0.01,
                    "row {row}, scroll {scroll}, text {text:?}: height {} line {} node scale {} width limit {max_width}",
                    layout.height,
                    font.height() * scale,
                    node.world.x_axis.length()
                );
            }
            assert!(nonempty > 0);
            h.press(pad::DOWN);
        }
        h.press(pad::BACK);
        if row < 6 {
            h.press(pad::DOWN);
        }
    }
    assert!(h.saved.is_empty(), "Help never saves tuning");
}

#[test]
#[ignore = "requires installed tuning package"]
fn batch11_tuning_help_round_trip_keeps_native_pc_footer_hidden() {
    let mut h = Harness::open(Page::Tuning, model());
    h.press(pad::BUTTON0 << 5);
    h.press(pad::BACK);
    for _ in 0..30 {
        for device in [crate::input::InputDevice::Xbox, crate::input::InputDevice::Keyboard] {
            let pages = h.screens.presented(&crate::input::Bindings::default(), device);
            let tree = &pages.last().unwrap().tree;
            for hash in [0xd9e6_3061, 0x9502_d148, 0x5232_50e9] {
                assert!(
                    !tree.nodes.iter().find(|n| n.name_hash == hash).unwrap().visible,
                    "native PC footer returned after Help"
                );
            }
        }
        h.run(0, 1);
    }
}

#[test]
#[ignore = "requires installed tuning package"]
fn batch12_tuning_help_has_one_device_aware_hide_hint() {
    let mut h = Harness::open(Page::Tuning, model());
    for _ in 0..2 {
        h.press(pad::BUTTON0 << 5);
        for device in [crate::input::InputDevice::Xbox, crate::input::InputDevice::Keyboard] {
            let pages = h.screens.presented(&crate::input::Bindings::default(), device);
            let tree = &pages.last().unwrap().tree;
            let text: Vec<_> = tree
                .nodes
                .iter()
                .filter(|n| n.visible && n.world_colour[3] > 0)
                .filter_map(|n| n.text.as_deref())
                .collect();
            assert_eq!(text.iter().filter(|&&label| label == "Hide").count(), 1);
            for label in ["Cancel", "Accept", "Help"] {
                assert!(!text.contains(&label), "Help owns its footer while open: {text:?}");
            }
            for hash in [0x027f_7503, 0x64bc_dff2] {
                assert!(!tree.nodes.iter().find(|n| n.name_hash == hash).unwrap().visible);
            }
        }
        h.press(pad::BACK);
    }
    assert!(h.saved.is_empty());
}

#[test]
#[ignore = "requires installed tuning and discard-dialog packages"]
fn batch12_changed_tuning_discard_waits_for_tuning_leave_and_gates_edits() {
    let mut initial = model();
    initial.draft.performance.levels.fill(1);
    let mut h = Harness::open(Page::Tuning, initial);
    h.press(pad::RIGHT);
    h.press(pad::BACK);
    h.press(pad::LEFT);
    h.events.clear();
    h.run(pad::ACCEPT, 1);
    for _ in 0..30 {
        if h.top() != "InGameDialog.fng" {
            break;
        }
        h.run(0, 1);
    }
    assert_eq!(h.top(), "Pause_Performance_Tuning.fng");
    assert!(
        !h.events.iter().any(|event| matches!(event, crate::garage::MenuEvent::Cancel)),
        "closing the discard popup must first animate the tuning page"
    );
    let before = h.model.draft.clone();
    for bits in [pad::RIGHT, pad::BUTTON0 << 5, pad::BACK, pad::ACCEPT] {
        h.run(bits, 1);
        h.run(0, 1);
    }
    assert_eq!(h.model.draft, before);
    h.run(0, 60);
    assert_eq!(h.events.iter().filter(|event| matches!(event, crate::garage::MenuEvent::Cancel)).count(), 1);
    assert!(h.saved.is_empty());
}

#[test]
#[ignore = "requires installed tuning package and textures"]
fn batch12_tuning_halves_retain_native_height_y_and_forward_uvs() {
    let mut initial = model();
    initial.draft.performance.levels.fill(1);
    let mut h = Harness::open(Page::Tuning, initial);
    for (profile, expected) in [(0, -5), (1, 5), (2, 0)] {
        h.model.draft.tuning.active = profile;
        h.model.draft.tuning.profiles[usize::from(profile)].fill(expected);
        h.ui.sync(&h.model);
        h.run(0, 1);
        let tree = h.screens.trees().pop().unwrap();
        let package = h.screens.assets();
        for row in 1..=7 {
            for (side, visible) in [("LEFT", expected < 0), ("RIGHT", expected > 0)] {
                let name = blackbox_feng::fe_hash_upper(&format!("FILLBAR_{side}_SLIDER_{row}"));
                let node = tree.nodes.iter().find(|n| n.name_hash == name).unwrap();
                assert_eq!(node.visible, visible);
                if !visible {
                    continue;
                }
                let NodeKind::Image { texture, uv, .. } = node.kind else { panic!("slider image") };
                assert!(uv[2] > uv[0], "left geometry provides mirroring; UVs advance forwards");
                let texture_width = package.texture_size(texture).unwrap().0 as f32;
                assert!(((uv[2] - uv[0]) * texture_width - 82.0).abs() < 0.01);
            }
        }
    }
}
