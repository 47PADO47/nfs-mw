//! Installed-package checks of the SMS inbox and message screens: the tab marks and frame, the footer that follows the
//! device used last, deleting from the list, and (with a GPU) captures of both screens for both devices.

use std::sync::Arc;

use blackbox_feng::runtime::pad;
use blackbox_feng::{NodeKind, UiTree};

use super::make;
use crate::career::{Career, CareerData, CareerHandle};
use crate::frontend::career_screens::rep_sheets::playtest::Harness;
use crate::frontend::ids::screen;
use crate::frontend::logic::Args;
use crate::input::InputDevice;
use crate::ui::input_icons::ATLAS;

/// The story tab's and the tips tab's new-message icons, the corner frame on the selected tab, and the first row's text.
const STORY_NEW: u32 = 0x000F_321F;
const TIPS_NEW: u32 = 0x000F_337E;
const TAB_FRAME: u32 = 0x000F_45EE;
const FIRST_ROW_TEXT: u32 = 0x000F_319B;
/// Both copies of the inbox's own hints: the console's (inside the footer group) and the keyboard's three groups.
const PACKAGE_HINTS: [u32; 6] = [0x000F_46A8, 0x000F_46AB, 0x000F_4714, 0x0012_2503, 0x0010_6663, 0x0010_6659];

fn career_with(ids: &[u32]) -> Career {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let mut career = Career::new("SMS", Arc::new(CareerData::load(&dir).unwrap()), &Default::default()).unwrap();
    for &id in ids {
        assert!(career.settings.sms.deliver(id), "message {id}");
    }
    career
}

fn open_inbox(h: &mut Harness) {
    let logic = make(&screen::SMS_MAILBOXES.to_ascii_lowercase(), Args::default()).unwrap();
    h.open(screen::SMS_MAILBOXES, logic);
}

fn node(tree: &UiTree, guid: u32) -> &blackbox_feng::UiNode {
    tree.nodes.iter().find(|n| n.guid == guid).unwrap_or_else(|| panic!("{guid:#x}"))
}

fn drawn_hints(h: &Harness, device: InputDevice) -> (UiTree, usize) {
    let raw = h.tree().nodes.len();
    (h.presented(device), raw)
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_tab_marks_and_frame_follow_the_selected_tab() {
    let _guard = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|e| e.into_inner());
    CareerHandle::shared().set(career_with(&[15, 123]));
    let mut h = Harness::installed();
    open_inbox(&mut h);
    let story = h.tree();
    assert!(!node(&story, STORY_NEW).visible, "no mark beside the tab the list is on");
    assert!(node(&story, TIPS_NEW).visible, "the unread tip is marked beside the Text tab");
    let at_story = node(&story, TAB_FRAME).position.x;
    assert!(
        node(&story, FIRST_ROW_TEXT).text.as_deref().is_some_and(|t| !t.contains("Debug")),
        "the row has a caption"
    );

    h.press(pad::LEFT);
    let tips = h.tree();
    assert!(node(&tips, STORY_NEW).visible, "the unread story message is marked beside the Voice tab");
    assert!(!node(&tips, TIPS_NEW).visible);
    assert!(node(&tips, TAB_FRAME).position.x > at_story + 50.0, "the frame moves to the Text tab");
    CareerHandle::shared().clear();
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_inbox_footer_follows_the_device_used_last() {
    let _guard = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|e| e.into_inner());
    CareerHandle::shared().set(career_with(&[15]));
    let mut h = Harness::installed();
    open_inbox(&mut h);
    for device in [InputDevice::Keyboard, InputDevice::Xbox, InputDevice::Keyboard] {
        let (shown, raw) = drawn_hints(&h, device);
        let hints = &shown.nodes[raw..];
        let says = |text: &str| hints.iter().any(|n| n.visible && n.text.as_deref() == Some(text));
        assert!(says("Back") && says("Read") && says("Delete"), "{device:?}: the drawn footer");
        let glyphs =
            hints.iter().any(|n| n.visible && matches!(n.kind, NodeKind::Image { texture, .. } if texture == ATLAS));
        assert_eq!(glyphs, device == InputDevice::Xbox, "{device:?}: pad glyphs only for the controller");
        for guid in PACKAGE_HINTS {
            assert!(!shown.nodes.iter().any(|n| n.guid == guid && n.visible), "{device:?}: {guid:#x} still shows");
        }
    }
    CareerHandle::shared().clear();
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn delete_on_the_list_asks_for_the_highlighted_message() {
    let _guard = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|e| e.into_inner());
    CareerHandle::shared().set(career_with(&[15]));
    let mut h = Harness::installed();
    open_inbox(&mut h);
    h.press(pad::BUTTON0 << 5);
    assert_eq!(h.screens.top(), Some(screen::DIALOG), "the question is up: {:?}", h.commands);
    CareerHandle::shared().clear();
}

#[test]
#[ignore = "requires installed SMS packages and a Vulkan GPU"]
fn gpu_sms_captures_for_both_devices() {
    let _guard = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|e| e.into_inner());
    CareerHandle::shared().set(career_with(&[15, 17, 18, 123]));
    let mut h = Harness::installed();
    open_inbox(&mut h);
    for (shot, then) in [("inbox", None), ("message", Some(pad::ACCEPT))] {
        if let Some(button) = then {
            h.press(button);
            h.run(0, 60);
            assert_eq!(h.screens.top(), Some(screen::SMS_MESSAGE), "Accept opens the highlighted message");
        }
        for (label, device) in [("keyboard", InputDevice::Keyboard), ("pad", InputDevice::Xbox)] {
            let tree = h.presented(device);
            crate::frontend::tests::rendered_playtest::render(
                &tree,
                h.screens.assets(),
                (1280, 720),
                &format!("sms-{shot}-{label}"),
            );
        }
    }
    CareerHandle::shared().clear();
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn accept_opens_the_highlighted_message_and_back_brings_the_list_back() {
    const LIST: u32 = 0x0012_2311;
    let _guard = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|e| e.into_inner());
    CareerHandle::shared().set(career_with(&[15, 17]));
    let mut h = Harness::installed();
    open_inbox(&mut h);
    let shows_list = |tree: &UiTree| node(tree, LIST).visible && node(tree, LIST).world_colour[3] > 0;
    assert!(shows_list(&h.tree()), "the list is in when the inbox opens");
    h.press(pad::DOWN);
    h.press(pad::ACCEPT);
    assert_eq!(h.screens.top(), Some(screen::SMS_MESSAGE));
    assert_eq!(super::opened(), Some(15), "Down then Accept opens the second row (the older message)");
    h.press(pad::BACK);
    h.run(0, 60);
    assert!(
        h.screens.top().is_some_and(|top| top.eq_ignore_ascii_case(screen::SMS_MAILBOXES)),
        "{:?}",
        h.screens.top()
    );
    assert!(shows_list(&h.tree()), "the list is back after the message");
    let (shown, raw) = drawn_hints(&h, InputDevice::Keyboard);
    assert!(shown.nodes[raw..].iter().any(|n| n.visible && n.text.as_deref() == Some("Read")), "the footer is back");
    CareerHandle::shared().clear();
}
