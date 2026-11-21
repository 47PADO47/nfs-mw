//! The Bevy side of speech: keeps the speech volume in step with the settings and runs the queue every frame.

use bevy_app::{App, Update};
use bevy_ecs::prelude::*;
use bevy_time::Time;

use crate::app::{FrameSet, Host};
use crate::audio::Audio;
use crate::settings::Settings;

/// Register the speech systems. Called by the audio plugin.
pub fn add(app: &mut App) {
    app.add_systems(Update, (sync_speech_volume, run_speech).in_set(FrameSet::Prepare));
}

/// `set speech_volume 50` in the console and the config file reach the speech track.
fn sync_speech_volume(settings: Res<Settings>, mut audio: NonSendMut<Audio>, mut applied: Local<Option<f32>>) {
    let wanted = settings.speech_volume.amplitude();
    if *applied == Some(wanted) {
        return;
    }
    *applied = Some(wanted);
    audio.set_speech_volume(wanted);
}

/// The queue advances with the game; the pause menu holds it (a line already sounding finishes by itself). A message
/// screen over the frozen game lets it run, so its voice sounds while it is read (`cell_call::message_screen`).
fn run_speech(host: NonSend<Host>, time: Res<Time>, mut audio: NonSendMut<Audio>) {
    if host.scene.paused() && !super::super::cell_call::screen_speaks() {
        return;
    }
    audio.update_speech(time.delta_secs());
}
