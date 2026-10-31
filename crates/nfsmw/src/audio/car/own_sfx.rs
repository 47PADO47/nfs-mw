//! A mod car's own effects: gear shifts, turbo whistle and blow-off, backfires and pops, played from the car's own
//! recordings (`[car_sound]`, docs/modding/car-sound.md, "Other sounds"). A car plays nothing that it does not
//! have: a missing list is silence for that effect, never another car's sound.
//!
//! The shifts take the place of the shared gear clunk when the car has them (a car with `shift_clunk = false` has no
//! clunk at all); the rest are the car's alone, since the shared banks have nothing for a car without a record of its
//! own. When they play is the game's own simple rules
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

mod gear;
pub mod logic;
mod overrun;
mod shared;

/// The level of the turbo whistle loops, a factor on their mix. The files are at a listening level of their own
/// (`nfsmw-2012-content/src/effects.rs`); next to the engine the whistle is a high, thin line, not a second engine, and
/// measured at the file level it was as loud as the whole engine and made the car sound like a hiss (centroid of 5 to
/// 10 kHz against 0.2 to 0.5 kHz for the cars of the game).
pub const WHINE_LEVEL: f32 = 0.08;

/// The level of the one-shots (shifts, blow-off, backfires, pops), a factor on the level each asks for. The files are
/// brought to a peak of 0.9 (`nfsmw-2012-content/src/effects.rs`), which is far above the gear clunk of the cars of the
/// game: a drive of six shifts peaked at -2.4 dB against -12.8 dB for the BMW M3 GTR.
pub const SHOT_LEVEL: f32 = 0.3;

/// The volume and pitch of the whistle change over this long, so a frame's step is not heard as a click.
const SMOOTH: Duration = Duration::from_millis(80);

/// Which list of one-shots.
const SHIFT_UP: usize = 0;
const SHIFT_DOWN: usize = 1;
const BLOWOFF: usize = 2;
const BACKFIRE: usize = 3;
const POPS: usize = 4;
/// The names of those lists, for the audio trace (`NFSMW_AUDIO_TRACE`).
const LIST_NAMES: [&str; 5] = ["shift_up", "shift_down", "turbo_blowoff", "backfire", "pops"];

/// What a frame tells the effects.
pub struct SfxFrame {
    pub throttle: f32,
    pub rpm_pct: f32,
    /// The throttle was lifted at high engine speed since the last frame.
    pub lift_off: bool,
    pub dt: f32,
    /// Seconds to hold a sound that starts now (the frame's place within the render step).
    pub delay: f32,
    /// The gear id of the frame (reverse 0, neutral 1, first 2), for a car whose shifts follow the gear number.
    pub gear: i32,
}

/// The effects of a car that is playing.
pub struct OwnSfx {
    /// The one-shots by list ([`SHIFT_UP`] and so on), each list a set of variants.
    shots: [Vec<StaticSoundData>; 5],
    /// The whistle loops, playing silent until the turbo spools.
    whine: Vec<StaticSoundHandle>,
    spool: f32,
    rng: logic::Rng,
    /// The body hits and scrapes from the recordings the mod's cars share.
    shared: shared::SharedSfx,
    /// The backfire and the pops of the lift-offs, waiting to light the tail pipes. They are decided whether or not the car
    /// has a recording for them: the flames do not need the sound.
    bangs: logic::Bangs,
    /// The car makes no gear clunk of the game's class-wide bank: a shift it has no recording for is silent.
    no_clunk: bool,
    /// The accumulator backfire of `[car_sound.overrun]`; it replaces the backfire of a lift-off (the pops stay, with its chance).
    overrun: Option<overrun::OverrunDrive>,
    /// `shift_trigger = "gear"`: the shifts play at every change of the gear number ([`gear`]).
    shift_on_gear: bool,
    gear: gear::GearWatch,
}

impl OwnSfx {
    /// The bangs of the car's lift-offs that came due since the last call (the tail pipes flame for them).
    pub fn take_bangs(&mut self) -> u32 {
        self.bangs.take()
    }

    /// One frame: take over the gear clunks from `commands` when the car has its own, follow the turbo, and play what a
    /// lifted throttle sets off.
    pub fn drive(&mut self, audio: &mut Audio, frame: &SfxFrame, commands: &mut Vec<SoundCommand>) {
        self.shared.tick(frame.dt);
        self.bangs.advance(frame.dt);
        let changed = self.gear.observe(frame.gear).filter(|_| self.shift_on_gear);
        commands.retain(|command| {
            let SoundCommand::Play { sound: SoundRef::GearClunk { up }, volume, .. } = *command else { return true };
            let list = if up { SHIFT_UP } else { SHIFT_DOWN };
            // A car that shifts by the gear number plays its recordings from the gear change, not from the clunk.
            if self.shift_on_gear && !self.shots[list].is_empty() {
                return false;
            }
            let played = self.shot(audio, list, volume, frame.delay);
            !(played || self.no_clunk)
        });
        if let Some(up) = changed {
            self.shot(audio, if up { SHIFT_UP } else { SHIFT_DOWN }, gear::GEAR_SHOT_VOLUME, frame.delay);
        }
        if !self.whine.is_empty() || !self.shots[BLOWOFF].is_empty() {
            self.follow_turbo(frame);
        }
        self.lift_off_effects(audio, frame);
        self.overrun_effects(audio, frame);
    }

    /// The accumulator backfire (`[car_sound.overrun]`): a bang plays a `backfire` recording at the bang's level and lights
    /// the tail pipes when it is strong enough, the pops after it follow; a burning flame lights the pipes.
    fn overrun_effects(&mut self, audio: &mut Audio, frame: &SfxFrame) {
        let Some(overrun) = self.overrun.as_mut() else { return };
        let play = overrun.step(frame.dt, frame.throttle, frame.rpm_pct, &mut self.rng);
        if play.flame_pulse {
            self.bangs.queue(0.0);
        }
        let Some(bang) = play.bang else { return };
        self.shot(audio, BACKFIRE, bang.volume, frame.delay);
        if bang.pipes {
            self.bangs.queue(0.0);
        }
        for pop in bang.pops {
            self.shot(audio, POPS, pop.volume, frame.delay + pop.delay);
            if bang.pipes {
                self.bangs.queue(pop.delay);
            }
        }
    }

    /// Spool the turbo and blend the whistle loops to it.
    fn follow_turbo(&mut self, frame: &SfxFrame) {
        self.spool = logic::spool_step(self.spool, frame.throttle, frame.rpm_pct, frame.dt);
        let tween = Tween { duration: SMOOTH, ..Tween::default() };
        let mix = logic::whine_mix(self.whine.len(), self.spool);
        for (handle, layer) in self.whine.iter_mut().zip(mix) {
            handle.set_volume(decibels(layer.gain * WHINE_LEVEL), tween);
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
        // The overrun table decides the backfire and what follows it; the lift-off keeps the turbo's blow-off and, with the
        // table's own chance, the pops (the crackle recordings), which light the pipes.
        if let Some(overrun) = &self.overrun {
            if self.rng.unit() < overrun.lift_pop_chance(frame.rpm_pct) {
                for pop in logic::pops_after_bang(&mut self.rng) {
                    self.shot(audio, POPS, pop.volume, frame.delay + pop.delay);
                    self.bangs.queue(pop.delay);
                }
            }
            return;
        }
        if let Some(volume) = lift.backfire {
            self.shot(audio, BACKFIRE, volume, frame.delay);
            self.bangs.queue(0.0);
        }
        for pop in lift.pops {
            self.shot(audio, POPS, pop.volume, frame.delay + pop.delay);
            self.bangs.queue(pop.delay);
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
        if audio.trace.enabled() {
            audio.trace.note(
                "own_sfx_shot",
                &format!("{} variant {variant} volume {volume:.2} delay {delay:.2}", LIST_NAMES[list]),
            );
        }
        let data = self.shots[list][variant]
            .clone()
            .volume(decibels(volume * SHOT_LEVEL))
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
    /// out, and the log says why. A car with nothing to play still gets the effects: its lift-offs light its tail pipes.
    pub(super) fn start_own_sfx(&mut self, name: &str, effects: &CarEffects) -> OwnSfx {
        // The car's own pops, else the ones its sound graph names in the mod's shared recordings.
        let pops = if effects.pops.is_empty() { &effects.shared.pops } else { &effects.pops };
        let shots = [
            self.read_shots(name, "shift_up", effects.shift_files(true)),
            self.read_shots(name, "shift_down", effects.shift_files(false)),
            self.read_shots(name, "turbo_blowoff", &effects.turbo_blowoff),
            self.read_shots(name, "backfire", &effects.backfire),
            self.read_shots(name, "pops", pops),
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
        let shared = self.read_shared(name, &effects.shared);
        let seed = name.bytes().fold(0x9e37_79b9u32, |seed, byte| seed.rotate_left(5) ^ u32::from(byte));
        // The engine speed over the redline needs the span the car's `0 to 1` engine speed covers: its own, or the built-in's.
        let (idle_rpm, redline_rpm) = nfsmw_data::car::mods::engine_sound(name).map_or_else(
            || {
                let built_in = super::own_engine::built_in();
                (built_in.idle_rpm, built_in.redline_rpm)
            },
            |sound| (sound.idle_rpm, sound.redline_rpm),
        );
        let overrun =
            effects.overrun.as_ref().map(|table| overrun::OverrunDrive::new(table, idle_rpm, redline_rpm, seed));
        OwnSfx {
            shots,
            whine,
            spool: 0.0,
            rng: logic::Rng::new(seed),
            shared,
            bangs: logic::Bangs::default(),
            no_clunk: effects.no_clunk,
            overrun,
            shift_on_gear: effects.shift_on_gear,
            gear: gear::GearWatch::default(),
        }
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

impl OwnSfx {
    /// Play a body hit from the shared recordings. False when the car has none for this hit, so the game's own sound plays.
    pub fn shared_impact(&mut self, audio: &mut Audio, event: &crate::audio::CarEvent, delay: f32) -> bool {
        self.shared.impact(audio, event, delay)
    }

    /// Whether the car scrapes with the shared recordings: the contacts then start no scrape of the game's.
    pub fn has_shared_scrapes(&self) -> bool {
        self.shared.has_scrapes()
    }

    /// One step of the shared scrape, at the strongest scrape strength the contacts reported (0 for none).
    pub fn shared_scrape(&mut self, audio: &mut Audio, level: f32, dt: f32, delay: f32) {
        self.shared.scrape(audio, level, dt, delay);
    }
}
