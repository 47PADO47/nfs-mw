//! The mouse on the installed menus: a click takes a row, its arrows and a footer hint, and the wheel scrolls.

use super::*;

#[path = "overview_click_tests.rs"]
mod overview_click_tests;

#[path = "icon_click_tests.rs"]
mod icon_click_tests;

#[path = "reference_visual_tests.rs"]
mod reference_visual_tests;

#[path = "navigation_screen_tests.rs"]
mod navigation_screen_tests;

#[path = "menu_audio_tests.rs"]
mod menu_audio_tests;

impl Harness {
    /// A click at a point of the screen, as the front end hands it over; false when the screen does not take it.
    pub(in crate::frontend) fn click(&mut self, point: glam::Vec2) -> bool {
        let mut env =
            Env { settings: &mut self.settings, changed: &mut self.changed, caps: test_caps::native(), mods: None };
        let Some(commands) = self.screens.click(point, &mut env) else { return false };
        self.commands.extend(commands);
        true
    }

    /// `steps` notches of the mouse wheel (positive is up).
    fn scroll(&mut self, steps: i32) {
        let mut env =
            Env { settings: &mut self.settings, changed: &mut self.changed, caps: test_caps::native(), mods: None };
        let commands = self.screens.scroll(steps, &mut env);
        self.commands.extend(commands);
    }

    /// The centre of the named object on the top screen, and whether it is shown.
    fn node(&self, name: &str) -> (glam::Vec2, bool) {
        let tree = self.screens.trees().pop().unwrap();
        let hash = blackbox_feng::fe_hash_upper(name);
        let node = tree.nodes.iter().find(|n| n.name_hash == hash).unwrap_or_else(|| panic!("no {name}"));
        (node.world.transform_point3(glam::Vec3::ZERO).truncate(), node.visible)
    }
}

#[test]
fn a_click_selects_an_option_row_and_its_arrows_change_it() {
    use crate::frontend::options::{self, Control};
    let Some(mut h) = Harness::open(screen::OPTIONS, Args { category: Category::Video, ..Args::default() }) else {
        return;
    };
    h.wait(1.5);
    let rows = options::rows(Category::Video, &test_caps::native());
    let slot = rows
        .iter()
        .take(9)
        .position(|r| r.setting.control(&h.settings) == Control::Toggle)
        .expect("a toggle row on the first page");
    let (data, _) = h.node(&format!("OPTION_DATA_{}", slot + 1));
    assert!(h.click(data), "a click on a row is taken by the screen");
    h.wait(0.5);
    let before = h.settings;
    let (right, shown) = h.node(&format!("RIGHT_ARROW_{}", slot + 1));
    assert!(shown, "the clicked row is the selected one, so its arrows show");
    assert!(h.click(right), "a click on an arrow is taken");
    assert_ne!(h.settings, before, "the right arrow changes the clicked row");
    let (left, _) = h.node(&format!("LEFT_ARROW_{}", slot + 1));
    assert!(h.click(left), "the left arrow is taken too");
    assert_eq!(h.settings, before, "the left arrow undoes the right one");
}

#[test]
fn the_footer_hint_under_a_click_names_its_action() {
    use crate::input::{Action, Bindings, InputDevice};
    use glam::Vec3;
    let Some(h) = Harness::open(screen::OPTIONS, Args { category: Category::Video, ..Args::default() }) else { return };
    let (bindings, device) = (Bindings::default(), InputDevice::Xbox);
    let tree = h.screens.presented_trees(&bindings, device).pop().unwrap();
    let done = tree.nodes.iter().find(|n| n.text.as_deref() == Some("Done")).expect("the options footer says Done");
    let point = done.world.transform_point3(Vec3::ZERO).truncate();
    assert_eq!(h.screens.hint_at(point, &bindings, device), Some(Action::MenuBack), "Done is back");
    assert_eq!(h.screens.hint_at(glam::Vec2::new(-600.0, 0.0), &bindings, device), None, "far off the footer");
}

#[test]
fn the_wheel_moves_the_option_selection_like_the_arrows() {
    let Some(mut h) = Harness::open(screen::OPTIONS, Args { category: Category::Video, ..Args::default() }) else {
        return;
    };
    h.wait(1.5);
    let (_, first) = h.node("RIGHT_ARROW_1");
    assert!(first, "the first row starts selected");
    h.scroll(-2);
    h.wait(0.5);
    let (_, third) = h.node("RIGHT_ARROW_3");
    assert!(third, "two notches down select the third row");
    let (_, first) = h.node("RIGHT_ARROW_1");
    assert!(!first, "the first row is no longer selected");
}

/// The centre of the visible node with this name hash on the top screen.
fn centre_of(h: &Harness, name_hash: u32) -> Option<glam::Vec2> {
    let tree = h.screens.trees().pop()?;
    let node = tree.nodes.iter().find(|n| n.name_hash == name_hash && n.visible)?;
    Some(node.world.transform_point3(glam::Vec3::ZERO).truncate())
}

/// The map's GPS Yes/No dialog over the options screen, asking to set `answer`.
fn gps_dialog(answer: Command) -> Option<Harness> {
    use crate::frontend::confirm::{Confirm, ConfirmLogic};
    use crate::ui::Text;
    let mut h = Harness::open(screen::OPTIONS, Args { category: Category::Video, ..Args::default() })?;
    h.wait(1.5);
    let confirm = Confirm::new(Text::Literal("Activate GPS?".into()), answer);
    let mut env = Env { settings: &mut h.settings, changed: &mut h.changed, caps: test_caps::native(), mods: None };
    let opened = h.screens.open_with(screen::DIALOG, Box::new(ConfirmLogic::new(confirm)), false, &mut env);
    h.commands.extend(opened);
    h.wait(0.5);
    Some(h)
}

#[test]
fn the_gps_dialog_answers_only_on_yes_or_no() {
    use crate::frontend::confirm::{NO_BUTTON, YES_BUTTON};
    let route = Command::SetWaypoint([1.0, 2.0]);
    let Some(mut h) = gps_dialog(route.clone()) else { return };
    assert_eq!(h.screens.top(), Some(screen::DIALOG));

    // A click that misses both answers is taken by the dialog and answers nothing.
    let before = h.commands.len();
    assert!(h.click(glam::Vec2::new(-400.0, -300.0)), "the dialog takes the click");
    assert_eq!(h.screens.top(), Some(screen::DIALOG), "a click off the answers keeps the dialog open");
    assert_eq!(h.commands.len(), before, "and answers nothing");

    // Yes sets the route and closes the dialog.
    let yes = centre_of(&h, YES_BUTTON).expect("the Yes answer is on screen");
    assert!(h.click(yes));
    assert!(h.commands.contains(&route), "Yes sets the route");

    // No closes the dialog and sets nothing.
    let Some(mut h) = gps_dialog(route.clone()) else { return };
    let no = centre_of(&h, NO_BUTTON).expect("the No answer is on screen");
    assert!(h.click(no));
    assert!(!h.commands.contains(&route), "No sets no route");
    assert_ne!(h.screens.top(), Some(screen::DIALOG), "No closes the dialog");
}
