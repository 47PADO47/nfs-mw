//! Test files for the custom tracks: folders and WAVs under the system temp folder, unique to the test process.

use std::path::{Path, PathBuf};

/// An empty folder named `name` under the temp folder, unique to this test process.
pub fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nfsmw-custom-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Write a 16-bit PCM WAV `dir/name.wav`: `frames` frames of `channels` channels at `rate`, every sample a short
/// ramp. Returns its path.
pub fn write_wav(dir: &Path, name: &str, rate: u32, channels: u16, frames: u32) -> PathBuf {
    let path = dir.join(format!("{name}.wav"));
    let data_len = frames * u32::from(channels) * 2;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&rate.to_le_bytes());
    bytes.extend_from_slice(&(rate * u32::from(channels) * 2).to_le_bytes());
    bytes.extend_from_slice(&(channels * 2).to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for i in 0..frames * u32::from(channels) {
        let sample = (i % 2000) as i16 * 10;
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    std::fs::write(&path, bytes).unwrap();
    path
}
