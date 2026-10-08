//! The sample layer of the car sound: the engine's looped samples and the sputters (backfire pops). Both are
//! modules of the AEMS banks that `blackbox-aems` runs; this module feeds them the car's state every tick and plays
//! the voices they ask for. Specs: `docs/specs/aems.md` (the interpreter) and `docs/specs/engine-sound-aems.md`
//! (what the car sound feeds them).

mod host;
mod kira_out;
mod params;
#[cfg(test)]
pub(super) mod soft;
#[cfg(test)]
mod tests;

use blackbox_aems::{Instance, ModuleBank};
use blackbox_carsound::EngineOutput;
use nfsmw_data::sound::{CarSound, ENGINE_DIR};

pub use self::host::{PartHost, Sampler, Voices};
pub use self::kira_out::KiraSampler;
use super::Audio;
use super::mixer::Levels;

/// The update tick of the modules, ms. The original uses the sound system's variable timer; this is the car
/// sound's own 60 Hz.
const TICK_MS: f32 = 1000.0 / 60.0;
/// Most ticks run in one frame; a longer frame drops the rest.
const MAX_TICKS: u32 = 4;
/// The `car id` the sputter is told it belongs to (it only tells its output object apart).
const SPUTTER_ID: u32 = 1;

/// One running module and its voices.
struct Part {
    instance: Instance,
    /// The module's bank, relative to `SOUND/`.
    bank: String,
    voices: Voices,
}

impl Part {
    fn load(audio: &mut Audio, bank: &str, class: &str, seed: u32) -> Result<Part, String> {
        let bytes = audio.bank_bytes(bank)?;
        let parsed = ModuleBank::parse(&bytes).map_err(|e| format!("{bank}: {e}"))?;
        let module = parsed.module_of(class).ok_or_else(|| format!("{bank} has no {class} module"))?;
        let instance = Instance::new(&parsed, module, seed).map_err(|e| format!("{bank}: {e}"))?;
        Ok(Part { instance, bank: bank.to_owned(), voices: Voices::default() })
    }

    /// Sets the class data and runs one tick; `false` when the module failed (it is then left alone).
    fn tick(&mut self, sampler: &mut dyn Sampler, params: &[i32], makeup: f32) -> bool {
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
    clock: f32,
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
            clock: 0.0,
        })
    }

    /// Whether the sputter's output object reports a volume (the spark chatter's mixer input).
    pub fn sparks(&self) -> bool {
        let object = self.sputter.as_ref().and_then(|p| p.voices.object.as_ref());
        object.and_then(|params| params.first()).is_some_and(|&volume| volume != 0)
    }

    /// The number of voices playing: the engine's, then the sputter's.
    #[cfg(test)]
    pub fn voices(&self) -> (usize, usize) {
        let count = |p: &Option<Part>| p.as_ref().map_or(0, |p| p.voices.slots.iter().flatten().count());
        (count(&self.engine), count(&self.sputter))
    }

    /// Runs the ticks `dt` seconds make with the parameters of this frame.
    pub fn update(&mut self, sampler: &mut dyn Sampler, dt: f32, feed: &Feed<'_>) {
        self.clock += dt.max(0.0) * 1000.0;
        let ticks = ((self.clock / TICK_MS) as u32).min(MAX_TICKS);
        self.clock = if self.clock / TICK_MS > MAX_TICKS as f32 { 0.0 } else { self.clock - ticks as f32 * TICK_MS };
        let makeup = feed.levels.makeup;
        let engine = params::engine(self.class, feed.engine, feed.levels);
        let sputter = params::sputter(self.class, SPUTTER_ID, self.sputter_volume, feed.engine, feed.levels);
        for _ in 0..ticks {
            if let Some(part) = self.engine.as_mut()
                && !part.tick(sampler, &engine, makeup)
            {
                self.engine = None;
            }
            if let Some(part) = self.sputter.as_mut()
                && !part.tick(sampler, &sputter, makeup)
            {
                self.sputter = None;
            }
        }
    }
}
