use super::icon_items::{Id, icons};
use super::icon_menu::Kind;

#[test]
fn my_cars_is_a_normal_enabled_main_menu_entry() {
    let list = icons(Kind::Main);
    let entry = list.iter().find(|icon| icon.id == Id::MyCars).expect("My Cars entry");
    assert!(entry.enabled);
}

#[test]
fn a_paused_scene_keeps_preview_separate_from_confirm_and_cancel() {
    use super::scene::{Pausable, PauseFlag};
    use crate::app::test_backend::NullBackend;
    use crate::garage::Selection;
    use crate::input::ActionState;
    use crate::viewer::Scene;
    use blackbox_gfx::{FrameParams, Instance, RenderBackend};
    use std::sync::{Arc, Mutex};

    struct Observed(Arc<Mutex<Vec<&'static str>>>);
    impl Scene for Observed {
        fn title(&self) -> String {
            String::new()
        }
        fn init(&mut self, _: &mut dyn RenderBackend) -> anyhow::Result<()> {
            Ok(())
        }
        fn update(&mut self, _: &mut dyn RenderBackend, _: &ActionState, _: f32) {
            panic!("world must stay paused")
        }
        fn frame(&mut self, _: f32) -> (FrameParams, &[Instance]) {
            unreachable!()
        }
        fn begin_garage(&mut self, _: &mut dyn RenderBackend) -> anyhow::Result<()> {
            self.0.lock().unwrap().push("begin");
            Ok(())
        }
        fn preview_garage(&mut self, _: &mut dyn RenderBackend, _: Selection) -> anyhow::Result<()> {
            self.0.lock().unwrap().push("preview");
            Ok(())
        }
        fn confirm_garage(&mut self, _: &mut dyn RenderBackend, _: Selection) -> anyhow::Result<()> {
            self.0.lock().unwrap().push("confirm");
            Ok(())
        }
        fn cancel_garage(&mut self, _: &mut dyn RenderBackend) -> anyhow::Result<()> {
            self.0.lock().unwrap().push("cancel");
            Ok(())
        }
    }
    let seen = Arc::new(Mutex::new(Vec::new()));
    let paused = PauseFlag::default();
    paused.set(true);
    let mut scene = Pausable::new(Box::new(Observed(seen.clone())), paused);
    let mut renderer = NullBackend::new(crate::settings::test_caps::native());
    scene.begin_garage(&mut renderer).unwrap();
    scene.preview_garage(&mut renderer, Selection::default()).unwrap();
    scene.cancel_garage(&mut renderer).unwrap();
    assert_eq!(*seen.lock().unwrap(), ["begin", "preview", "cancel"]);
    scene.begin_garage(&mut renderer).unwrap();
    scene.preview_garage(&mut renderer, Selection::default()).unwrap();
    scene.confirm_garage(&mut renderer, Selection::default()).unwrap();
    assert_eq!(*seen.lock().unwrap(), ["begin", "preview", "cancel", "begin", "preview", "confirm"]);
}
