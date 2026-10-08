//! The dynamic mixer for the car sound: reads `SOUND/MIXMAPS/MAPOUTPUT.mxb`, publishes the car's state to it
//! every frame and hands back the levels of the engine and the effects. Specs: `docs/specs/dynamic-mixer.md`
//! (the evaluator, `blackbox-mixmap`) and `docs/specs/car-sound-mixer.md` (this wiring).

mod feed;
mod ids;
mod levels;
mod scale;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use blackbox_mixmap::{MixMap, Mixer};
use game_install::GameDir;

pub use self::feed::Frame;
use self::ids::{MAIN, PLAYER, object, player_object};
pub use self::levels::Levels;
pub use self::scale::scale as scale_command;

/// The map for circuit races and free roam.
const MAP: &str = "SOUND/MIXMAPS/MAPOUTPUT.mxb";

/// The mixer of the player's car.
pub struct CarMixer {
    mixer: Mixer,
    dual: bool,
}

impl CarMixer {
    /// Reads the map from the install. `dual` says the car plays the two-loop engine (object 2), else the
    /// single one (object 1).
    pub fn load(dir: &GameDir, dual: bool) -> Result<Self, String> {
        let bytes = dir.read(MAP).map_err(|e| format!("{MAP}: {e:#}"))?;
        let map = MixMap::parse(&bytes).map_err(|e| format!("{MAP}: {e}"))?;
        Ok(Self::new(Arc::new(map), dual))
    }

    /// A mixer over `map`: one main state and the player's car.
    pub fn new(map: Arc<MixMap>, dual: bool) -> Self {
        let mut instances = [0u8; 13];
        instances[usize::from(MAIN)] = 1;
        instances[usize::from(PLAYER)] = 1;
        let mut mixer = Mixer::new(map, &instances);
        let engine = if dual { object::ENGINE_DUAL } else { object::ENGINE_SINGLE };
        for o in [engine, object::SHIFT, object::TURBO, object::NITROUS, object::SPARKS, object::SKIDS] {
            mixer.attach(player_object(o), true);
        }
        for o in [object::ROAD, object::WIND, object::BOTTOM_OUT] {
            mixer.attach(player_object(o), true);
        }
        Self { mixer, dual }
    }

    /// Publishes `frame`, advances by `dt` seconds and returns the levels for the next frame.
    pub fn update(&mut self, dt: f32, frame: &Frame<'_>) -> Levels {
        feed::publish(&mut self.mixer, frame);
        self.mixer.process(dt);
        Levels::read(&self.mixer, self.dual)
    }
}
