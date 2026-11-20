use bevy_app::{App, Update};
use bevy_input::ButtonInput;
use bevy_input::keyboard::KeyCode;
use blackbox_gfx::{FrameParams, Instance, RenderBackend};

use super::{DevPanels, Panel, Request, apply};
use crate::app::Host;
use crate::devtools::{Console, GraphicsLab};
use crate::input::{MouseCapture, UiFocus};
use crate::settings::{Partial, Settings};
use crate::viewer::Scene;

/// A scene with a garage, so the garage can be opened.
struct Stage(crate::garage::Model);

impl Scene for Stage {
    fn title(&self) -> String {
        String::new()
    }
    fn init(&mut self, _: &mut dyn RenderBackend) -> anyhow::Result<()> {
        Ok(())
    }
    fn update(&mut self, _: &mut dyn RenderBackend, _: &crate::input::ActionState, _: f32) {}
    fn frame(&mut self, _: f32) -> (FrameParams, &[Instance]) {
        unreachable!()
    }
    fn garage(&mut self) -> Option<&mut crate::garage::Model> {
        Some(&mut self.0)
    }
}

fn app() -> App {
    let settings = Settings::from(Partial::default());
    let host = Host::new(Box::new(Stage(Default::default())), &settings, None);
    let mut app = App::new();
    app.init_resource::<DevPanels>()
        .init_resource::<Console>()
        .init_resource::<GraphicsLab>()
        .init_resource::<crate::devtools::extra_options::Overlay>()
        .init_resource::<crate::garage::Overlay>()
        .init_resource::<MouseCapture>()
        .init_resource::<UiFocus>()
        .init_resource::<ButtonInput<KeyCode>>()
        .insert_non_send(host)
        .add_systems(Update, apply);
    app
}

fn ask(app: &mut App, request: Request) {
    app.world_mut().resource_mut::<DevPanels>().request(request);
    app.update();
}

fn which(app: &mut App) -> (bool, bool, bool) {
    let world = app.world();
    let dev = world.resource::<DevPanels>();
    (dev.menu, dev.vehicle, world.resource::<Console>().open)
}

#[test]
fn the_chord_opens_the_menu_and_gives_it_the_keyboard_and_closing_gives_it_back() {
    let mut app = app();
    app.world_mut().resource_mut::<MouseCapture>().0 = true;
    ask(&mut app, Request::Toggle);
    assert_eq!(which(&mut app), (true, false, false));
    assert!(app.world().resource::<UiFocus>().0, "the game's controls go quiet");
    assert!(!app.world().resource::<MouseCapture>().0, "the mouse is free");
    ask(&mut app, Request::Toggle);
    assert_eq!(which(&mut app), (false, false, false));
    assert!(!app.world().resource::<UiFocus>().0);
    assert!(app.world().resource::<MouseCapture>().0, "the capture comes back");
}

#[test]
fn a_panel_opened_from_the_menu_goes_back_to_it_and_back_again_closes() {
    let mut app = app();
    ask(&mut app, Request::Toggle);
    ask(&mut app, Request::Open { panel: Panel::Console, from_menu: true });
    assert_eq!(which(&mut app), (false, false, true));
    assert!(app.world().resource::<UiFocus>().0);
    ask(&mut app, Request::Back);
    assert_eq!(which(&mut app), (true, false, false), "back to the menu");
    ask(&mut app, Request::Back);
    assert_eq!(which(&mut app), (false, false, false));
    assert!(!app.world().resource::<UiFocus>().0);
}

#[test]
fn a_panel_opened_on_its_own_closes_whatever_was_up_and_back_closes_it_for_good() {
    let mut app = app();
    ask(&mut app, Request::Open { panel: Panel::Console, from_menu: false });
    ask(&mut app, Request::Open { panel: Panel::Vehicle, from_menu: false });
    assert_eq!(which(&mut app), (false, true, false), "one panel at a time");
    ask(&mut app, Request::Back);
    assert_eq!(which(&mut app), (false, false, false));
}

#[test]
fn the_chord_closes_a_panel_that_a_key_opened() {
    let mut app = app();
    app.world_mut().resource_mut::<GraphicsLab>().open = true;
    app.update();
    ask(&mut app, Request::Toggle);
    assert!(!app.world().resource::<GraphicsLab>().open);
    assert_eq!(which(&mut app), (false, false, false));
}

#[test]
fn a_key_that_opens_another_panel_takes_the_menu_away() {
    let mut app = app();
    ask(&mut app, Request::Toggle);
    app.world_mut().resource_mut::<Console>().open = true;
    app.update();
    assert_eq!(which(&mut app), (false, false, true));
}

#[test]
fn f11_opens_and_closes_the_vehicle_panel() {
    let mut app = app();
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::F11);
    app.update();
    assert_eq!(which(&mut app), (false, true, false));
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().clear();
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().release(KeyCode::F11);
    app.update();
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().clear();
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::F11);
    app.update();
    assert_eq!(which(&mut app), (false, false, false));
}

#[test]
fn the_garage_opens_from_the_menu_and_a_shop_transaction_is_left_alone() {
    let mut app = app();
    ask(&mut app, Request::Open { panel: Panel::Garage, from_menu: true });
    assert!(app.world().resource::<crate::garage::Overlay>().open);
    ask(&mut app, Request::CloseAll);
    assert!(!app.world().resource::<crate::garage::Overlay>().open);
    app.world_mut().resource_mut::<crate::garage::Overlay>().menu = true;
    ask(&mut app, Request::Toggle);
    assert_eq!(which(&mut app), (false, false, false), "the shop owns its input");
}
