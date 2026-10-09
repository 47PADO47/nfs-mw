//! The radio's keys and buttons: the bound actions become [`Control`]s while the game is being driven.

use bevy_ecs::prelude::*;

use super::Control;
use crate::app::Host;
use crate::audio::Audio;
use crate::input::{Action, ActionState};

/// The control the player asked for this frame, if any (one press at a time: toggle, then next, then previous).
pub fn requested(actions: &ActionState) -> Option<Control> {
    [
        (Action::RadioToggle, Control::Toggle),
        (Action::RadioNext, Control::Next),
        (Action::RadioPrevious, Control::Previous),
    ]
    .into_iter()
    .find(|(action, _)| actions.just_pressed(*action))
    .map(|(_, control)| control)
}

/// Pass a radio key to the radio. It only listens while the game is on and not paused: the pause menu uses the
/// same pad buttons (D-pad left and right) to move its sliders.
pub fn radio_input(actions: Res<ActionState>, host: NonSend<Host>, mut audio: NonSendMut<Audio>) {
    let Some(control) = requested(&actions) else { return };
    if !host.scene.hud_state().is_some_and(|hud| hud.visible) {
        return;
    }
    match audio.radio_control(control) {
        Ok(message) => log::info!("radio {message}"),
        Err(e) => log::info!("radio: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use bevy_app::App;
    use bevy_input::keyboard::{Key, KeyCode, KeyboardInput};
    use bevy_input::{ButtonState, InputPlugin};
    use bevy_time::Time;

    use super::*;
    use crate::input::InputLayerPlugin;

    fn press(app: &mut App, key_code: KeyCode) {
        app.world_mut().write_message(KeyboardInput {
            key_code,
            logical_key: Key::Enter,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
        app.update();
    }

    #[test]
    fn the_default_keys_ask_for_the_radio_controls_once() {
        for (key, control) in
            [(KeyCode::KeyM, Control::Toggle), (KeyCode::Period, Control::Next), (KeyCode::Comma, Control::Previous)]
        {
            let mut app = App::new();
            app.add_plugins((InputPlugin, InputLayerPlugin)).init_resource::<Time>();
            press(&mut app, key);
            assert_eq!(requested(app.world().resource::<ActionState>()), Some(control), "{key:?}");
            // Held down, it does not ask again.
            app.update();
            assert_eq!(requested(app.world().resource::<ActionState>()), None, "{key:?}");
        }
    }

    #[test]
    fn nothing_pressed_asks_for_nothing() {
        assert_eq!(requested(&ActionState::default()), None);
    }
}
