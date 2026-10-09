//! The Bevy side: creates [`Audio`] and keeps its volumes in step with the settings.

use bevy_app::{App, Plugin, Update};
use bevy_ecs::prelude::*;
use bevy_time::Time;
use game_install::GameDir;

use super::{Audio, MusicInput, Volumes};
use crate::app::FrameSet;
use crate::app::Host;
use crate::settings::Settings;

pub struct AudioPlugin {
    pub dir: GameDir,
}

/// The volumes the settings ask for.
pub fn volumes_of(settings: &Settings) -> Volumes {
    Volumes {
        master: settings.master_volume.amplitude(),
        music: settings.music_volume.amplitude(),
        sfx: settings.sfx_volume.amplitude(),
        engine: settings.engine_volume.amplitude(),
    }
}

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        let volumes = app.world().get_resource::<Settings>().map(volumes_of).unwrap_or_default();
        app.init_resource::<MusicInput>().insert_non_send(Audio::new(self.dir.clone(), volumes)).add_systems(
            Update,
            ((sync_volumes, sync_radio).in_set(FrameSet::Prepare), drive_car.in_set(FrameSet::Ui)),
        );
    }
}

/// `set volume 50` in the console and the config file reach the mixer.
fn sync_volumes(settings: Res<Settings>, mut audio: NonSendMut<Audio>) {
    if !settings.is_changed() {
        return;
    }
    let wanted = volumes_of(&settings);
    if audio.volumes() != wanted {
        audio.set_volumes(wanted);
    }
}

/// `radio = false` in the config file and `set radio off` in the console reach the radio. Only a change of the setting
/// does: the `radio on` and `radio off` commands stay in force until the setting changes again.
fn sync_radio(settings: Res<Settings>, mut audio: NonSendMut<Audio>, mut applied: Local<Option<bool>>) {
    if *applied == Some(settings.radio) {
        return;
    }
    *applied = Some(settings.radio);
    audio.set_radio_wanted(settings.radio);
}

/// The scene's car plays its engine.
fn drive_car(
    mut host: NonSendMut<Host>,
    time: Res<Time>,
    music: Res<MusicInput>,
    mut audio: NonSendMut<Audio>,
    mut seen: Local<u32>,
) {
    // A scene that went away takes its soundtrack with it, even when the next one has none.
    if *seen != host.scene_changes {
        *seen = host.scene_changes;
        audio.stop_music();
    }
    let car = host.scene.car_sound();
    audio.drive_car(car.as_ref(), time.delta_secs());
    // The pause menu freezes the car (no sound) but the game goes on, and so does the radio.
    let driving = car.is_some() || host.scene.paused();
    // The pursuit music takes the songs' place while a chase is on and for a while after it.
    let songs_free = audio.update_interactive(&music.state(), driving, time.delta_secs());
    audio.update_radio(driving && songs_free);
    if let Some(clip) = host.scene.take_clip()
        && let Err(e) = audio.play_music(super::pcm::sound(&clip))
    {
        log::warn!("the scene's sound: {e}");
    }
}
