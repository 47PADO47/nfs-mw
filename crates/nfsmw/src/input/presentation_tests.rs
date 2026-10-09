use super::{Action, ActionState, InputDevice, InputLayerPlugin, InputPresentation};
use bevy_app::App;
use bevy_ecs::prelude::*;
use bevy_input::{
    ButtonState, InputPlugin,
    gamepad::{
        GamepadAxis, GamepadButton, GamepadConnection, GamepadConnectionEvent, RawGamepadAxisChangedEvent,
        RawGamepadButtonChangedEvent, RawGamepadEvent,
    },
    keyboard::{Key, KeyCode, KeyboardInput},
};
use bevy_time::Time;

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((InputPlugin, InputLayerPlugin)).init_resource::<Time>();
    app
}
fn connect(app: &mut App) -> Entity {
    let entity = app.world_mut().spawn_empty().id();
    app.world_mut().write_message(GamepadConnectionEvent::new(
        entity,
        GamepadConnection::Connected { name: "Software Xbox".into(), vendor_id: None, product_id: None },
    ));
    app.update();
    entity
}
fn button(app: &mut App, entity: Entity, button: GamepadButton, value: f32) {
    app.world_mut().write_message(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(entity, button, value)));
}
fn axis(app: &mut App, entity: Entity, value: f32) {
    app.world_mut().write_message(RawGamepadEvent::Axis(RawGamepadAxisChangedEvent::new(
        entity,
        GamepadAxis::LeftStickX,
        value,
    )));
}
fn key(app: &mut App, key_code: KeyCode) {
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(KeyboardInput {
            key_code,
            logical_key: Key::Enter,
            state,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    }
}

#[test]
fn fast_start_and_accept_pulses_are_not_lost() {
    let mut app = app();
    let pad = connect(&mut app);
    for (code, action) in [(GamepadButton::Start, Action::MenuStart), (GamepadButton::South, Action::MenuAccept)] {
        button(&mut app, pad, code, 1.0);
        button(&mut app, pad, code, 0.0);
        app.update();
        assert!(app.world().resource::<ActionState>().just_pressed(action));
        app.update();
        assert!(!app.world().resource::<ActionState>().pressed(action));
    }
}

#[test]
fn console_neutral_polling_and_drift_do_not_switch_prompts() {
    let mut app = app();
    let pad = connect(&mut app);
    button(&mut app, pad, GamepadButton::South, 1.0);
    app.update();
    key(&mut app, KeyCode::F12);
    app.update();
    assert_eq!(app.world().resource::<InputPresentation>().device, InputDevice::Xbox);
    button(&mut app, pad, GamepadButton::South, 0.0);
    key(&mut app, KeyCode::Enter);
    app.update();
    assert_eq!(app.world().resource::<InputPresentation>().device, InputDevice::Keyboard);
    for amount in [0.02, 0.14, 0.28, 0.0] {
        axis(&mut app, pad, amount);
        app.update();
        assert_eq!(app.world().resource::<InputPresentation>().device, InputDevice::Keyboard);
    }
    button(&mut app, pad, GamepadButton::East, 1.0);
    app.update();
    assert_eq!(app.world().resource::<InputPresentation>().device, InputDevice::Xbox);
}

#[test]
fn controllers_take_over_without_combining_axes_and_disconnect_clears_input() {
    let mut app = app();
    let first = connect(&mut app);
    let second = connect(&mut app);
    axis(&mut app, first, 0.7);
    app.update();
    assert_eq!(app.world().resource::<InputPresentation>().active_pad, Some(first));
    axis(&mut app, second, -0.65);
    app.update();
    assert_eq!(app.world().resource::<InputPresentation>().active_pad, Some(second));
    assert!(app.world().resource::<ActionState>().value(Action::Steer) < 0.0);
    axis(&mut app, first, 0.0);
    app.world_mut().write_message(GamepadConnectionEvent::new(second, GamepadConnection::Disconnected));
    app.update();
    let presentation = app.world().resource::<InputPresentation>();
    assert!(presentation.disconnected);
    assert_eq!(presentation.device, InputDevice::Keyboard);
    assert_eq!(app.world().resource::<ActionState>().value(Action::Steer), 0.0);
    app.update();
    assert!(!app.world().resource::<InputPresentation>().disconnected);
}

#[test]
fn menu_stick_hysteresis_is_independent_of_driving_curve() {
    let mut app = app();
    let pad = connect(&mut app);
    for (amount, pressed, edge) in [
        (0.54, false, false),
        (0.56, true, true),
        (0.5, true, false),
        (0.36, true, false),
        (0.34, false, false),
        (0.56, true, true),
        (-0.6, false, false),
    ] {
        axis(&mut app, pad, amount);
        app.update();
        let state = app.world().resource::<ActionState>();
        assert_eq!(state.pressed(Action::MenuRight), pressed, "{amount}");
        assert_eq!(state.just_pressed(Action::MenuRight), edge, "{amount}");
    }
    assert!(app.world().resource::<ActionState>().pressed(Action::MenuLeft));
}

#[test]
fn slow_deliberate_stick_movement_switches_devices_without_held_noise_flicker() {
    let mut app = app();
    let first = connect(&mut app);
    let second = connect(&mut app);
    key(&mut app, KeyCode::Enter);
    app.update();
    for amount in [0.1, 0.2, 0.3, 0.4, 0.5, 0.6] {
        axis(&mut app, second, amount);
        app.update();
    }
    assert_eq!(app.world().resource::<InputPresentation>().device, InputDevice::Xbox);
    assert_eq!(app.world().resource::<InputPresentation>().active_pad, Some(second));
    key(&mut app, KeyCode::Enter);
    app.update();
    for amount in [0.62, 0.58, 0.63, 0.61] {
        axis(&mut app, second, amount);
        axis(&mut app, first, 0.02);
        app.update();
        assert_eq!(app.world().resource::<InputPresentation>().device, InputDevice::Keyboard);
    }
    for amount in [0.65, 0.7, 0.75, 0.8] {
        axis(&mut app, second, amount);
        app.update();
    }
    assert_eq!(app.world().resource::<InputPresentation>().device, InputDevice::Xbox);
}
