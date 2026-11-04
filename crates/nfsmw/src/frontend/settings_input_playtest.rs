//! Settings transitions must not reveal the package's keyboard artwork behind replacement hints.

use super::*;
use crate::input::{Bindings, InputDevice};
use blackbox_feng::NodeKind;

fn check_frame(h: &Harness, saw_hint: &mut bool) {
    let raw = h.screens.trees().pop().expect("settings or return menu");
    for device in [InputDevice::Xbox, InputDevice::Keyboard, InputDevice::Xbox] {
        let shown = h.screens.presented_trees(&Bindings::default(), device).pop().unwrap();
        for n in &raw.nodes {
            let pc_key = matches!(n.kind, NodeKind::Image { texture: 0xd7b6_edb7 | 0x1bb3_c514, .. });
            if pc_key {
                let painted = &shown.nodes[n.index];
                if h.screens.top().is_some_and(|name| [screen::OPTIONS, screen::PAUSE_OPTIONS].contains(&name)) {
                    let root = n.parent.expect("PC hint owns a key, caption and background");
                    assert!(!shown.nodes[root].visible, "the native hint background also stays hidden");
                }
                assert!(
                    !painted.visible,
                    "native PC key flashed during {:?}: hash={:08x}",
                    h.screens.top(),
                    n.name_hash
                );
                assert!(!shown.draw_order.contains(&n.index));
            }
            if n.name_hash != 0xe795_717d {
                continue;
            }
            let parent = n.parent.and_then(|i| raw.nodes.get(i));
            if !parent.is_some_and(|p| [0x84bc_f2b5, 0xb3a6_9e89].contains(&p.name_hash)) {
                continue;
            }
            let painted = &shown.nodes[n.index];
            assert!(!painted.visible, "legacy settings key must not be composited: {:?}", h.screens.top());
            assert!(!shown.draw_order.contains(&n.index));
        }
        let hints = &shown.nodes[raw.nodes.len()..];
        let visible = |n: &&blackbox_feng::UiNode| n.visible && n.world_colour[3] != 0;
        *saw_hint |= hints.iter().filter(visible).any(|n| n.text.is_some());
        if device == InputDevice::Xbox {
            assert!(hints.iter().filter(visible).all(|n| n.text.as_deref() != Some("Esc")));
        }
        if device == InputDevice::Keyboard && hints.iter().filter(visible).any(|n| n.text.as_deref() == Some("Back")) {
            assert!(
                hints.iter().filter(visible).any(|n| n.text.as_deref() == Some("Esc")),
                "real device switch retains keyboard binding"
            );
        }
    }
}

#[test]
#[ignore = "requires installed menu packages"]
fn installed_playtest_settings_back_keeps_controller_prompts() {
    for (name, menu, pause) in
        [(screen::OPTIONS, screen::MAIN_MENU_SUB, false), (screen::PAUSE_OPTIONS, screen::PAUSE_MENU, true)]
    {
        for category in [Category::Audio, Category::Gameplay, Category::Player, Category::Video] {
            let mut h = Harness::open(name, Args { pause, category, ..Args::default() }).unwrap();
            let mut saw_hint = false;
            for _ in 0..90 {
                check_frame(&h, &mut saw_hint);
                h.run(0, 1);
            }
            assert!(saw_hint, "settings hints must become visible");
            h.run(pad::BACK, 1);
            let mut returned = false;
            for _ in 0..180 {
                check_frame(&h, &mut saw_hint);
                returned |= h.screens.top() == Some(menu);
                h.run(0, 1);
            }
            assert!(returned, "Back returns to the visible category menu");
            // Re-enter the remembered category and repeat Back to exercise rebuilt presentation caches.
            h.press(pad::ACCEPT);
            h.wait(1.0);
            assert_eq!(h.screens.top(), Some(name));
            h.run(pad::BACK, 1);
            for _ in 0..180 {
                check_frame(&h, &mut saw_hint);
                h.run(0, 1);
            }
            assert_eq!(h.screens.top(), Some(menu));
        }
    }
}

#[test]
#[ignore = "requires installed settings footer artwork"]
fn pass_b_settings_footer_suppresses_reactivated_native_content_without_matching_caption_text() {
    use crate::frontend::logic::Footer;
    for (name, pause, root_hash) in [(screen::OPTIONS, false, 0x84bc_f2b5), (screen::PAUSE_OPTIONS, true, 0xb3a6_9e89)]
    {
        let mut h = Harness::open(name, Args { pause, category: Category::Audio, ..Default::default() }).unwrap();
        h.wait(1.0);
        let raw = h.screens.trees().pop().unwrap();
        let root = raw.nodes.iter().find(|n| n.name_hash == root_hash).unwrap().index;
        let mut candidates = Vec::new();
        for node in &raw.nodes {
            if node.text.is_none() {
                continue;
            }
            let mut parent = node.parent;
            while let Some(at) = parent {
                if at == root {
                    candidates.push(node.index);
                    break;
                }
                parent = raw.nodes[at].parent;
            }
        }
        assert!(!candidates.is_empty());
        for device in [InputDevice::Xbox, InputDevice::Keyboard] {
            let mut tree = raw.clone();
            for &index in &candidates {
                tree.nodes[index].text = Some("Reactivated native hint".into());
                tree.nodes[index].visible = true;
                tree.nodes[index].world_colour[3] = 255;
                if !tree.draw_order.contains(&index) {
                    tree.draw_order.push(index);
                }
            }
            crate::frontend::prompts::decorate(
                &mut tree,
                name,
                Footer::Standard,
                &Bindings::default(),
                device,
                h.screens.assets(),
            );
            assert!(
                candidates.iter().all(|&i| !tree.nodes[i].visible),
                "footer ownership must not depend on caption wording: {name} {device:?}"
            );
            assert!(
                tree.nodes[raw.nodes.len()..].iter().any(|n| n.visible && n.text.is_some()),
                "host hints remain visible"
            );
        }
    }
}
