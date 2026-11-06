//! Normal play wraps the driving scene; F5 must cross that boundary just like F6/F8.
use super::*;
use crate::extra_options::{Action, State};
use crate::viewer::Scene;
/// The fixture's driving scene: its extra options and the overlay state the wrapper passes on.
struct Tools(State, std::rc::Rc<std::cell::Cell<bool>>);
impl Scene for Tools {
    fn title(&self) -> String {
        "tools fixture".into()
    }
    fn init(&mut self, _: &mut dyn blackbox_gfx::RenderBackend) -> anyhow::Result<()> {
        Ok(())
    }
    fn update(&mut self, _: &mut dyn blackbox_gfx::RenderBackend, _: &crate::input::ActionState, _: f32) {}
    fn frame(&mut self, _: f32) -> (blackbox_gfx::FrameParams, &[blackbox_gfx::Instance]) {
        unreachable!()
    }
    fn extra_options(&mut self) -> Option<&mut State> {
        Some(&mut self.0)
    }
    fn screenshot_ui_hidden(&self) -> bool {
        self.0.hide_ui
    }
    fn set_overlay_open(&mut self, open: bool) {
        self.1.set(open);
    }
    fn apply_extra_option(&mut self, _: &mut dyn blackbox_gfx::RenderBackend, action: Action) -> anyhow::Result<()> {
        assert!(matches!(action, Action::Center));
        self.0.hide_ui = true;
        Ok(())
    }
}
#[test]
fn normal_play_forwards_f5_preferences_actions_and_screenshot_visibility() {
    let mut scene = Pausable::new(Box::new(Tools(State::default(), Default::default())), PauseFlag::default());
    scene.extra_options().expect("F5 must reach the driving world").camera.auto_center = false;
    assert!(!scene.extra_options().unwrap().camera.auto_center);
    assert!(!scene.screenshot_ui_hidden());
    let mut renderer = crate::app::test_backend::NullBackend::new(crate::settings::test_caps::native());
    scene.apply_extra_option(&mut renderer, Action::Center).unwrap();
    assert!(scene.screenshot_ui_hidden());
}
#[test]
fn normal_play_forwards_the_open_overlay_so_the_world_pauses_for_it() {
    let overlay = std::rc::Rc::new(std::cell::Cell::new(false));
    let mut scene = Pausable::new(Box::new(Tools(State::default(), overlay.clone())), PauseFlag::default());
    scene.set_overlay_open(true);
    assert!(overlay.get(), "F5 / F8 open must reach the driving world");
    scene.set_overlay_open(false);
    assert!(!overlay.get());
}
