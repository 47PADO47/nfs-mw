use super::*;
use crate::input::{Action, ActionState, Bindings};
use bevy_input::keyboard::KeyCode;

#[test]
fn categories_exclude_traffic_and_accessories() {
    assert_eq!(Category::from_usage(0), Some(Category::Racing));
    assert_eq!(Category::from_usage(1), Some(Category::Police));
    for usage in [2, 3, 4, -1, 99] {
        assert_eq!(Category::from_usage(usage), None);
    }
}

#[test]
fn garage_claims_opening_frame_and_its_rebind_survives_focus() {
    let mut bindings = Bindings::default();
    bindings.rebind_key(Action::Garage, KeyCode::F7);
    let mut actions = ActionState::default();
    actions.update_for_test(&bindings, &[KeyCode::F7, KeyCode::ArrowUp], &[], false);
    assert!(actions.just_pressed(Action::Garage));
    actions.capture_ui();
    assert_eq!(actions.value(Action::Throttle), 0.0);
    assert_eq!(actions.value(Action::MenuUp), 0.0);
    assert!(actions.typing());
    actions.update_for_test(&bindings, &[], &[], true);
    actions.update_for_test(&bindings, &[KeyCode::F7, KeyCode::Escape], &[], true);
    assert!(actions.just_pressed(Action::Garage));
    assert!(actions.just_pressed(Action::Cancel));
    actions.consume(Action::Cancel);
    assert!(!actions.just_pressed(Action::Cancel));
}

#[test]
fn closing_restores_previous_capture_and_releases_ui_focus() {
    for initial in [false, true] {
        let mut overlay = Overlay::default();
        let mut capture = MouseCapture(initial);
        let mut focus = UiFocus(false);
        overlay.set_open(true, &mut capture, &mut focus);
        assert!(!capture.0);
        assert!(focus.0);
        overlay.set_open(false, &mut capture, &mut focus);
        assert_eq!(capture.0, initial);
        assert!(!focus.0);
        assert!(overlay.had_input);
    }
}

#[test]
fn browsing_reverts_unapplied_paint_and_never_changes_the_applied_car() {
    let applied = Selection { car: "CAR_A".into(), color: Some([20, 30, 40]), ..Default::default() };
    let mut model = Model { applied: applied.clone(), ..Model::default() };
    model.reset_draft();
    model.select("CAR_B".into());
    assert_eq!(model.draft.color, None);
    assert_eq!(model.applied, applied);
    model.reset_draft();
    assert_eq!(model.draft, applied);
}

#[test]
fn garage_ui_builds_with_unavailable_cars_and_fixed_liveries() {
    let mut model = Model {
        cars: vec![Car {
            folder: "FIXED".into(),
            name: "FIXED".into(),
            paintable: false,
            unavailable: Some("no physics".into()),
            ..Car::default()
        }],
        draft: Selection { car: "FIXED".into(), color: None, ..Default::default() },
        ..Model::default()
    };
    let context = egui::Context::default();
    let mut open = true;
    let mut settings = crate::settings::Settings::from(crate::settings::Partial::default());
    context.begin_pass(egui::RawInput::default());
    assert!(show(&context, &mut open, &mut model, &mut settings).is_none());
    let mut output = context.end_pass();
    assert!(!output.shapes.is_empty());
    output.textures_delta.clear();
    assert!(model.applied.car.is_empty());
}

#[test]
fn cursor_system_opens_with_f6_blocks_clicks_and_consumes_closing_escape() {
    use crate::viewer::Scene;
    use bevy_app::{App, AppExit, Update};
    use bevy_input::{ButtonInput, mouse::MouseButton};
    use bevy_window::{CursorOptions, PrimaryWindow, Window};
    use blackbox_gfx::{FrameParams, Instance, RenderBackend};

    #[derive(Default)]
    struct GarageScene(Model);
    impl Scene for GarageScene {
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
        fn captures_mouse(&self) -> bool {
            true
        }
    }
    let settings = crate::settings::Settings::from(crate::settings::Partial::default());
    let mut host = crate::app::Host::new(Box::new(GarageScene::default()), &settings, None);
    host.renderer = Some(Box::new(crate::app::test_backend::NullBackend::new(crate::settings::test_caps::native())));
    let mut app = App::new();
    app.insert_non_send(host)
        .init_resource::<Overlay>()
        .init_resource::<crate::devtools::Console>()
        .insert_resource(MouseCapture(true))
        .insert_resource(UiFocus(false))
        .init_resource::<ActionState>()
        .init_resource::<ButtonInput<MouseButton>>()
        .add_message::<AppExit>()
        .add_systems(Update, crate::app::cursor_update);
    let window = app
        .world_mut()
        .spawn((Window { focused: true, ..Window::default() }, PrimaryWindow, CursorOptions::default()))
        .id();
    let bindings = Bindings::default();
    app.world_mut().resource_mut::<ActionState>().update_for_test(
        &bindings,
        &[KeyCode::F6, KeyCode::ArrowUp],
        &[],
        false,
    );
    app.world_mut().resource_mut::<ButtonInput<MouseButton>>().press(MouseButton::Left);
    app.update();
    assert!(app.world().resource::<Overlay>().open);
    assert!(!app.world().resource::<MouseCapture>().0);
    assert!(app.world().resource::<UiFocus>().0);
    assert_eq!(app.world().resource::<ActionState>().value(Action::Throttle), 0.0);
    assert!(app.world().entity(window).get::<CursorOptions>().unwrap().visible);
    app.world_mut().resource_mut::<ActionState>().update_for_test(&bindings, &[KeyCode::Escape], &[], true);
    app.update();
    assert!(!app.world().resource::<Overlay>().open);
    assert!(app.world().resource::<MouseCapture>().0);
    assert!(!app.world().resource::<UiFocus>().0);
    assert!(!app.world().resource::<ActionState>().pressed(Action::Cancel));
    assert!(app.world().resource::<Overlay>().had_input);
}
