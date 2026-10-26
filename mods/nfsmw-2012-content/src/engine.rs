//! The car's own engine sound, from the sample resources of its 2012 bundle (docs/formats/chameleon-sound.md).
//!
//! A car bundle holds about forty samples: mono 48 kHz `EL31` streams behind a 128-byte header, seven or so of them
//! loops. The 2012 game mixes them in a sound graph with synthesised harmonics, which this mod does not rebuild. What
//! it does instead is read each loop's pitch, treat the loops as recordings of the engine at different speeds (a loop's
//! rpm is the idle rpm times its pitch over the lowest pitch), and let the game's mixer play them by engine speed. That
//! is an approximation: the loops that are not engine loops (a whine, a drone) are played as if they were.

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
const SAMPLE_KIND: u32 = 0x81;
/// Bytes before the stream in a sample resource.
const SAMPLE_HEADER: usize = 128;
/// Samples a Layer III decoder adds in front of the audio (its filter bank and one granule).
const DECODER_DELAY: usize = 1105;
/// Shortest loop kept: shorter ones are one-shot-like and repeat too fast.
const MIN_SECONDS: f32 = 0.8;
/// The pitch range searched, in Hz: idle of a big engine to the whine of a high-revving one.
const PITCH_RANGE: (f32, f32) = (20.0, 700.0);
/// Layers closer than this ratio in pitch are one layer.
const SAME_PITCH: f32 = 1.06;
/// A layer above this many times the redline would be played at a pitch the engine never reaches.
const BEYOND_REDLINE: f32 = 1.15;
/// The level the layers are brought to (root mean square).
const TARGET_RMS: f32 = 0.2;
/// Most layers a car gets (what the car manifest allows).
const MAX_LAYERS: usize = 8;

/// One looping sample of a car, decoded.
pub struct Loop {
    pub samples: Vec<f32>,
    pub rate: u32,
    /// The pitch of the loop in Hz, 0 when it has none.
    pub pitch: f32,
}

/// A layer of the car's engine sound: a loop and the engine speed it stands for.
pub struct Layer {
    pub rpm: f32,
    pub wav: Vec<u8>,
}

/// The looping samples of the car bundle `data`, decoded and trimmed to one loop each.
pub fn loops(data: &[u8]) -> Result<Vec<Loop>> {
    let bundle = Bundle::parse(data)?;
    let mut out = Vec::new();
    for (at, entry) in bundle.entries().iter().enumerate() {
        if entry.kind != SAMPLE_KIND {
            continue;
        }
        let resource = bundle.resource(at)?;
        let block = &resource.blocks[0];
        let Some(size) = block.get(24..28).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize) else {
            continue;
        };
        let Some(stream) = block.get(SAMPLE_HEADER..SAMPLE_HEADER + size) else { continue };
        let Ok(header) = ea_audio::sps::read_header(stream) else { continue };
        if header.loop_start.is_none() || header.channels != 1 {
            continue;
        }
        let Ok(mp3) = ea_audio::sps::to_mp3(stream) else { continue };
        let Ok((rate, pcm)) = decode(mp3.cores[0].bytes.clone()) else { continue };
        let end = (DECODER_DELAY + header.samples as usize).min(pcm.len());
        let Some(samples) = pcm.get(DECODER_DELAY.min(end)..end) else { continue };
        if (samples.len() as f32) < MIN_SECONDS * rate as f32 {
            continue;
        }
        let pitch = pitch_of(samples, rate);
        out.push(Loop { samples: samples.to_vec(), rate, pitch });
    }
    Ok(out)
}

/// An MP3 stream as mono samples (the first channel) and its sample rate.
fn decode(bytes: Vec<u8>) -> Result<(u32, Vec<f32>)> {
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
        samples.extend(interleaved.chunks_exact(channels).map(|frame| frame[0]));
    }
    Ok((rate, samples))
}

/// The pitch of `samples` in Hz: the first strong peak of the normalised autocorrelation of a stretch from the middle of
/// the loop, or 0 when nothing in [`PITCH_RANGE`] repeats.
pub fn pitch_of(samples: &[f32], rate: u32) -> f32 {
    let window = 8192.min(samples.len() / 2);
    let from = (samples.len() - window) / 2;
    let stretch = &samples[from..from + window];
    let (shortest, longest) = ((rate as f32 / PITCH_RANGE.1) as usize, (rate as f32 / PITCH_RANGE.0) as usize);
    let longest = longest.min(window / 2);
    if shortest >= longest {
        return 0.0;
    }
    let energy: f32 = stretch.iter().map(|s| s * s).sum();
    if energy <= 1e-6 {
        return 0.0;
    }
    let correlation: Vec<f32> = (shortest..longest)
        .map(|lag| stretch[..window - lag].iter().zip(&stretch[lag..]).map(|(a, b)| a * b).sum::<f32>() / energy)
        .collect();
    let best = correlation.iter().copied().fold(f32::MIN, f32::max);
    if best < 0.2 {
        return 0.0;
    }
    // The first local peak that is nearly as strong as the best: a multiple of the period is as strong or stronger.
    let peak = (1..correlation.len().saturating_sub(1)).find(|&at| {
        correlation[at] >= 0.85 * best
            && correlation[at] >= correlation[at - 1]
            && correlation[at] > correlation[at + 1]
    });
    let Some(at) = peak else { return 0.0 };
    rate as f32 / (shortest + at) as f32
}

/// The layers of an engine that idles at `idle_rpm` and is limited at `redline_rpm`, from the loops of its car: the
/// pitched loops, slowest first, each given the rpm its pitch stands for, at one level. Empty when there is no pitched
/// loop.
pub fn layers(loops: Vec<Loop>, idle_rpm: f32, redline_rpm: f32) -> Vec<Layer> {
    let mut pitched: Vec<Loop> = loops.into_iter().filter(|l| l.pitch > 0.0).collect();
    pitched.sort_by(|a, b| a.pitch.total_cmp(&b.pitch));
    let Some(lowest) = pitched.first().map(|l| l.pitch) else { return Vec::new() };
    let mut layers: Vec<(f32, Loop)> = Vec::new();
    for loop_ in pitched {
        let rpm = idle_rpm * loop_.pitch / lowest;
        if rpm > redline_rpm * BEYOND_REDLINE {
            continue;
        }
        let same = layers.last().is_some_and(|(known, _)| rpm / known < SAME_PITCH);
        if same {
            continue;
        }
        layers.push((rpm, loop_));
    }
    layers.into_iter().take(MAX_LAYERS).map(|(rpm, l)| Layer { rpm, wav: wav_bytes(&l) }).collect()
}

/// A 16-bit mono WAV of the loop at the level every layer is brought to.
fn wav_bytes(loop_: &Loop) -> Vec<u8> {
    let rms = (loop_.samples.iter().map(|s| s * s).sum::<f32>() / loop_.samples.len().max(1) as f32).sqrt();
    let gain = if rms > 1e-4 { (TARGET_RMS / rms).min(8.0) } else { 1.0 };
    let data_len = (loop_.samples.len() * 2) as u32;
    let mut bytes = b"RIFF".to_vec();
    bytes.extend((36 + data_len).to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16u32.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(loop_.rate.to_le_bytes());
    bytes.extend((loop_.rate * 2).to_le_bytes());
    bytes.extend(2u16.to_le_bytes());
    bytes.extend(16u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend(data_len.to_le_bytes());
    bytes.extend(
        loop_.samples.iter().flat_map(|s| (((s * gain).clamp(-1.0, 1.0) * 32767.0).round() as i16).to_le_bytes()),
    );
    bytes
}

#[cfg(test)]
mod tests;
