//! The Bevy side: creates [`Audio`] and keeps its volumes in step with the settings.

use bevy_app::{App, Plugin, Update};
use bevy_ecs::prelude::*;
use bevy_time::Time;
use game_install::GameDir;

use super::oneshot::{self, OneShotPlayer};
use super::preview_play::PreviewPlayer;
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
        let mut audio = Audio::new(self.dir.clone(), volumes);
        audio.set_air_support(super::chase::load_air_support());
        app.init_resource::<MusicInput>().insert_non_send(audio).add_systems(
            Update,
            (
                (sync_volumes, sync_radio).in_set(FrameSet::Prepare),
                super::radio::input::radio_input.in_set(FrameSet::Ui),
                drive_car.in_set(FrameSet::Ui),
                play_preview.in_set(FrameSet::Ui),
                play_one_shots.in_set(FrameSet::Ui),
            ),
        );
        super::speech::add_systems(app);
    }
}

/// `set volume 50` in the console and the config file reach the mixer.
fn sync_volumes(settings: Res<Settings>, mut audio: NonSendMut<Audio>) {
    let _trace = blackbox_trace::scope("Audio: volumes");
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
    let _trace = blackbox_trace::scope("Audio: radio settings");
    if *applied == Some(settings.radio) {
        return;
    }
    *applied = Some(settings.radio);
    audio.set_radio_wanted(settings.radio);
}

/// The sound viewer's transport: what the scene asks for plays, and its playhead is reported back. A scene without one
/// stops what it had started.
fn play_preview(host: NonSendMut<Host>, mut audio: NonSendMut<Audio>, mut player: Local<PreviewPlayer>) {
    let _trace = blackbox_trace::scope("Audio: previews");
    match host.scene.sound_preview() {
        Some(preview) => player.service(&mut audio, &preview),
        None => player.stop(),
    }
}

/// The one-shot sounds the mods asked for (`bb.audio` `play`) since the last frame.
fn play_one_shots(mut audio: NonSendMut<Audio>, mut player: Local<OneShotPlayer>) {
    let _trace = blackbox_trace::scope("Audio: one shots");
    oneshot::service(&mut audio, &mut player);
}

/// The scene's car plays its engine.
fn drive_car(
    mut host: NonSendMut<Host>,
    time: Res<Time>,
    mut music: ResMut<MusicInput>,
    mut audio: NonSendMut<Audio>,
    mut seen: Local<u32>,
) {
    let _trace = blackbox_trace::scope("Audio: car / traffic / music update");
    // A scene that went away takes its soundtrack with it, even when the next one has none.
    if *seen != host.scene_changes {
        *seen = host.scene_changes;
        audio.stop_music();
        audio.stop_traffic();
    }
    let car = host.scene.car_sound();
    audio.drive_car(car.as_ref(), time.delta_secs());
    audio.update_common(host.scene.take_common_sounds(), time.delta_secs());
    audio.trace.advance(time.delta_secs());
    let traffic = host.scene.take_traffic_sounds();
    if let Some(state) = &car {
        audio.update_traffic(traffic, state.view);
    }
    if car.is_none() {
        audio.stop_traffic();
    }
    // The helicopter's rotor follows the same car: it stops with the pause (no car state) and when the car is gone.
    let heli = host.scene.rotor_frame(time.delta_secs());
    match &car {
        Some(state) => audio.update_heli(heli, state.view),
        None => audio.stop_heli(),
    }
    // The police sirens follow the same car and stop with it.
    let sirens = host.scene.siren_frame(time.delta_secs());
    match &car {
        Some(state) => audio.update_sirens(sirens, state.view),
        None => audio.stop_sirens(),
    }
    // Each petrol-station blast of this frame plays its explosion.
    for _ in 0..host.scene.take_explosions() {
        audio.play_explosion();
    }
    // The sputter pops of this frame reach the scene, which lights the tail pipes for them.
    let pops = audio.take_sputter_pops();
    if pops > 0 {
        host.scene.note_sputters(pops);
    }
    // The pause menu freezes the car (no sound) but the game goes on, and so does the radio.
    let driving = car.is_some() || host.scene.paused();
    // The chase of the world reaches the music and the speech; the pursuit music takes the songs' place while a chase
    // is on and for a while after it.
    let chase = host.scene.pursuit_feed();
    audio.follow_chase(&mut music, chase.as_ref(), host.scene.pursuit_ended());
    let songs_free = audio.update_interactive(&music.state(), driving, time.delta_secs());
    audio.update_radio(driving, host.scene.radio_on_menus(), !songs_free);
    if let Some(clip) = host.scene.take_clip()
        && let Err(e) = audio.play_music(super::pcm::sound(&clip))
    {
        log::warn!("the scene's sound: {e}");
    }
}
