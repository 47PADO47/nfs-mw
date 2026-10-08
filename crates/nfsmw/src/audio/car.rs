//! The sound of the car being driven: the scene reports its telemetry each frame and the engine mixer
//! (`blackbox-carsound`) turns it into the Ginsu frequency and loop volumes the engine voice plays.

use blackbox_carsound::{CarInput, EngineMixer};

use super::tuning::tuning;
use super::{Audio, EngineHandle, EngineMix};

/// What the scene tells the sound each frame about the car it drives.
#[derive(Debug, Clone, PartialEq)]
pub struct CarSoundState {
    /// The car type (`BMWM3GTR`), which picks its sound set.
    pub car: String,
    pub input: CarInput,
}

/// The mixer and the voice of one car.
pub(super) struct CarAudio {
    car: String,
    mixer: EngineMixer,
    handle: EngineHandle,
}

impl Audio {
    /// Follow the scene's car: start its engine when it changes, feed the mixer, silence it when there is no car.
    pub fn drive_car(&mut self, state: Option<&CarSoundState>, dt: f32) {
        let Some(state) = state else {
            self.car = None;
            self.failed = None;
            return;
        };
        if self.car.as_ref().is_none_or(|c| c.car != state.car) {
            self.car = None;
            if self.failed.as_deref() == Some(&state.car) || !self.available() {
                return;
            }
            match self.start_car(&state.car) {
                Ok(car) => self.car = Some(car),
                Err(e) => {
                    log::warn!("no engine sound for {}: {e}", state.car);
                    self.failed = Some(state.car.clone());
                    return;
                }
            }
        }
        let Some(car) = self.car.as_mut() else { return };
        let out = car.mixer.update(dt, &state.input);
        car.handle.set(EngineMix {
            frequency: out.ginsu_frequency,
            accel_volume: out.accel_volume,
            decel_volume: out.decel_volume,
            pitch: state.input.pitch_multiplier,
        });
    }

    fn start_car(&mut self, name: &str) -> Result<CarAudio, String> {
        let loaded = self.load_car_engine(name)?;
        let mixer = EngineMixer::new(&tuning(&loaded.sound));
        let silent = EngineMix { frequency: loaded.sound.engine.min_rpm, ..EngineMix::default() };
        let handle = self.start_engine(super::EngineVoice { start: silent, ..loaded.voice })?;
        log::info!("engine sound: {} ({})", loaded.sound.engine.name, loaded.sound.engine.accel_loop);
        Ok(CarAudio { car: name.to_owned(), mixer, handle })
    }
}

#[cfg(test)]
mod tests {
    use blackbox_carsound::GEAR_FIRST;
    use game_install::GameDir;

    use super::*;
    use crate::audio::Volumes;

    /// With the install (`NFSMW_GAME_DIR`) a real car's data drives the mixer: revving raises the Ginsu
    /// frequency and the accelerate loop is audible. Without it the test passes without checking anything.
    #[test]
    fn a_real_car_revs_up_and_is_audible() {
        let Some(dir) = std::env::var_os("NFSMW_GAME_DIR") else { return };
        let mut audio = Audio::new(GameDir::open(std::path::PathBuf::from(dir)).unwrap(), Volumes::default());
        let car = audio.load_car_engine("BMWM3GTR").unwrap();
        let mut mixer = EngineMixer::new(&tuning(&car.sound));
        let at = |rpm_pct: f32, throttle: f32| CarInput { rpm_pct, throttle, gear: GEAR_FIRST, ..CarInput::default() };
        let idle = (0..120).map(|_| mixer.update(1.0 / 60.0, &at(0.0, 0.0))).last().unwrap();
        let revving = (0..240).map(|_| mixer.update(1.0 / 60.0, &at(0.8, 1.0))).last().unwrap();
        assert!(revving.ginsu_frequency > idle.ginsu_frequency + 500.0, "{idle:?} -> {revving:?}");
        assert!(revving.accel_volume > 0.1, "{}", revving.accel_volume);
    }
}
