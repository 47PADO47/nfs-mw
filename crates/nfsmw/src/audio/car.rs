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
    use kira::Frame;
    use kira::sound::SoundData;

    use super::*;
    use crate::audio::{EngineVoice, Volumes};

    const RATE: f64 = 48_000.0;
    const TICK: f32 = 1.0 / 60.0;

    fn audio() -> Option<Audio> {
        let dir = std::env::var_os("NFSMW_GAME_DIR")?;
        Some(Audio::new(GameDir::open(std::path::PathBuf::from(dir)).unwrap(), Volumes::default()))
    }

    /// With the install (`NFSMW_GAME_DIR`) a real car's data drives the mixer: revving raises the Ginsu
    /// frequency and the accelerate loop is audible. Without it the test passes without checking anything.
    #[test]
    fn a_real_car_revs_up_and_is_audible() {
        let Some(mut audio) = audio() else { return };
        let car = audio.load_car_engine("BMWM3GTR").unwrap();
        let mut mixer = EngineMixer::new(&tuning(&car.sound));
        let at = |rpm_pct: f32, throttle: f32| CarInput { rpm_pct, throttle, gear: GEAR_FIRST, ..CarInput::default() };
        let idle = (0..120).map(|_| mixer.update(TICK, &at(0.0, 0.0))).last().unwrap();
        let revving = (0..240).map(|_| mixer.update(TICK, &at(0.8, 1.0))).last().unwrap();
        assert!(revving.ginsu_frequency > idle.ginsu_frequency + 500.0, "{idle:?} -> {revving:?}");
        assert!(revving.accel_volume > 0.1, "{}", revving.accel_volume);
    }

    /// One scripted second of telemetry: `(rpm fraction, throttle, gear)` at time `t`.
    fn script(t: f32) -> (f32, f32, i32) {
        match t {
            t if t < 2.0 => (0.0, 0.0, 2),
            t if t < 5.0 => (0.05 + (t - 2.0) / 3.0 * 0.9, 1.0, 2),
            t if t < 5.5 => (0.95 - (t - 5.0) * 0.6, 0.0, 3),
            t if t < 8.5 => (0.65 + (t - 5.5) / 3.0 * 0.3, 1.0, 3),
            t if t < 9.0 => (0.95 - (t - 8.5) * 0.5, 0.0, 4),
            t => (0.7 - ((t - 9.0) * 0.2).min(0.65), 0.0, 4),
        }
    }

    fn write_wav(path: &std::path::Path, samples: &[f32]) {
        let data: Vec<u8> =
            samples.iter().flat_map(|s| ((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes()).collect();
        let mut wav = Vec::new();
        wav.extend(b"RIFF");
        wav.extend((36 + data.len() as u32).to_le_bytes());
        wav.extend(b"WAVEfmt ");
        wav.extend([16u32.to_le_bytes().as_slice(), &1u16.to_le_bytes(), &1u16.to_le_bytes()].concat());
        wav.extend((RATE as u32).to_le_bytes());
        wav.extend((RATE as u32 * 2).to_le_bytes());
        wav.extend([2u16.to_le_bytes().as_slice(), &16u16.to_le_bytes()].concat());
        wav.extend(b"data");
        wav.extend((data.len() as u32).to_le_bytes());
        wav.extend(data);
        std::fs::write(path, wav).unwrap();
    }

    /// Renders a drive of the M3 GTR (idle, a pull with two shifts, a lift) through the mixer and the engine
    /// voice, as the game does, and checks the level and that the sound moves with the revs. With
    /// `NFSMW_ENGINE_WAV` set the sound is written there to listen to.
    #[test]
    fn a_scripted_drive_sounds_like_an_engine() {
        let Some(mut audio) = audio() else { return };
        let car = audio.load_car_engine("BMWM3GTR").unwrap();
        let mut mixer = EngineMixer::new(&tuning(&car.sound));
        let silent = EngineMix { frequency: car.sound.engine.min_rpm, ..EngineMix::default() };
        let (mut sound, handle) = EngineVoice { start: silent, ..car.voice }.into_sound().unwrap();
        let info = kira::info::MockInfoBuilder::new().build();
        let per_tick = (RATE / 60.0) as usize;
        let (mut out, mut shifts) = (Vec::new(), 0);
        for i in 0..(14.0 * 60.0) as usize {
            let (rpm_pct, throttle, gear) = script(i as f32 * TICK);
            let input = CarInput { rpm_pct, throttle, gear, speed: 10.0 + rpm_pct * 40.0, ..CarInput::default() };
            let o = mixer.update(TICK, &input);
            shifts += usize::from(o.events.gear_clunk.is_some());
            handle.set(EngineMix {
                frequency: o.ginsu_frequency,
                accel_volume: o.accel_volume,
                decel_volume: o.decel_volume,
                pitch: 1.0,
            });
            let mut block = vec![Frame::ZERO; per_tick];
            sound.process(&mut block, 1.0 / RATE, &info);
            out.extend(block.iter().map(|f| f.left));
        }
        if let Some(path) = std::env::var_os("NFSMW_ENGINE_WAV") {
            write_wav(std::path::Path::new(&path), &out);
        }
        let rms = |from: f32, to: f32| {
            let part = &out[(from * RATE as f32) as usize..(to * RATE as f32) as usize];
            (part.iter().map(|s| s * s).sum::<f32>() / part.len() as f32).sqrt()
        };
        let peak = out.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        eprintln!(
            "peak {peak:.3}; rms idle {:.4}, pull {:.4}, high revs {:.4}, lift {:.4}; {shifts} gear clunks",
            rms(0.5, 2.0),
            rms(2.5, 5.0),
            rms(4.0, 5.0),
            rms(10.0, 13.0)
        );
        assert!(out.iter().all(|s| s.is_finite()));
        assert!(peak < 1.5, "the engine alone should not clip: {peak}");
        assert!(rms(0.5, 2.0) > 0.005, "the idle is audible: {}", rms(0.5, 2.0));
        assert!(rms(2.5, 5.0) > rms(0.5, 2.0) * 0.5, "the pull is not quieter than half the idle");
    }
}
