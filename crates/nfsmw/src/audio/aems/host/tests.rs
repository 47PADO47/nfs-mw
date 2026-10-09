//! The engine's availability follows explicit decoder/backend failures, never its requested volume.

use std::sync::Arc;

use kira::Frame;
use kira::sound::static_sound::StaticSoundSettings;

use super::*;

#[derive(Clone, Copy)]
enum Failure {
    Decode,
    Start,
    Empty,
    Rate,
    None,
}

struct Output(Failure);

impl Sampler for Output {
    fn sound(&mut self, _: &str, _: usize) -> Result<StaticSoundData, String> {
        if matches!(self.0, Failure::Decode) {
            return Err("missing recording".into());
        }
        let frames = match self.0 {
            Failure::Empty => Vec::new(),
            _ => vec![Frame::from_mono(0.5); 8],
        };
        let sample_rate = match self.0 {
            Failure::Rate => 0,
            _ => 48_000,
        };
        Ok(StaticSoundData {
            sample_rate,
            frames: Arc::from(frames),
            settings: StaticSoundSettings::new(),
            slice: None,
        })
    }

    fn start(&mut self, _: StaticSoundData, _: f32, _: f32) -> Option<Box<dyn Voice>> {
        if matches!(self.0, Failure::Start) {
            return None;
        }
        Some(Box::new(Playing))
    }
}

struct Playing;

impl Voice for Playing {
    fn set_gain(&mut self, _: f32) {}
    fn set_rate(&mut self, _: f32) {}
    fn pause(&mut self) {}
    fn resume(&mut self) {}
    fn stop(&mut self) {}
    fn status(&self) -> VoiceStatus {
        VoiceStatus { playing: true, elapsed_ms: 0, remaining_ms: 0 }
    }
}

fn entry() -> SampleEntry {
    SampleEntry { kind: 0, priority: 0, index: 1, loop_offset: 0 }
}

#[test]
fn decoder_and_output_failures_disable_sample_takeover() {
    for failure in [Failure::Decode, Failure::Start, Failure::Empty, Failure::Rate] {
        let (mut output, mut voices) = (Output(failure), Voices::default());
        let mut host = PartHost { sampler: &mut output, bank: "synthetic", voices: &mut voices, makeup: 1.0 };
        assert!(!host.play(0, &entry(), &PlayerInputs::default()));
        assert!(voices.failed);
    }
}

#[test]
fn a_healthy_voice_does_not_disable_takeover() {
    let (mut output, mut voices) = (Output(Failure::None), Voices::default());
    let mut host = PartHost { sampler: &mut output, bank: "synthetic", voices: &mut voices, makeup: 1.0 };
    assert!(host.play(0, &entry(), &PlayerInputs::default()));
    assert!(!voices.failed);
    assert_eq!(voices.slots.len(), 1);
}

#[test]
fn unsupported_recordings_do_not_promise_a_limiter_replacement() {
    let (mut output, mut voices) = (Output(Failure::None), Voices::default());
    let mut host = PartHost { sampler: &mut output, bank: "synthetic", voices: &mut voices, makeup: 1.0 };
    assert!(!host.play(0, &SampleEntry { kind: 1, ..entry() }, &PlayerInputs::default()));
    assert!(voices.failed);
}
