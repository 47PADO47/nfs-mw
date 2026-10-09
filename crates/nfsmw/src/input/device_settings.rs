//! Keep Bevy's device values unfiltered; the action layer applies the user's deadzones.

use bevy_ecs::prelude::*;
use bevy_input::gamepad::{AxisSettings, ButtonAxisSettings, Gamepad, GamepadSettings};

/// Required settings already exist when a gamepad is added. Configuring them here also covers
/// reconnects and runs before Bevy processes raw input in the connection frame.
pub(super) fn configure_added_gamepad(added: On<Add<Gamepad>>, mut devices: Query<&mut GamepadSettings>) {
    let Ok(mut settings) = devices.get_mut(added.entity) else { return };
    // Bevy retains raw values, but its default 5% zones and 1% change thresholds can discard
    // small changes or leave a stale value when returning to zero. Apply filtering once, later.
    let axis = AxisSettings::new(-1.0, 0.0, 0.0, 1.0, 0.0).expect("valid unfiltered axis settings");
    let button_axis = ButtonAxisSettings { low: 0.0, high: 1.0, threshold: 0.0 };
    settings.default_axis_settings = axis.clone();
    settings.default_button_axis_settings = button_axis.clone();
    for override_settings in settings.axis_settings.values_mut() {
        *override_settings = axis.clone();
    }
    for override_settings in settings.button_axis_settings.values_mut() {
        *override_settings = button_axis.clone();
    }
    // Digital ButtonSettings retain their separate press/release hysteresis.
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_app::App;
    use bevy_ecs::message::MessageCursor;
    use bevy_input::{
        InputPlugin,
        gamepad::{
            ButtonSettings, GamepadAxis, GamepadAxisChangedEvent, GamepadButton, GamepadButtonChangedEvent,
            GamepadConnection, GamepadConnectionEvent, RawGamepadAxisChangedEvent, RawGamepadButtonChangedEvent,
            RawGamepadEvent,
        },
    };

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(InputPlugin).add_observer(configure_added_gamepad);
        app
    }

    fn connect(app: &mut App, entity: Entity) {
        app.world_mut()
            .write_message(GamepadConnectionEvent::new(
                entity,
                GamepadConnection::Connected { name: "Software gamepad".into(), vendor_id: None, product_id: None },
            ))
            .unwrap();
    }

    fn axis(app: &mut App, entity: Entity, axis: GamepadAxis, value: f32) {
        app.world_mut()
            .write_message(RawGamepadEvent::Axis(RawGamepadAxisChangedEvent::new(entity, axis, value)))
            .unwrap();
    }

    fn button(app: &mut App, entity: Entity, button: GamepadButton, value: f32) {
        app.world_mut()
            .write_message(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(entity, button, value)))
            .unwrap();
    }

    fn close(actual: f32, expected: f32) {
        assert!((actual - expected).abs() < 1e-6, "{actual} versus {expected}");
    }

    #[test]
    fn connection_frame_and_small_axis_changes_reach_bevy_without_stale_values() {
        for input in GamepadAxis::all() {
            let mut app = app();
            let entity = app.world_mut().spawn_empty().id();
            let mut events = MessageCursor::<GamepadAxisChangedEvent>::default();
            connect(&mut app, entity);
            for value in [0.006, 0.0, 0.03, 0.031, 0.002, 0.0, -0.006, 0.0, -0.03, -0.002, 0.0] {
                axis(&mut app, entity, input, value);
                app.update();
                close(app.world().get::<Gamepad>(entity).unwrap().get(input).unwrap(), value);
                let received =
                    events.read(app.world().resource::<Messages<GamepadAxisChangedEvent>>()).collect::<Vec<_>>();
                assert_eq!(received.len(), 1);
                assert_eq!(received[0].axis, input);
                close(received[0].value, value);
            }
        }
    }

    #[test]
    fn small_triggers_reach_bevy_and_digital_buttons_keep_hysteresis() {
        for input in [GamepadButton::LeftTrigger2, GamepadButton::RightTrigger2] {
            let mut app = app();
            let entity = app.world_mut().spawn_empty().id();
            let mut events = MessageCursor::<GamepadButtonChangedEvent>::default();
            connect(&mut app, entity);
            for (value, pressed) in [
                (0.006, false),
                (0.0, false),
                (0.03, false),
                (0.031, false),
                (0.002, false),
                (0.0, false),
                (0.74, false),
                (0.75, true),
                (1.0, true),
                (0.70, true),
                (0.65, false),
                (0.002, false),
                (0.0, false),
            ] {
                button(&mut app, entity, input, value);
                app.update();
                let pad = app.world().get::<Gamepad>(entity).unwrap();
                close(pad.get(input).unwrap(), value);
                assert_eq!(pad.pressed(input), pressed);
                let received =
                    events.read(app.world().resource::<Messages<GamepadButtonChangedEvent>>()).collect::<Vec<_>>();
                assert_eq!(received.len(), 1);
                close(received[0].value, value);
            }
        }
    }

    #[test]
    fn existing_analog_overrides_are_neutralized_and_digital_overrides_survive_reconnect() {
        let mut app = app();
        let mut settings =
            GamepadSettings { default_button_settings: ButtonSettings::new(0.8, 0.6).unwrap(), ..Default::default() };
        settings.button_settings.insert(GamepadButton::South, ButtonSettings::new(0.9, 0.7).unwrap());
        settings.axis_settings.insert(GamepadAxis::LeftStickX, AxisSettings::default());
        settings.button_axis_settings.insert(GamepadButton::RightTrigger2, ButtonAxisSettings::default());
        let entity = app.world_mut().spawn(settings).id();
        connect(&mut app, entity);
        axis(&mut app, entity, GamepadAxis::LeftStickX, 0.03);
        button(&mut app, entity, GamepadButton::RightTrigger2, 0.03);
        app.update();
        close(app.world().get::<Gamepad>(entity).unwrap().get(GamepadAxis::LeftStickX).unwrap(), 0.03);
        close(app.world().get::<Gamepad>(entity).unwrap().get(GamepadButton::RightTrigger2).unwrap(), 0.03);
        app.world_mut().write_message(GamepadConnectionEvent::new(entity, GamepadConnection::Disconnected)).unwrap();
        app.update();
        assert!(app.world().get::<Gamepad>(entity).is_none());
        {
            let mut settings = app.world_mut().get_mut::<GamepadSettings>(entity).unwrap();
            settings.default_axis_settings = AxisSettings::default();
            settings.default_button_axis_settings = ButtonAxisSettings::default();
            settings.axis_settings.insert(GamepadAxis::LeftStickX, AxisSettings::default());
            settings.button_axis_settings.insert(GamepadButton::RightTrigger2, ButtonAxisSettings::default());
        }
        connect(&mut app, entity);
        axis(&mut app, entity, GamepadAxis::LeftStickX, 0.006);
        button(&mut app, entity, GamepadButton::RightTrigger2, 0.006);
        app.update();
        axis(&mut app, entity, GamepadAxis::LeftStickX, 0.0);
        button(&mut app, entity, GamepadButton::RightTrigger2, 0.0);
        app.update();
        let pad = app.world().get::<Gamepad>(entity).unwrap();
        assert_eq!(pad.get(GamepadAxis::LeftStickX), Some(0.0));
        assert_eq!(pad.get(GamepadButton::RightTrigger2), Some(0.0));
        let settings = app.world().get::<GamepadSettings>(entity).unwrap();
        assert!(!settings.default_button_settings.is_pressed(0.79));
        assert!(settings.default_button_settings.is_pressed(0.8));
        assert!(!settings.default_button_settings.is_released(0.61));
        assert!(settings.default_button_settings.is_released(0.6));
        let south = settings.get_button_settings(GamepadButton::South);
        assert!(!south.is_pressed(0.89));
        assert!(south.is_pressed(0.9));
        assert!(!south.is_released(0.71));
        assert!(south.is_released(0.7));
    }
}
