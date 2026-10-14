//! Captured low-volume startup followed by unchanged, changed and resumed playback.
use std::sync::{Arc, Mutex};

use super::*;
use crate::audio::Audio;
use crate::audio::aems::Part;
use game_install::GameDir;

#[derive(Default)]
struct Gains {
    starts: Vec<f32>,
    updates: Vec<f32>,
}

struct GainOutput(Arc<Mutex<Gains>>);
struct GainVoice(Arc<Mutex<Gains>>);

#[test]
fn captured_start_options_keep_their_native_gain_steps() {
    for (word, option) in [(-1, 0), (118, 0), (235, 0), (323, 1), (32767, 127), (50000, 127)] {
        assert_eq!(creation_gain(word), option as f32 / 127.0);
    }
}

impl Sampler for GainOutput {
    fn sound(&mut self, _: &str, _: usize) -> Result<StaticSoundData, String> {
        Output(Failure::None).sound("synthetic", 1)
    }
    fn start(&mut self, _: StaticSoundData, gain: f32, _: f32) -> Option<Box<dyn Voice>> {
        self.0.lock().unwrap().starts.push(gain);
        Some(Box::new(GainVoice(Arc::clone(&self.0))))
    }
}

impl Voice for GainVoice {
    fn set_gain(&mut self, gain: f32) {
        self.0.lock().unwrap().updates.push(gain);
    }
    fn set_rate(&mut self, _: f32) {}
    fn pause(&mut self) {}
    fn resume(&mut self) {}
    fn stop(&mut self) {}
    fn status(&self) -> VoiceStatus {
        VoiceStatus { playing: true, elapsed_ms: 0, remaining_ms: 0 }
    }
}

#[test]
#[ignore = "requires installed AEMS graph; set NFSMW_GAME_DIR"]
fn installed_scrape_attack_quantizes_only_creation_and_preserves_changed_q15() {
    let install = GameDir::open(std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR").unwrap())).unwrap();
    let mut audio = Audio::replay(install, 48_000);
    let mut part = Part::load(&mut audio, "IG_GLOBAL/FX_Misc_MB.abk", "FX_Scrape", 1).unwrap();
    let gains = Arc::new(Mutex::new(Gains::default()));
    let mut output = GainOutput(Arc::clone(&gains));
    let mut params = [118, 4096, 0, 0, 127, 25000, 0, 32767, 0];
    assert!(part.tick(&mut output, &params, 1.0));
    assert_eq!(part.instance.player_inputs(0).get(input::VOLUME), Some(1));
    assert_eq!(gains.lock().unwrap().starts, [0.0]);
    assert!(gains.lock().unwrap().updates.is_empty());
    let initial = part.instance.player_inputs(0);
    let mut host = PartHost { sampler: &mut output, bank: "synthetic", voices: &mut part.voices, makeup: 1.0 };
    host.update(0, &initial);
    assert!(gains.lock().unwrap().updates.is_empty(), "unchanged volume retains its startup gain");
    params[0] = 235;
    assert!(part.tick(&mut output, &params, 1.0));
    let changed = part.instance.player_inputs(0).get(input::VOLUME).unwrap();
    assert!(changed > 0 && changed < 256);
    let full_gain = changed as f32 / 32767.0;
    assert_eq!(gains.lock().unwrap().updates, [full_gain]);
    let inputs = part.instance.player_inputs(0);
    let mut host = PartHost { sampler: &mut output, bank: "synthetic", voices: &mut part.voices, makeup: 1.0 };
    host.pause(0);
    host.resume(0, &inputs);
    assert_eq!(gains.lock().unwrap().updates, [full_gain, full_gain]);
}
