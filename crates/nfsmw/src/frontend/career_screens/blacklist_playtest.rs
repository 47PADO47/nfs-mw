//! Installed-package reproduction for UI-032: a bio must bind its own rival media.

use blackbox_feng::{NodeKind, fe_hash_upper};

use super::*;
use crate::frontend::career_screens::rep_sheets::playtest::Harness;
use crate::input::InputDevice;

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn career_playtest_bio_uses_the_requested_rival_background_and_graffiti() {
    let mut h = Harness::installed();
    for bin in [1, 7, 15] {
        h.open(screen::SAFE_HOUSE_RIVAL_BIO, Box::new(BioLogic::new(Args { rival: bin, ..Args::default() })));
        for (hash, suffix) in [(0x2CBE_1DD0, "_BG"), (0xF5A2_A087, "_GRAF")] {
            let node = h.node(hash);
            assert!(node.visible, "rival {bin} {suffix} remains hidden");
            let texture = fe_hash_upper(&format!("RIVAL_{bin:02}{suffix}"));
            assert!(
                matches!(node.kind, NodeKind::Image { texture: key, .. } if key == texture),
                "rival {bin} {suffix}: {:?}",
                node.kind
            );
        }
        assert!(
            h.tree().nodes.iter().any(|n| n.visible && n.text.as_ref().is_some_and(|text| text.len() > 100)),
            "bio body"
        );
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_sonny_intro_presents_one_authored_continue_per_device() {
    let _career = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|err| err.into_inner());
    let mut h = Harness::installed();
    let args = Args { sequence: true, rival: 15, ..Args::default() };
    h.open(screen::SAFE_HOUSE_RIVAL_BIO, Box::new(BioLogic::new(args)));
    h.run(0, 620);
    for (device, caption, icon, texture) in [
        (InputDevice::Keyboard, 0x0011_6318, 0x0011_631B, 0xD7B6_EDB7),
        (InputDevice::Xbox, 0x0010_449D, 0x0010_449E, crate::ui::input_icons::ATLAS),
    ] {
        let tree = h.presented(device);
        let drawn: Vec<_> = tree.draw_order.iter().map(|&i| &tree.nodes[i]).collect();
        let label = crate::i18n::game_or_error(0x8098_A54C);
        let captions: Vec<_> =
            drawn.iter().filter(|node| node.text.as_deref() == Some(label.as_str())).map(|node| node.guid).collect();
        assert_eq!(
            captions,
            [caption],
            "{device:?} draws its authored Continue caption once: {:?}",
            tree.nodes.iter().find(|node| node.guid == caption).map(|node| (
                &node.text,
                node.visible,
                node.world_colour
            ))
        );
        assert!(drawn.iter().any(|node| node.guid == icon
            && matches!(node.kind, NodeKind::Image { texture: rendered, .. } if rendered == texture)));
    }
}
