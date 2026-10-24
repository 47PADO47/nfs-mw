//! A scene request is handed to the original FEng front end; F6 cannot replace it.

use crate::garage::{Car, Model, Overlay, Selection};
use crate::input::{Action, ActionState, Bindings, MouseCapture, UiFocus};
use crate::viewer::Scene;
use bevy_app::{App, AppExit, Update};
use bevy_input::{ButtonInput, keyboard::KeyCode, mouse::MouseButton};
use bevy_window::{CursorOptions, PrimaryWindow, Window};
use blackbox_gfx::{FrameParams, Instance, RenderBackend};

struct Shop(Model);
impl Scene for Shop {
    fn title(&self) -> String {
        String::new()
    }
    fn init(&mut self, _: &mut dyn RenderBackend) -> anyhow::Result<()> {
        Ok(())
    }
    fn update(&mut self, _: &mut dyn RenderBackend, _: &ActionState, _: f32) {}
    fn frame(&mut self, _: f32) -> (FrameParams, &[Instance]) {
        unreachable!()
    }
    fn garage(&mut self) -> Option<&mut Model> {
        Some(&mut self.0)
    }
}

#[test]
fn a_scene_request_waits_for_feng_and_blocks_the_testing_overlay() {
    let mut car = Car { folder: "CAR".into(), ..Default::default() };
    car.performance.categories[0].max_level = 3;
    let model = Model {
        cars: vec![car],
        applied: Selection { car: "CAR".into(), ..Default::default() },
        open_requested: true,
        shop: true,
        ..Default::default()
    };
    let settings = crate::settings::Settings::from(crate::settings::Partial::default());
    let mut host = crate::app::Host::new(Box::new(Shop(model)), &settings, None);
    host.renderer = Some(Box::new(crate::app::test_backend::NullBackend::new(crate::settings::test_caps::native())));
    let mut overlay = Overlay::default();
    overlay.start_requested = true;
    let mut app = App::new();
    app.insert_non_send(host)
        .insert_resource(overlay)
        .init_resource::<crate::devtools::Console>()
        .init_resource::<MouseCapture>()
        .init_resource::<UiFocus>()
        .init_resource::<ActionState>()
        .init_resource::<ButtonInput<MouseButton>>()
        .add_message::<AppExit>()
        .add_systems(Update, crate::app::cursor_update);
    app.world_mut().spawn((Window { focused: true, ..Default::default() }, PrimaryWindow, CursorOptions::default()));
    let bindings = Bindings::default();
    app.world_mut().resource_mut::<ActionState>().update_for_test(
        &bindings,
        &[KeyCode::Enter, KeyCode::KeyW],
        &[],
        false,
    );
    app.update();
    assert!(app.world().resource::<Overlay>().menu_requested);
    assert!(!app.world().resource::<Overlay>().open, "normal requests never open egui");
    assert!(!app.world().resource::<Overlay>().start_requested, "F6 cannot bypass a pending transaction");
    assert!(!app.world().resource::<UiFocus>().0);
    assert!(!app.world().resource::<ActionState>().pressed(Action::Throttle));
    assert!(app.world().resource::<ActionState>().pressed(Action::MenuAccept));
}
