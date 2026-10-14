//! The dynamic mixer for the car sound: reads `SOUND/MIXMAPS/MAPOUTPUT.mxb`, publishes the car's state to it
//! every frame and hands back the levels of the engine and the effects. Specs: `docs/specs/dynamic-mixer.md`
//! (the evaluator, `blackbox-mixmap`) and `docs/specs/car-sound-mixer.md` (this wiring).

pub(super) mod ambient;
mod authored;
pub(super) mod collision;
mod feed;
mod ids;
mod levels;
mod power;
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
    map: Arc<MixMap>,
    view: super::AudioView,
    tyres: crate::audio::car::tyres::TyreMixerSignals,
    environment: super::environment::EnvironmentController,
    environment_values: super::environment::EnvironmentValues,
}

impl CarMixer {
    #[cfg(test)]
    pub(in crate::audio) fn mute_continuous_effects(&mut self) {
        for object in [object::WEATHER, object::WIND, object::ROAD, object::SKIDS] {
            self.mixer.attach(player_object(object), false);
        }
    }
    pub(in crate::audio) fn set_landing(&mut self, landing: Option<blackbox_carsound::Landing>) {
        self.mixer
            .set_input(ids::player_object_input(object::BOTTOM_OUT, 1), if landing.is_some() { 32767 } else { 0 });
        if let Some(landing) = landing {
            self.mixer
                .set_input(ids::player_object_input(object::BOTTOM_OUT, 3), (landing.magnitude * 127.0) as i32 * 256);
        }
    }
    pub(in crate::audio) fn landing_level(&self) -> (f32, u16) {
        let object = player_object(object::BOTTOM_OUT);
        (self.mixer.volume(object, 1).unwrap_or(0.0), self.mixer.azimuth(object, 0).unwrap_or(0))
    }
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
        let mut mixer = Mixer::new(map.clone(), &instances);
        let engine = if dual { object::ENGINE_DUAL } else { object::ENGINE_SINGLE };
        for o in [engine, object::SHIFT, object::TURBO, object::NITROUS, object::SPARKS, object::SKIDS] {
            mixer.attach(player_object(o), true);
        }
        for o in [object::ROAD, object::WIND, object::WEATHER, object::BOTTOM_OUT] {
            mixer.attach(player_object(o), true);
        }
        Self {
            mixer,
            dual,
            map,
            view: Default::default(),
            tyres: Default::default(),
            environment: Default::default(),
            environment_values: Default::default(),
        }
    }

    /// Publishes `frame`, advances by `dt` seconds and returns the levels for the next frame.
    pub fn update(&mut self, dt: f32, frame: &Frame<'_>) -> Levels {
        feed::publish(&mut self.mixer, frame, &self.view);
        for (index, value) in self.tyres.update(dt, frame.input).into_iter().enumerate() {
            self.mixer.set_input(ids::player_object_input(object::SKIDS, index as u8), value);
        }
        self.environment_values = self.environment.update(dt, self.view.environment);
        for (index, value) in self.view.environment.mixer_inputs(self.environment_values).into_iter().enumerate() {
            self.mixer.set_input(ids::player_controller(6, index as u8), value);
        }
        self.mixer.process(dt);
        self.mixer.set_input(ids::player_object_input(object::BOTTOM_OUT, 1), 0);
        let mut levels = Levels::read(&self.mixer, self.dual);
        levels.environment = self.environment_values;
        levels.engine_distance_filter =
            super::environment::distance_filter(self.view.distances(self.view.car_position)[1]);
        levels
    }

    pub fn set_view(&mut self, view: super::AudioView) {
        self.mixer.set_camera(view.camera_state);
        self.view = view;
    }

    pub(super) fn collision(&self) -> collision::CollisionMixer {
        collision::CollisionMixer::new(self.map.clone())
    }
}
