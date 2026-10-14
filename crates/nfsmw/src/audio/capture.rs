//! Unclipped float WAV evidence and basic level measurements for headless audio replays.
use std::path::Path;

pub(super) fn write_float(path: &Path, samples: &[f32], rate: u32, channels: u16) {
    let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    let mut wav = Vec::with_capacity(data.len() + 56);
    wav.extend(b"RIFF");
    wav.extend((48 + data.len() as u32).to_le_bytes());
    wav.extend(b"WAVEfmt ");
    wav.extend(16u32.to_le_bytes());
    wav.extend(3u16.to_le_bytes());
    wav.extend(channels.to_le_bytes());
    wav.extend(rate.to_le_bytes());
    wav.extend((rate * u32::from(channels) * 4).to_le_bytes());
    wav.extend((channels * 4).to_le_bytes());
    wav.extend(32u16.to_le_bytes());
    wav.extend(b"fact");
    wav.extend(4u32.to_le_bytes());
    wav.extend((samples.len() as u32 / u32::from(channels)).to_le_bytes());
    wav.extend(b"data");
    wav.extend((data.len() as u32).to_le_bytes());
    wav.extend(data);
    std::fs::write(path, wav).unwrap();
}

#[derive(Debug, serde::Serialize)]
pub(super) struct Levels {
    pub peak: f32,
    pub rms: f32,
    pub over_full_scale: usize,
}

pub(super) fn levels(samples: &[f32]) -> Levels {
    assert!(!samples.is_empty());
    assert!(samples.iter().all(|s| s.is_finite()));
    Levels {
        peak: samples.iter().fold(0.0_f32, |m, s| m.max(s.abs())),
        rms: (samples.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>() / samples.len() as f64).sqrt() as f32,
        over_full_scale: samples.iter().filter(|s| s.abs() > 1.0).count(),
    }
}
