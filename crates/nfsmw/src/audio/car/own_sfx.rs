//! A mod car's own effects: gear shifts, turbo whistle and blow-off, backfires and pops, played from the car's own
//! recordings (`[car_sound]`, docs/modding/car-sound.md, "Other sounds"). A car plays nothing that it does not
//! have: a missing list is silence for that effect, never another car's sound.
//!
//! The shifts take the place of the shared gear clunk when the car has them; the rest are the car's alone, since the
//! shared banks have nothing for a car without a record of its own. When they play is the game's own simple rules
//! ([`logic`]): the shift when the engine mixer asks for a gear clunk, the rest when the throttle is lifted at high
//! engine speed or, for the whistle, as the turbo spools.

use std::time::Duration;

use blackbox_carsound::{SoundCommand, SoundRef};
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::{PlaybackRate, StartTime, Tween};
use nfsmw_data::car::mods::CarEffects;

use super::own_engine::parse_wav;
use crate::audio::volume::decibels;
use crate::audio::{Audio, Group, pcm};

pub mod logic;

/// The volume and pitch of the whistle change over this long, so a frame's step is not heard as a click.
const SMOOTH: Duration = Duration::from_millis(80);

/// Which list of one-shots.
const SHIFT_UP: usize = 0;
const SHIFT_DOWN: usize = 1;
const BLOWOFF: usize = 2;
const BACKFIRE: usize = 3;
const POPS: usize = 4;

/// What a frame tells the effects.
pub struct SfxFrame {
    pub throttle: f32,
    pub rpm_pct: f32,
    /// The throttle was lifted at high engine speed since the last frame.
    pub lift_off: bool,
    pub dt: f32,
    /// Seconds to hold a sound that starts now (the frame's place within the render step).
    pub delay: f32,
}

/// The effects of a car that is playing.
pub struct OwnSfx {
    /// The one-shots by list ([`SHIFT_UP`] and so on), each list a set of variants.
    shots: [Vec<StaticSoundData>; 5],
    /// The whistle loops, playing silent until the turbo spools.
    whine: Vec<StaticSoundHandle>,
    spool: f32,
    rng: logic::Rng,
}

impl OwnSfx {
    /// Whether the car has any sound to play.
    pub fn is_empty(&self) -> bool {
        self.shots.iter().all(Vec::is_empty) && self.whine.is_empty()
    }

    /// One frame: take over the gear clunks from `commands` when the car has its own, follow the turbo, and play what a
    /// lifted throttle sets off.
    pub fn drive(&mut self, audio: &mut Audio, frame: &SfxFrame, commands: &mut Vec<SoundCommand>) {
        commands.retain(|command| {
            let SoundCommand::Play { sound: SoundRef::GearClunk { up }, volume, .. } = *command else { return true };
            let list = if up { SHIFT_UP } else { SHIFT_DOWN };
            !self.shot(audio, list, volume, frame.delay)
        });
        if !self.whine.is_empty() || !self.shots[BLOWOFF].is_empty() {
            self.follow_turbo(frame);
        }
        self.lift_off_effects(audio, frame);
    }

    /// Spool the turbo and blend the whistle loops to it.
    fn follow_turbo(&mut self, frame: &SfxFrame) {
        self.spool = logic::spool_step(self.spool, frame.throttle, frame.rpm_pct, frame.dt);
        let tween = Tween { duration: SMOOTH, ..Tween::default() };
        let mix = logic::whine_mix(self.whine.len(), self.spool);
        for (handle, layer) in self.whine.iter_mut().zip(mix) {
            handle.set_volume(decibels(layer.gain), tween);
            handle.set_playback_rate(PlaybackRate(f64::from(layer.rate)), tween);
        }
    }

    fn lift_off_effects(&mut self, audio: &mut Audio, frame: &SfxFrame) {
        if !frame.lift_off {
            return;
        }
        let lift = logic::lift_off(self.spool, frame.rpm_pct, &mut self.rng);
        if let Some(volume) = lift.blowoff
            && self.shot(audio, BLOWOFF, volume, frame.delay)
        {
            self.spool *= logic::SPOOL_AFTER_BLOWOFF;
        }
        if let Some(volume) = lift.backfire {
            self.shot(audio, BACKFIRE, volume, frame.delay);
        }
        for pop in lift.pops {
            self.shot(audio, POPS, pop.volume, frame.delay + pop.delay);
        }
    }

    /// Play a variant of list `list` at `volume`, `delay` seconds from now. False (and nothing played) when the list is
    /// empty, so the caller can leave the sound to the shared one.
    fn shot(&mut self, audio: &mut Audio, list: usize, volume: f32, delay: f32) -> bool {
        let count = self.shots[list].len();
        if count == 0 {
            return false;
        }
        let delay = if delay.is_finite() { delay.max(0.0) } else { 0.0 };
        let variant = self.rng.below(count);
        let data = self.shots[list][variant]
            .clone()
            .volume(decibels(volume))
            .start_time(StartTime::Delayed(Duration::from_secs_f32(delay)));
        if let Err(e) = audio.play(Group::Sfx, data) {
            log::debug!("a car effect: {e}");
        }
        true
    }
}

impl Drop for OwnSfx {
    /// A kira handle that is dropped leaves its sound playing, so the whistle is stopped here.
    fn drop(&mut self) {
        let fade = Tween { duration: Duration::from_millis(120), ..Tween::default() };
        for handle in &mut self.whine {
            handle.stop(fade);
        }
    }
}

impl Audio {
    /// Read the recordings of mod car `name` and start its whistle loops, silent. A file that cannot be read is left
    /// out, and the log says why; `None` when nothing is left to play.
    pub(super) fn start_own_sfx(&mut self, name: &str, effects: &CarEffects) -> Option<OwnSfx> {
        let shots = [
            self.read_shots(name, "shift_up", effects.shift_files(true)),
            self.read_shots(name, "shift_down", effects.shift_files(false)),
            self.read_shots(name, "turbo_blowoff", &effects.turbo_blowoff),
            self.read_shots(name, "backfire", &effects.backfire),
            self.read_shots(name, "pops", &effects.pops),
        ];
        let mut whine = Vec::new();
        for file in &effects.turbo_whine {
            let started = self.read_wav(file);
            let started = started.and_then(|pcm| self.play(Group::Engine, pcm::sound(&pcm).volume(decibels(0.0))));
            match started {
                Ok(handle) => whine.push(handle),
                Err(e) => log::warn!("{name}: turbo_whine {file} is not used: {e}"),
            }
        }
        let seed = name.bytes().fold(0x9e37_79b9u32, |seed, byte| seed.rotate_left(5) ^ u32::from(byte));
        let sfx = OwnSfx { shots, whine, spool: 0.0, rng: logic::Rng::new(seed) };
        (!sfx.is_empty()).then_some(sfx)
    }

    fn read_shots(&mut self, name: &str, what: &str, files: &[String]) -> Vec<StaticSoundData> {
        let mut sounds = Vec::with_capacity(files.len());
        for file in files {
            match self.read_wav(file) {
                Ok(mut pcm) => {
                    pcm.loop_range = None;
                    sounds.push(pcm::sound(&pcm));
                }
                Err(e) => log::warn!("{name}: {what} {file} is not used: {e}"),
            }
        }
        sounds
    }

    fn read_wav(&mut self, file: &str) -> Result<ea_audio::Pcm, String> {
        let bytes = self.dir.read(file).map_err(|e| format!("{e:#}"))?;
        parse_wav(&bytes)
    }
}
