//! The mods' place in the frame: two systems that send the frame events in `ModsPre` and `ModsPost`.

use bevy_app::{App, Plugin, Update};
use bevy_ecs::prelude::*;
use bevy_time::Time;

use super::Mods;
use crate::app::{FrameSet, Host};
use crate::settings::Settings;

/// Sends `FRAME_START` to the mods in `ModsPre`, and `SCENE_TICK` in `ModsPost`, after the scene has updated. Without
/// a [`Mods`] resource (the game started with none) the systems do nothing.
pub struct ModsPlugin;

impl Plugin for ModsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, frame_start.in_set(FrameSet::ModsPre))
            .add_systems(Update, scene_tick.in_set(FrameSet::ModsPost));
    }
}

fn frame_start(mods: Option<NonSendMut<Mods>>, time: Res<Time>) {
    let Some(mut mods) = mods else {
        return;
    };
    mods.frame_start(time.delta_secs());
}

fn scene_tick(mods: Option<NonSendMut<Mods>>, host: NonSend<Host>, settings: Res<Settings>, time: Res<Time>) {
    let Some(mut mods) = mods else {
        return;
    };
    mods.scene_tick(time.delta_secs(), settings.game_speed, host.scene.paused());
}
