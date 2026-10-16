//! The mods' place in the frame: two systems that send the frame events in `ModsPre` and `ModsPost`.

use bevy_app::{App, Plugin, Update};
use bevy_ecs::prelude::*;
use bevy_input::{ButtonInput, keyboard::KeyCode};
use bevy_time::Time;

use super::{Mods, outbox};
use crate::app::{FrameSet, Host};
use crate::devtools::Console;
use crate::input::ActionState;
use crate::settings::Settings;

/// Sends `FRAME_START` to the mods in `ModsPre`, and `SCENE_TICK` in `ModsPost`, after the scene has updated. Without
/// a [`Mods`] resource (the game started with none) the systems do nothing.
///
/// Each system is one frame's share of the mods' work: it copies the scene's clock in, fires the hotkeys pressed this
/// frame (`ModsPre` only), sends the event, and then makes the scene and the console take what the mods asked for.
pub struct ModsPlugin;

impl Plugin for ModsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, frame_start.in_set(FrameSet::ModsPre))
            .add_systems(Update, scene_tick.in_set(FrameSet::ModsPost));
    }
}

fn frame_start(
    mods: Option<NonSendMut<Mods>>,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    actions: Res<ActionState>,
    mut host: NonSendMut<Host>,
    mut console: ResMut<Console>,
) {
    let Some(mut mods) = mods else {
        return;
    };
    mods.sync(&mut *host.scene);
    mods.press(|key| keys.just_pressed(key) && !actions.typing());
    mods.frame_start(time.delta_secs());
    mods.apply(&mut *host.scene, &mut console);
}

fn scene_tick(
    mods: Option<NonSendMut<Mods>>,
    mut host: NonSendMut<Host>,
    settings: Res<Settings>,
    time: Res<Time>,
    mut console: ResMut<Console>,
) {
    // The outbox is drained even without mods, so a session that starts none does not keep its notices.
    let notices = outbox::drain_notices();
    let Some(mut mods) = mods else {
        return;
    };
    mods.sync(&mut *host.scene);
    mods.dispatch_notices(&notices);
    mods.scene_tick(time.delta_secs(), settings.game_speed, host.scene.paused());
    mods.apply(&mut *host.scene, &mut console);
}
