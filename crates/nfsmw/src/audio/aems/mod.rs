//! The sample layer of the car sound: the engine's looped samples and the sputters (backfire pops). Both are
//! modules of the AEMS banks that `blackbox-aems` runs; this module feeds them the car's state every tick and plays
//! the voices they ask for. Specs: `docs/specs/aems.md` (the interpreter) and `docs/specs/engine-sound-aems.md`
//! (what the car sound feeds them).

mod host;
mod kira_out;
#[cfg(test)]
mod limiter_tests;
mod params;
#[cfg(test)]
pub(super) mod soft;
#[cfg(test)]
mod tests;
mod timeline;

use blackbox_aems::{Instance, ModuleBank};
use blackbox_carsound::EngineOutput;
use nfsmw_data::sound::{CarSound, ENGINE_DIR};

pub use self::host::{PartHost, Sampler, Voices};
pub use self::kira_out::{DelayedSampler, EffectSampler, KiraSampler};
use super::Audio;
use super::mixer::Levels;

/// The update tick of the modules, ms. The original uses the sound system's variable timer; this is the car
/// sound's own 60 Hz.
const TICK_MS: f32 = 1000.0 / 60.0;
/// Most ticks run in one frame; excess elapsed time remains owed.
const MAX_TICKS: u32 = 4;
/// The `car id` the sputter is told it belongs to (it only tells its output object apart).
const SPUTTER_ID: u32 = 1;

/// One running module and its voices.
pub(super) struct Part {
    instance: Instance,
    /// The module's bank, relative to `SOUND/`.
    bank: String,
    voices: Voices,
}

impl Part {
    pub(super) fn load(audio: &mut Audio, bank: &str, class: &str, seed: u32) -> Result<Part, String> {
        let result: Result<Part, String> = (|| {
            let bytes = audio.bank_bytes(bank)?;
            let parsed = ModuleBank::parse(&bytes).map_err(|e| format!("{bank}: {e}"))?;
            let module = parsed.module_of(class).ok_or_else(|| format!("{bank} has no {class} module"))?;
            let instance = Instance::new(&parsed, module, seed).map_err(|e| format!("{bank}: {e}"))?;
            let mut voices = Voices::default();
            voices.trace = Some(host::ModuleTrace::new(audio.trace.clone(), bank, class));
            Ok(Part { instance, bank: bank.to_owned(), voices })
        })();
        if let Err(error) = &result {
            audio.trace.record(crate::audio::trace::Record {
                event: "module_failure",
                bank,
                class,
                logical_ms: audio.trace.now_ms(),
                success: Some(false),
                detail: error,
                ..Default::default()
            });
        }
        result
    }

    /// Sets the class data and runs one tick; `false` when the module failed (it is then left alone).
    pub(super) fn tick(&mut self, sampler: &mut dyn Sampler, params: &[i32], makeup: f32) -> bool {
        self.voices.trace_tick(params);
        self.instance.set_class_data(params);
        let mut host = PartHost { sampler, bank: &self.bank, voices: &mut self.voices, makeup };
        match self.instance.update(TICK_MS, &mut host) {
            Ok(()) => true,
            Err(e) => {
                log::warn!("sample layer {}: {e}", self.bank);
                false
            }
        }
    }

    pub(super) fn stop(&mut self) {
        self.voices.trace_event("module_drop", None, None, None, "");
        for voice in self.voices.slots.iter_mut().flatten() {
            voice.stop();
        }
        self.voices.slots.clear();
    }
}

/// One authored continuous effect, using the same interpreter/host as the engine modules.
pub(super) struct EffectModule {
    part: Part,
    timeline: timeline::Timeline,
    release_at: Option<f64>,
    has_ticked: bool,
    discarded: bool,
}

impl EffectModule {
    pub fn release(&mut self) {
        if !self.has_ticked {
            // No graph update has reached the host, so no voice or child object exists to release.
            self.discarded = true;
            return;
        }
        self.release_at = Some(self.timeline.submitted());
        self.part.voices.trace_event("module_release", None, None, None, "");
    }
    #[cfg(test)]
    pub fn voice_counts(&self) -> (u32, usize) {
        self.part.voices.counts()
    }
    pub fn finished(&self) -> bool {
        self.discarded || self.part.instance.is_destroyed()
    }
    pub fn load(audio: &mut Audio, bank: &str, class: &str, seed: u32) -> Result<Self, String> {
        Ok(Self {
            part: Part::load(audio, bank, class, seed)?,
            timeline: timeline::Timeline::default(),
            release_at: None,
            has_ticked: false,
            discarded: false,
        })
    }
    pub fn update(&mut self, sampler: &mut dyn Sampler, dt: f32, params: &[i32]) -> bool {
        if self.discarded {
            return true;
        }
        if !dt.is_finite() {
            return true;
        }
        self.timeline.push(dt, params);
        for _ in 0..12 {
            let Some((at, words)) = self.timeline.next() else { break };
            if self.release_at.is_some_and(|when| at + 1e-7 >= when) {
                self.part.instance.request_destroy();
                self.release_at = None;
            }
            self.has_ticked = true;
            if !self.part.tick(sampler, &words, 1.0) || self.part.voices.failed {
                return false;
            }
        }
        if self.timeline.pending() >= 1.0 / 60.0 {
            self.part.voices.trace_event(
                "clock_debt",
                None,
                None,
                None,
                &format!("{} seconds", self.timeline.pending()),
            );
        }
        true
    }
}

impl Drop for Part {
    fn drop(&mut self) {
        self.stop();
    }
}

/// What a frame gives the layer.
pub struct Feed<'a> {
    pub engine: &'a EngineOutput,
    pub levels: &'a Levels,
}

/// The engine module and the sputter of one car.
pub struct AemsLayer {
    engine: Option<Part>,
    sputter: Option<Part>,
    class: u32,
    sputter_volume: i32,
    /// Milliseconds of time not yet run as ticks.
    timeline: timeline::Timeline,
    synth_frequency: Option<f32>,
}

fn bank_path(name: &str) -> String {
    format!("{}/{name}", ENGINE_DIR.strip_prefix("SOUND/").unwrap_or(ENGINE_DIR))
}

impl AemsLayer {
    /// Loads the modules of the car's engine bank and sweetener bank. An error when the engine module is missing;
    /// a car without a sputter just has none.
    pub fn load(audio: &mut Audio, car: &CarSound) -> Result<Self, String> {
        let engine = &car.engine;
        if engine.bank_main.is_empty() {
            return Err("the engine set has no sample-layer bank".into());
        }
        let main = Part::load(audio, &bank_path(&engine.bank_main), "CAR", 1)?;
        let sputter = engine.sweet_banks.first().filter(|n| !n.is_empty()).and_then(|name| {
            Part::load(audio, &bank_path(name), "CAR_Sputter", 2).map_err(|e| log::warn!("no sputters: {e}")).ok()
        });
        Ok(Self {
            engine: Some(main),
            sputter,
            class: engine.car_id,
            sputter_volume: engine.sputter_volume,
            timeline: timeline::Timeline::default(),
            synth_frequency: None,
        })
    }

    /// Whether the sputter's output object reports a volume (the spark chatter's mixer input).
    pub fn sparks(&self) -> bool {
        let object = self.sputter.as_ref().and_then(|p| p.voices.object.as_ref());
        object.and_then(|params| params.first()).is_some_and(|&volume| volume != 0)
    }

    /// The pops the sputter has played since the last call: each is a voice its module started.
    pub fn take_pops(&mut self) -> u32 {
        self.sputter.as_mut().map_or(0, |p| p.voices.take_started())
    }

    /// Whether the engine sample module and every requested voice are usable. A missing or failed engine
    /// cannot replace the Ginsu loops at the limiter; a failed sputter does not affect that replacement.
    pub fn engine_available(&self) -> bool {
        self.engine.as_ref().is_some_and(|p| !p.instance.is_destroyed() && !p.voices.failed)
    }

    pub fn set_synth_frequency(&mut self, frequency: f32) {
        self.synth_frequency = (frequency.is_finite() && frequency > 0.0).then_some(frequency);
    }

    /// The number of voices playing: the engine's, then the sputter's.
    #[cfg(test)]
    pub fn voices(&self) -> (usize, usize) {
        let count = |p: &Option<Part>| p.as_ref().map_or(0, |p| p.voices.slots.iter().flatten().count());
        (count(&self.engine), count(&self.sputter))
    }

    /// Runs the ticks `dt` seconds make with the parameters of this frame.
    pub fn update(&mut self, sampler: &mut dyn Sampler, dt: f32, feed: &Feed<'_>) {
        if !dt.is_finite() {
            return;
        }
        let engine = params::engine_with_frequency(self.class, feed.engine, feed.levels, self.synth_frequency);
        let sputter = params::sputter(self.class, SPUTTER_ID, self.sputter_volume, feed.engine, feed.levels);
        let mut words = engine.to_vec();
        words.extend(sputter);
        words.push(feed.levels.makeup.to_bits() as i32);
        self.timeline.push(dt, &words);
        for _ in 0..MAX_TICKS {
            let Some((_, words)) = self.timeline.next() else { break };
            let makeup = f32::from_bits(words[params::CAR_PARAMS + params::SPUTTER_PARAMS] as u32);
            if let Some(part) = self.engine.as_mut()
                && !part.tick(sampler, &words[..params::CAR_PARAMS], makeup)
            {
                self.engine = None;
            }
            if let Some(part) = self.sputter.as_mut()
                && !part.tick(sampler, &words[params::CAR_PARAMS..params::CAR_PARAMS + params::SPUTTER_PARAMS], makeup)
            {
                self.sputter = None;
            }
        }
    }
}
