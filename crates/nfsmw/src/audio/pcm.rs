//! Decoded PCM as a `kira` sound.

use std::sync::Arc;

use ea_audio::Pcm;
use kira::Frame;
use kira::sound::static_sound::{StaticSoundData, StaticSoundSettings};

/// The frames of `pcm`: mono is duplicated, stereo kept, and a surround stream (the cut-scene audio has up to
/// six channels) keeps its first two channels, which are the front pair.
pub fn frames(pcm: &Pcm) -> Vec<Frame> {
    let channels = usize::from(pcm.channels.max(1));
    let scale = 1.0 / 32768.0;
    pcm.samples
        .chunks_exact(channels)
        .map(|c| {
            let left = f32::from(c[0]) * scale;
            let right = c.get(1).map_or(left, |&r| f32::from(r) * scale);
            Frame::new(left, right)
        })
        .collect()
}

/// A sound that plays `pcm` once, or loops its loop points when it has them.
pub fn sound(pcm: &Pcm) -> StaticSoundData {
    let mut settings = StaticSoundSettings::new();
    if let Some((start, end)) = pcm.loop_range {
        let rate = f64::from(pcm.sample_rate.max(1));
        settings = settings.loop_region(f64::from(start) / rate..f64::from(end) / rate);
    }
    StaticSoundData { sample_rate: pcm.sample_rate, frames: Arc::from(frames(pcm)), settings, slice: None }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pcm(channels: u16, samples: Vec<i16>, loop_range: Option<(u32, u32)>) -> Pcm {
        Pcm { sample_rate: 1000, channels, samples, loop_range }
    }

    #[test]
    fn mono_is_duplicated_and_stereo_kept() {
        let mono = frames(&pcm(1, vec![16384, -16384], None));
        assert_eq!(mono, [Frame::new(0.5, 0.5), Frame::new(-0.5, -0.5)]);
        let stereo = frames(&pcm(2, vec![16384, -16384, 0, 8192], None));
        assert_eq!(stereo, [Frame::new(0.5, -0.5), Frame::new(0.0, 0.25)]);
    }

    #[test]
    fn surround_keeps_the_front_pair_and_a_ragged_tail_is_dropped() {
        let six = frames(&pcm(6, vec![1, 2, 3, 4, 5, 6, 7, 8, 9], None));
        assert_eq!(six.len(), 1);
        assert!((six[0].left - 1.0 / 32768.0).abs() < 1e-9 && (six[0].right - 2.0 / 32768.0).abs() < 1e-9);
    }

    #[test]
    fn loop_points_become_a_loop_region_in_seconds() {
        let looped = sound(&pcm(1, vec![0; 100], Some((10, 90))));
        assert!(looped.settings.loop_region.is_some());
        assert!(sound(&pcm(1, vec![0; 100], None)).settings.loop_region.is_none());
        assert_eq!(looped.frames.len(), 100);
    }
}
