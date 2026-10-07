//! The Bevy side of the input layer: reads `bevy_input` and resolves the actions.

use bevy_app::{App, Plugin, PreUpdate};
use bevy_ecs::prelude::*;
use bevy_input::gamepad::{Gamepad, GamepadInput};
use bevy_input::keyboard::KeyCode;
use bevy_input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseButton, MouseScrollUnit};
use bevy_input::{ButtonInput, InputSystems};
use bevy_time::Time;

use super::snapshot::Snapshot;
use super::state::{ActionState, Bindings};

/// Whether the cursor is captured for mouse look. The window code sets it; the input layer reads it.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct MouseCapture(pub bool);

/// The UI (the console) has the keyboard: game actions go quiet.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct UiFocus(pub bool);

pub struct InputLayerPlugin;

impl Plugin for InputLayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Bindings>()
            .init_resource::<ActionState>()
            .init_resource::<MouseCapture>()
            .init_resource::<UiFocus>()
            .add_systems(PreUpdate, update_actions.after(InputSystems));
    }
}

/// Pixels per line, for devices that scroll in pixels.
const PIXELS_PER_LINE: f32 = 60.0;

#[allow(clippy::too_many_arguments)]
fn update_actions(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    pads: Query<&Gamepad>,
    capture: Res<MouseCapture>,
    focus: Res<UiFocus>,
    time: Res<Time>,
    bindings: Res<Bindings>,
    mut state: ResMut<ActionState>,
) {
    let mut snapshot = Snapshot {
        keys: keys.get_pressed().copied().collect(),
        buttons: buttons.get_pressed().copied().collect(),
        mouse_delta: (motion.delta.x, motion.delta.y),
        scroll: match scroll.unit {
            MouseScrollUnit::Line => scroll.delta.y,
            MouseScrollUnit::Pixel => scroll.delta.y / PIXELS_PER_LINE,
        },
        mouse_captured: capture.0,
        dt: time.delta_secs(),
        ..Snapshot::default()
    };
    for pad in &pads {
        snapshot.pad_buttons.extend(pad.get_pressed().copied());
        for input in pad.get_analog_axes() {
            let GamepadInput::Axis(axis) = *input else { continue };
            let value = pad.get(axis).unwrap_or(0.0);
            let entry = snapshot.pad_axes.entry(axis).or_insert(0.0);
            if value.abs() > entry.abs() {
                *entry = value;
            }
        }
    }
    state.update(&bindings, &snapshot, focus.0);
}
