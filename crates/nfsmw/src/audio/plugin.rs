//! The Bevy side: creates [`Audio`] and keeps its volumes in step with the settings.

use bevy_app::{App, Plugin, Update};
use bevy_ecs::prelude::*;
use bevy_time::Time;
use game_install::GameDir;

use super::{Audio, Volumes};
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
        app.insert_non_send(Audio::new(self.dir.clone(), volumes))
            .add_systems(Update, (sync_volumes.in_set(FrameSet::Prepare), drive_car.in_set(FrameSet::Ui)));
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

/// The scene's car plays its engine.
fn drive_car(host: NonSend<Host>, time: Res<Time>, mut audio: NonSendMut<Audio>) {
    audio.drive_car(host.scene.car_sound().as_ref(), time.delta_secs());
}
