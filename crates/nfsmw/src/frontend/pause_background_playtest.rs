use super::*;
use crate::frontend::logic::{Cx, ScreenLogic};
use crate::input::{Bindings, InputDevice};

struct Cover;
impl ScreenLogic for Cover {
    fn covers_screens(&self) -> bool {
        true
    }
    fn message(&mut self, _: &mut Cx, _: u32) {}
}

fn background(h: &Harness) -> blackbox_feng::UiTree {
    let screens = h.screens.presented(&Bindings::default(), InputDevice::Xbox);
    let backgrounds: Vec<_> = screens.iter().filter(|screen| screen.name == "InGameBackground.fng").collect();
    assert_eq!(backgrounds.len(), 1, "one shared native pause background must be drawn");
    (*backgrounds[0].tree).clone()
}

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn batch12_pause_tuning_and_options_keep_one_authored_background_beneath_their_pages() {
    let mut h = Harness::open(screen::PAUSE_MENU, Args { pause: true, ..Default::default() }).expect("installed menus");
    h.wait(1.0);
    let tree = background(&h);
    let shade = tree.nodes.iter().find(|node| node.name_hash == 0x2716_CDBF).expect("authored backdrop group");
    assert!(shade.visible && shade.world_colour[3] > 0);
    let centre = tree.nodes.iter().find(|node| node.name_hash == 0xC6AF_DD7E).unwrap();
    assert!(centre.visible && centre.size.x.abs() >= 500.0 && centre.size.y.abs() >= 480.0);
    h.press(pad::RIGHT);
    h.press(pad::ACCEPT);
    h.wait(1.0);
    background(&h);
    h.press(pad::ACCEPT);
    h.wait(1.0);
    assert_eq!(h.screens.top(), Some(screen::PAUSE_OPTIONS));
    background(&h);
    let mut env = Env { settings: &mut h.settings, changed: &mut h.changed, caps: test_caps::native(), mods: None };
    h.screens.open_with("Pause_Performance_Tuning.fng", Box::new(Cover), false, &mut env);
    h.wait(1.0);
    let pages = h.screens.presented(&Bindings::default(), InputDevice::Xbox);
    assert_eq!(pages.len(), 2, "the covering tuning page hides the pause menu, while retaining the background");
    background(&h);
    h.screens.clear();
    let mut env = Env { settings: &mut h.settings, changed: &mut h.changed, caps: test_caps::native(), mods: None };
    h.screens.open(screen::MAIN_MENU, Args::default(), true, &mut env);
    h.wait(1.0);
    assert!(
        h.screens
            .presented(&Bindings::default(), InputDevice::Xbox)
            .iter()
            .all(|screen| screen.name != "InGameBackground.fng")
    );
}
