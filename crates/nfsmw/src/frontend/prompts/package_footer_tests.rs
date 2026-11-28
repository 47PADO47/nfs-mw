use blackbox_feng::{NodeKind, UiNode, UiTree};
use glam::Mat4;

use super::*;

/// A node of a synthetic tree: its name, parent, package flags and picture (if any).
struct N(u32, Option<usize>, u32, Option<u32>);

fn tree(nodes: &[N]) -> (UiTree, Vec<u32>) {
    let mut tree = UiTree::default();
    let mut flags = Vec::new();
    for (i, N(name, parent, flag, texture)) in nodes.iter().enumerate() {
        let kind = match texture {
            Some(texture) => NodeKind::Image {
                texture: *texture,
                uv: [0.0, 0.0, 1.0, 1.0],
                mask: None,
                mask_rotation: [0.5, 0.5, 0.0],
                mask_uv: [0.0, 0.0, 1.0, 1.0],
            },
            None => NodeKind::default(),
        };
        tree.nodes.push(UiNode {
            index: i,
            name_hash: *name,
            parent: *parent,
            kind,
            visible: true,
            world: Mat4::IDENTITY,
            ..UiNode::default()
        });
        tree.draw_order.push(i);
        flags.push(*flag);
    }
    (tree, flags)
}

/// The photo's footer: keyboard Continue and Race Stats groups, the pad's, and an unrelated start-hidden panel.
fn photo() -> (UiTree, Vec<u32>) {
    tree(&[
        N(PHOTO[0].keys, None, KEYBOARD, None),
        N(0xC6D9_1A27, Some(0), 0, Some(0xC6AF_DD7E)),
        N(0xD7B6_EDB7, Some(0), 0, Some(0xD7B6_EDB7)),
        N(PHOTO[0].pad, None, PAD, None),
        N(0x6851_AAF5, Some(3), 0x10, Some(0x6851_AAF5)),
        N(PHOTO[1].keys, None, KEYBOARD, None),
        N(0xD7B6_EDB7, Some(5), 0, Some(0x68A2_9A8B)),
        N(PHOTO[1].pad, None, PAD, None),
        N(0x7A8B_6BD0, Some(7), 0x10, Some(0x7A8B_6BD0)),
        N(0x46CB_7764, None, KEYBOARD, Some(0xC6AF_DD7E)),
    ])
}

fn drawn(tree: &UiTree) -> Vec<u32> {
    tree.draw_order.iter().map(|&i| tree.nodes[i].name_hash).collect()
}

fn run(tree: &mut UiTree, flags: &[u32], footer: Footer, device: InputDevice) -> Vec<HintArea> {
    apply(tree, |n| flags[n.index], screen::PHOTO, footer, &Bindings::default(), device)
}

#[test]
fn on_the_keyboard_the_pad_copy_goes() {
    let (mut t, flags) = photo();
    let areas = run(&mut t, &flags, Footer::RacePhoto { race: true }, InputDevice::Keyboard);
    let shown = drawn(&t);
    assert!(!shown.contains(&PHOTO[0].pad) && !shown.contains(&0x6851_AAF5), "the pad's Continue is hidden");
    assert!(shown.contains(&0xD7B6_EDB7) && shown.contains(&0x46CB_7764), "the keys and the panel stay");
    let actions: Vec<_> = areas.iter().map(|a| a.action).collect();
    assert_eq!(actions, [Action::MenuAccept, Action::MenuButton2], "clicks press the keyboard buttons");
}

#[test]
fn on_a_pad_the_keyboard_hints_go_and_the_icons_follow_the_pad() {
    let (mut t, flags) = photo();
    run(&mut t, &flags, Footer::RacePhoto { race: true }, InputDevice::Xbox);
    let shown = drawn(&t);
    assert!(!shown.contains(&PHOTO[0].keys) && !shown.contains(&0xD7B6_EDB7), "the keyboard hints are hidden");
    assert!(shown.contains(&0x46CB_7764), "a start-hidden panel with no key art is no hint");
    let icon = t.nodes.iter().find(|n| n.name_hash == 0x6851_AAF5).unwrap();
    assert!(matches!(icon.kind, NodeKind::Image { texture: ATLAS, .. }), "the pad's glyph replaces the console art");
}

#[test]
fn a_speed_trap_photo_has_continue_only() {
    let (mut t, flags) = photo();
    run(&mut t, &flags, Footer::RacePhoto { race: false }, InputDevice::Keyboard);
    let shown = drawn(&t);
    assert!(!shown.contains(&PHOTO[1].keys) && !shown.contains(&0x68A2_9A8B));
    assert!(shown.contains(&PHOTO[0].keys));
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_race_photo_draws_the_pad_stats_caption_over_its_shadow() {
    use crate::frontend::flow_movie_tests::install_rig;
    use crate::frontend::photo::PhotoLogic;
    use crate::viewer::{PhotoReport, RaceReport};

    let mut rig = install_rig().expect("the install is available");
    rig.with(|fe, _, env| {
        let photo = PhotoLogic::for_race(PhotoReport::sample(), RaceReport::sample());
        fe.screens.open_with(screen::PHOTO, Box::new(photo), true, env);
        for _ in 0..280 {
            fe.screens.update(1.0 / 60.0, 0, env);
        }
    });
    let presented = |device| rig.fe.screens.presented_trees(&Bindings::default(), device).pop().unwrap();
    let pad_tree = presented(InputDevice::Xbox);
    let position = |guid| pad_tree.draw_order.iter().position(|&i| pad_tree.nodes[i].guid == guid);
    let shadow = position(0x000F_C868).expect("stock dark Stats shadow is drawn");
    let caption = position(0x000F_C866).expect("stock light Stats caption is drawn");
    assert!(shadow < caption, "the dark offset cannot cover the light caption");
    let colour = |i: usize| pad_tree.nodes[pad_tree.draw_order[i]].world_colour;
    assert!(colour(caption)[0] > colour(shadow)[0], "the foreground keeps its authored contrast");
    let keyboard = presented(InputDevice::Keyboard);
    assert!(keyboard.draw_order.iter().all(|&i| !matches!(keyboard.nodes[i].guid, 0x000F_C866 | 0x000F_C868)));
    let label = crate::i18n::game_or_error(0xD1F2_C488);
    assert!(keyboard.draw_order.iter().any(|&i| keyboard.nodes[i].text.as_deref() == Some(label.as_str())));
}

#[test]
fn only_the_compared_screens_draw_their_package_footer() {
    assert!(owns(screen::PHOTO, Footer::RacePhoto { race: true }));
    assert!(owns("SMS_Message.fng", Footer::SmsMessage { replay: true }));
    assert!(owns(screen::CAR_SELECT, Footer::CareerCarSelect { accept: None, marker: false, back: true }));
    assert!(
        !owns(screen::CAR_SELECT, Footer::CareerCarSelect { accept: None, marker: true, back: true }),
        "the marker offer keeps the host hints"
    );
    assert!(owns(screen::CAR_SELECT, Footer::CarSelect), "the car lot's and the quick race's car select");
    assert!(!owns(screen::MAIN_MENU, Footer::Standard));
}
