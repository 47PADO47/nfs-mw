//! The sample resources (kind `0x81`) of a 2012 bundle, decoded (docs/formats/chameleon-sound.md).
//!
//! A sample is a 128-byte header and a whole `SPS` stream: mono, 48 kHz in a car bundle, in the `EL31` codec, which is
//! repacked as MPEG-1 Layer III and decoded.

use std::io::Cursor;

use anyhow::{Context, Result, bail};
use chameleon_bundle::Bundle;
use symphonia::core::codecs::CodecParameters;
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::formats::probe::Hint;
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;

/// The resource kind of a sample.
pub const SAMPLE_KIND: u32 = 0x81;
/// Bytes before the stream in a sample resource.
const SAMPLE_HEADER: usize = 128;
/// Samples a Layer III decoder adds in front of the audio (its filter bank and one granule).
const DECODER_DELAY: usize = 1105;

/// One sample of a bundle, decoded.
#[derive(Debug, Clone)]
pub struct Sample {
    /// The resource id.
    pub id: u64,
    pub samples: Vec<f32>,
    pub rate: u32,
    /// The stream repeats from its start (the loop flag of its header).
    pub looped: bool,
}

impl Sample {
    pub fn seconds(&self) -> f32 {
        self.samples.len() as f32 / self.rate.max(1) as f32
    }

    /// The largest absolute sample.
    pub fn peak(&self) -> f32 {
        self.samples.iter().fold(0.0f32, |peak, s| peak.max(s.abs()))
    }

    /// The root mean square.
    pub fn rms(&self) -> f32 {
        (self.samples.iter().map(|s| s * s).sum::<f32>() / self.samples.len().max(1) as f32).sqrt()
    }
}

/// Every mono sample of `bundle` that decodes, trimmed to the length its header gives. The others are left out.
pub fn read_all(bundle: &Bundle) -> Vec<Sample> {
    (0..bundle.entries().len()).filter_map(|at| read(bundle, at)).collect()
}

/// The sample at `at` of the bundle's table, or `None` if it is not a mono sample this reader decodes.
pub fn read(bundle: &Bundle, at: usize) -> Option<Sample> {
    let entry = bundle.entries().get(at)?;
    if entry.kind != SAMPLE_KIND {
        return None;
    }
    let resource = bundle.resource(at).ok()?;
    let block = resource.blocks.first()?;
    let size = u32::from_le_bytes(block.get(24..28)?.try_into().ok()?) as usize;
    let stream = block.get(SAMPLE_HEADER..SAMPLE_HEADER + size)?;
    let header = ea_audio::sps::read_header(stream).ok()?;
    if header.channels != 1 {
        return None;
    }
    let mp3 = ea_audio::sps::to_mp3(stream).ok()?;
    let (rate, pcm) = decode(mp3.cores.first()?.bytes.clone()).ok()?;
    let end = (DECODER_DELAY + header.samples as usize).min(pcm.len());
    let samples = pcm.get(DECODER_DELAY.min(end)..end)?.to_vec();
    Some(Sample { id: entry.id, samples, rate, looped: header.loop_start.is_some() })
}

/// One stereo sample of a bundle, decoded.
#[derive(Debug, Clone)]
pub struct StereoSample {
    pub id: u64,
    pub frames: Vec<[f32; 2]>,
    pub rate: u32,
    /// The stream repeats from its start (the loop flag of its header).
    pub looped: bool,
}

/// The sample at `at` of the bundle's table, as stereo frames: a mono sample is doubled, a stereo one kept, and a stream of
/// more channels (the 5.1 ones) gives its first stereo core, the front pair. `None` if it is not a sample this reader decodes.
pub fn read_stereo(bundle: &Bundle, at: usize) -> Option<StereoSample> {
    let entry = bundle.entries().get(at)?;
    if entry.kind != SAMPLE_KIND {
        return None;
    }
    let resource = bundle.resource(at).ok()?;
    let block = resource.blocks.first()?;
    let size = u32::from_le_bytes(block.get(24..28)?.try_into().ok()?) as usize;
    let stream = block.get(SAMPLE_HEADER..SAMPLE_HEADER + size)?;
    let header = ea_audio::sps::read_header(stream).ok()?;
    let mp3 = ea_audio::sps::to_mp3(stream).ok()?;
    let (rate, pcm) = decode_stereo(mp3.cores.first()?.bytes.clone()).ok()?;
    let end = (DECODER_DELAY + header.samples as usize).min(pcm.len());
    let frames = pcm.get(DECODER_DELAY.min(end)..end)?.to_vec();
    Some(StereoSample { id: entry.id, frames, rate, looped: header.loop_start.is_some() })
}

/// An MP3 stream as mono samples (the first channel) and its sample rate.
fn decode(bytes: Vec<u8>) -> Result<(u32, Vec<f32>)> {
    decode_by(bytes, |frame| frame[0])
}

/// An MP3 stream as stereo frames (a mono stream is doubled) and its sample rate.
fn decode_stereo(bytes: Vec<u8>) -> Result<(u32, Vec<[f32; 2]>)> {
    decode_by(bytes, |frame| [frame[0], frame[frame.len().min(2) - 1]])
}

/// An MP3 stream decoded, each frame of interleaved channels reduced to one value by `pick`.
fn decode_by<T>(bytes: Vec<u8>, pick: impl Fn(&[f32]) -> T) -> Result<(u32, Vec<T>)> {
    let mut hint = Hint::new();
    hint.with_extension("mp3");
    let source = MediaSourceStream::new(Box::new(Cursor::new(bytes)), MediaSourceStreamOptions::default());
    let mut format = symphonia::default::get_probe()
        .probe(&hint, source, FormatOptions::default(), MetadataOptions::default())
        .context("probing the repacked stream")?;
    let track = format
        .tracks()
        .iter()
        .find(|track| matches!(track.codec_params, Some(CodecParameters::Audio(_))))
        .context("no audio track")?;
    let id = track.id;
    let Some(CodecParameters::Audio(params)) = track.codec_params.clone() else { bail!("no audio parameters") };
    let mut decoder = symphonia::default::get_codecs().make_audio_decoder(&params, &AudioDecoderOptions::default())?;
    let (mut rate, mut samples) = (0, Vec::new());
    while let Ok(Some(packet)) = format.next_packet() {
        if packet.track_id != id {
            continue;
        }
        let Ok(buffer) = decoder.decode(&packet) else { continue };
        rate = buffer.spec().rate();
        let channels = buffer.spec().channels().count().max(1);
        let mut interleaved: Vec<f32> = Vec::new();
        buffer.copy_to_vec_interleaved(&mut interleaved);
        samples.extend(interleaved.chunks_exact(channels).map(&pick));
    }
    Ok((rate, samples))
}

/// A 16-bit mono WAV of `samples` at `rate`, each multiplied by `gain` and clamped.
pub fn wav_bytes(samples: &[f32], rate: u32, gain: f32) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut bytes = b"RIFF".to_vec();
    bytes.extend((36 + data_len).to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16u32.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(rate.to_le_bytes());
    bytes.extend((rate * 2).to_le_bytes());
    bytes.extend(2u16.to_le_bytes());
    bytes.extend(16u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend(data_len.to_le_bytes());
    bytes.extend(samples.iter().flat_map(|s| (((s * gain).clamp(-1.0, 1.0) * 32767.0).round() as i16).to_le_bytes()));
    bytes
}

/// A 16-bit stereo WAV of `frames` at `rate`, each value multiplied by `gain` and clamped.
pub fn wav_stereo_bytes(frames: &[[f32; 2]], rate: u32, gain: f32) -> Vec<u8> {
    let data_len = (frames.len() * 4) as u32;
    let mut bytes = b"RIFF".to_vec();
    bytes.extend((36 + data_len).to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16u32.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(2u16.to_le_bytes());
    bytes.extend(rate.to_le_bytes());
    bytes.extend((rate * 4).to_le_bytes());
    bytes.extend(4u16.to_le_bytes());
    bytes.extend(16u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend(data_len.to_le_bytes());
    let sample = |s: f32| (((s * gain).clamp(-1.0, 1.0) * 32767.0).round() as i16).to_le_bytes();
    bytes.extend(frames.iter().flat_map(|frame| frame.iter().flat_map(|s| sample(*s))));
    bytes
}

/// The gain that brings `rms` to `target`, at most `limit` times.
pub fn gain_to_rms(rms: f32, target: f32, limit: f32) -> f32 {
    if rms > 1e-4 { (target / rms).min(limit) } else { 1.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(values: &[f32]) -> Sample {
        Sample { id: 1, samples: values.to_vec(), rate: 1000, looped: false }
    }

    #[test]
    fn a_sample_knows_its_length_peak_and_level() {
        let s = sample(&[0.5, -1.0, 0.5, -0.5]);
        assert_eq!(s.seconds(), 0.004);
        assert_eq!(s.peak(), 1.0);
        assert!((s.rms() - (1.75f32 / 4.0).sqrt()).abs() < 1e-6);
        assert_eq!(sample(&[]).rms(), 0.0);
    }

    #[test]
    fn a_wav_is_16_bit_mono_with_the_gain_applied_and_clamped() {
        let bytes = wav_bytes(&[0.0, 0.5, -0.5, 1.0], 44_100, 3.0);
        assert_eq!(&bytes[..4], b"RIFF");
        assert_eq!(u16::from_le_bytes([bytes[22], bytes[23]]), 1);
        assert_eq!(u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]), 44_100);
        let samples: Vec<i16> = bytes[44..].as_chunks::<2>().0.iter().map(|b| i16::from_le_bytes(*b)).collect();
        assert_eq!(samples, [0, 32767, -32767, 32767]);
        assert_eq!(u32::from_le_bytes([bytes[40], bytes[41], bytes[42], bytes[43]]), 8);
    }

    #[test]
    fn a_stereo_wav_interleaves_the_channels_and_clamps() {
        let bytes = wav_stereo_bytes(&[[0.5, -0.5], [2.0, 0.0]], 48_000, 1.0);
        assert_eq!(u16::from_le_bytes([bytes[22], bytes[23]]), 2);
        assert_eq!(u32::from_le_bytes([bytes[40], bytes[41], bytes[42], bytes[43]]), 8);
        let samples: Vec<i16> = bytes[44..].as_chunks::<2>().0.iter().map(|b| i16::from_le_bytes(*b)).collect();
        assert_eq!(samples, [16384, -16384, 32767, 0]);
    }

    #[test]
    fn the_gain_brings_a_level_to_its_target_but_never_boosts_silence_or_past_the_limit() {
        assert_eq!(gain_to_rms(0.1, 0.2, 8.0), 2.0);
        assert_eq!(gain_to_rms(0.001, 0.2, 8.0), 8.0);
        assert_eq!(gain_to_rms(0.0, 0.2, 8.0), 1.0);
    }
}
