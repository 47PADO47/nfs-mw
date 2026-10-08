//! Playing what the effects mixer asks for: one-shots, loops that keep their voice between frames, and the
//! collision stitches.

use std::sync::Arc;
use std::time::Duration;

use blackbox_carsound::{ImpactRequest, SoundCommand, SoundRef};
use kira::sound::static_sound::StaticSoundData;
use kira::{PlaybackRate, StartTime, Tween};
use nfsmw_data::car::physics::SurfaceTable;
use nfsmw_data::sound::{Stitch, collision_stitches};

use super::car::{CarAudio, CarEvent};
use super::refs::{self, COLLISION_BANK};
use super::volume::decibels;
use super::{Audio, Group};

/// A loop changes its volume and pitch over this long, so a frame's step is not heard as a click.
const SMOOTH: Duration = Duration::from_millis(60);
/// A loop that stops fades out over this long.
const FADE_OUT: Duration = Duration::from_millis(120);

fn tween(duration: Duration) -> Tween {
    Tween { duration, ..Tween::default() }
}

impl Audio {
    /// The collision stitches of `InGameB.bun`, read once (empty when the file cannot be read).
    fn stitches(&mut self) -> Arc<Vec<Stitch>> {
        if let Some(stitches) = &self.stitches {
            return stitches.clone();
        }
        let stitches = match nfsmw_data::read_unwrapped(&self.dir, "GLOBAL/InGameB.bun") {
            Ok(bytes) => collision_stitches(&bytes),
            Err(e) => {
                log::warn!("no collision stitches: {e:#}");
                Vec::new()
            }
        };
        let stitches = Arc::new(stitches);
        self.stitches = Some(stitches.clone());
        stitches
    }

    /// The sound `sound` of this car as playable data, `None` (and a note in the log, once) when it has none.
    fn effect_data(&mut self, car: &CarAudio, sound: SoundRef) -> Option<(StaticSoundData, Group)> {
        let target = refs::resolve(&car.sound, sound)?;
        match self.bank_sound(&target.bank, target.index) {
            Ok(data) => Some((data, target.group)),
            Err(e) => {
                if self.missing.insert((target.bank.clone(), target.index)) {
                    log::warn!("no sound for {sound:?}: {e}");
                }
                None
            }
        }
    }

    /// Carry out one command of the effects mixer.
    pub(super) fn apply(&mut self, car: &mut CarAudio, command: SoundCommand) {
        match command {
            SoundCommand::Play { sound, volume, pitch } => {
                let Some((data, group)) = self.effect_data(car, sound) else { return };
                let data = data.volume(decibels(volume)).playback_rate(f64::from(pitch));
                if let Err(e) = self.play(group, data) {
                    log::debug!("{sound:?}: {e}");
                }
            }
            SoundCommand::SetLoop { id, sound, volume, pitch } => {
                if let Some((playing, handle)) = car.loops.get_mut(&id) {
                    if *playing == sound {
                        handle.set_volume(decibels(volume), tween(SMOOTH));
                        handle.set_playback_rate(PlaybackRate(f64::from(pitch)), tween(SMOOTH));
                        return;
                    }
                    handle.stop(tween(FADE_OUT));
                    car.loops.remove(&id);
                }
                let Some((data, group)) = self.effect_data(car, sound) else { return };
                // A loop without loop points repeats whole.
                let data = match data.settings.loop_region {
                    Some(_) => data,
                    None => {
                        let seconds = data.frames.len() as f64 / f64::from(data.sample_rate.max(1));
                        data.loop_region(0.0..seconds)
                    }
                };
                let data = data.volume(decibels(volume)).playback_rate(f64::from(pitch));
                match self.play(group, data) {
                    Ok(handle) => {
                        log::debug!("loop {id:?} starts: {sound:?}");
                        car.loops.insert(id, (sound, handle));
                    }
                    Err(e) => log::debug!("{sound:?}: {e}"),
                }
            }
            SoundCommand::StopLoop(id) => {
                if let Some((_, mut handle)) = car.loops.remove(&id) {
                    handle.stop(tween(FADE_OUT));
                }
            }
        }
    }

    /// A hit: pick the collection of the car for the kind of hit and surface, the stitch by magnitude, and play
    /// its pieces one after the other.
    pub(super) fn play_impact(&mut self, car: &mut CarAudio, event: &CarEvent) {
        let default = SurfaceTable::hash_of("default");
        let Some(sound) = car.sound.collision.pick(event.kind, event.surface, default, event.front) else { return };
        let lengths = std::array::from_fn(|i| sound.levels[i].len() as u32);
        let volumes = std::array::from_fn(|i| (sound.volumes[i] * 32767.0) as u32);
        let tuning =
            blackbox_carsound::CollisionTuning { level_lengths: lengths, volumes, stream_thresholds: Vec::new() };
        let request = ImpactRequest { magnitude: event.magnitude, light_wall: event.smackable, tuning: &tuning };
        let Some(play) = car.effects.impact(&request) else { return };
        log::debug!("{:?} impact {:.2} -> level {} sample {}", event.kind, event.magnitude, play.level, play.index);
        let Some(&id) = sound.levels[play.level].get(play.index) else { return };
        let stitches = self.stitches();
        let Some(stitch) = stitches.get(id as usize) else { return };
        let mut start = 0.0f64;
        for piece in &stitch.pieces {
            let index = usize::from(piece.sample) + 1;
            let Ok(data) = self.bank_sound(COLLISION_BANK, index) else { continue };
            let volume = (stitch.volume * piece.volume * play.volume).clamp(0.0, 1.0);
            let rate = f64::from(data.sample_rate.max(1));
            let data =
                data.volume(decibels(volume)).start_time(StartTime::Delayed(Duration::from_secs_f64(start.max(0.0))));
            if let Err(e) = self.play(Group::Sfx, data) {
                log::debug!("collision sound: {e}");
                return;
            }
            start += f64::from(piece.advance) / rate;
        }
    }
}
