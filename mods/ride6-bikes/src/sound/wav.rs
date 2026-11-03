//! A small WAV reader and writer for the recordings the user decoded from their own install: PCM 8, 16, 24 and 32 bit,
//! float 32 and 64 bit, any channel count, plus the first loop of a `smpl` chunk. Output is 16-bit PCM, the format the
//! game reads (`docs/modding/car-sound.md`).

use anyhow::{Result, bail, ensure};

/// Decoded audio: interleaved samples between -1 and 1.
#[derive(Debug, Clone, PartialEq)]
pub struct Pcm {
    pub sample_rate: u32,
    pub channels: u16,
    /// Interleaved, `frames * channels` long.
    pub samples: Vec<f32>,
    /// The first loop of the file's `smpl` chunk, in frames: start and end (exclusive).
    pub loop_range: Option<(usize, usize)>,
}

fn u16_at(bytes: &[u8], at: usize) -> Result<u16> {
    let Some(slice) = bytes.get(at..at + 2) else { bail!("WAV ends inside a chunk") };
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

fn u32_at(bytes: &[u8], at: usize) -> Result<u32> {
    let Some(slice) = bytes.get(at..at + 4) else { bail!("WAV ends inside a chunk") };
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

struct Format {
    float: bool,
    channels: u16,
    sample_rate: u32,
    bits: u16,
}

fn parse_format(chunk: &[u8]) -> Result<Format> {
    let mut tag = u16_at(chunk, 0)?;
    let channels = u16_at(chunk, 2)?;
    let sample_rate = u32_at(chunk, 4)?;
    let bits = u16_at(chunk, 14)?;
    if tag == 0xFFFE {
        tag = u16_at(chunk, 24)?;
    }
    ensure!(tag == 1 || tag == 3, "WAV format tag {tag} is neither PCM nor float");
    ensure!(channels >= 1, "WAV with no channel");
    Ok(Format { float: tag == 3, channels, sample_rate, bits })
}

fn decode_samples(format: &Format, data: &[u8]) -> Result<Vec<f32>> {
    let width = usize::from(format.bits / 8);
    ensure!(
        matches!((format.float, format.bits), (false, 8 | 16 | 24 | 32) | (true, 32 | 64)),
        "unsupported sample format"
    );
    let mut out = Vec::with_capacity(data.len() / width);
    for raw in data.chunks_exact(width) {
        let value = match (format.float, format.bits) {
            (false, 8) => (f32::from(raw[0]) - 128.0) / 128.0,
            (false, 16) => f32::from(i16::from_le_bytes([raw[0], raw[1]])) / 32768.0,
            (false, 24) => {
                let wide = i32::from_le_bytes([0, raw[0], raw[1], raw[2]]) >> 8;
                wide as f32 / 8_388_608.0
            }
            (false, _) => i32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]) as f32 / 2_147_483_648.0,
            (true, 32) => f32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]),
            (true, _) => {
                let mut eight = [0u8; 8];
                eight.copy_from_slice(raw);
                f64::from_le_bytes(eight) as f32
            }
        };
        out.push(if value.is_finite() { value.clamp(-1.0, 1.0) } else { 0.0 });
    }
    Ok(out)
}

fn parse_loop(chunk: &[u8]) -> Option<(usize, usize)> {
    let loops = u32_at(chunk, 28).ok()?;
    if loops == 0 {
        return None;
    }
    let start = u32_at(chunk, 44).ok()? as usize;
    let end = u32_at(chunk, 48).ok()? as usize;
    // The end of a `smpl` loop is the last frame of the loop, inclusive.
    (end >= start).then_some((start, end + 1))
}

/// Decode a WAV file.
pub fn parse(bytes: &[u8]) -> Result<Pcm> {
    ensure!(bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE", "not a RIFF WAVE file");
    let mut at = 12;
    let mut format = None;
    let mut data = None;
    let mut smpl = None;
    while at + 8 <= bytes.len() {
        let size = u32_at(bytes, at + 4)? as usize;
        let body_start = at + 8;
        let body_end = (body_start + size).min(bytes.len());
        let body = &bytes[body_start..body_end];
        match &bytes[at..at + 4] {
            b"fmt " => format = Some(parse_format(body)?),
            b"data" => data = Some(body),
            b"smpl" => smpl = Some(body),
            _ => {}
        }
        at = body_start + size + (size & 1);
    }
    let Some(format) = format else { bail!("WAV without a fmt chunk") };
    let Some(data) = data else { bail!("WAV without a data chunk") };
    let samples = decode_samples(&format, data)?;
    let frames = samples.len() / usize::from(format.channels);
    let loop_range = smpl.and_then(parse_loop).filter(|&(start, end)| start < end && end <= frames);
    Ok(Pcm { sample_rate: format.sample_rate, channels: format.channels, samples, loop_range })
}

/// Encode as a 16-bit PCM WAV file (no loop chunk: the game loops the whole file).
pub fn encode16(pcm: &Pcm) -> Vec<u8> {
    let data_len = pcm.samples.len() * 2;
    let block_align = u32::from(pcm.channels) * 2;
    let mut out = Vec::with_capacity(44 + data_len);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&pcm.channels.to_le_bytes());
    out.extend_from_slice(&pcm.sample_rate.to_le_bytes());
    out.extend_from_slice(&(pcm.sample_rate * block_align).to_le_bytes());
    out.extend_from_slice(&(block_align as u16).to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data_len as u32).to_le_bytes());
    for &sample in &pcm.samples {
        let value = (sample.clamp(-1.0, 1.0) * 32767.0).round() as i16;
        out.extend_from_slice(&value.to_le_bytes());
    }
    out
}

impl Pcm {
    /// Frames (one sample per channel).
    pub fn frames(&self) -> usize {
        self.samples.len() / usize::from(self.channels)
    }

    /// The largest absolute sample.
    pub fn peak(&self) -> f32 {
        self.samples.iter().fold(0.0_f32, |peak, sample| peak.max(sample.abs()))
    }

    /// Scale so the peak is `target` (0 to 1); silence is left alone.
    pub fn normalize(&mut self, target: f32) {
        let peak = self.peak();
        if peak <= 1e-6 {
            return;
        }
        let gain = target / peak;
        self.samples.iter_mut().for_each(|sample| *sample *= gain);
    }

    /// Keep only the loop body of a file that has a `smpl` loop; a file without one is returned whole.
    pub fn loop_body(&self) -> Pcm {
        let Some((start, end)) = self.loop_range else { return Pcm { loop_range: None, ..self.clone() } };
        let channels = usize::from(self.channels);
        let samples = self.samples[start * channels..end * channels].to_vec();
        Pcm { sample_rate: self.sample_rate, channels: self.channels, samples, loop_range: None }
    }

    /// Make a clip loop without a click: the last `fade_frames` frames are blended into the first ones with equal
    /// power and removed. A clip shorter than three times the fade is returned unchanged.
    pub fn seamless_loop(&self, fade_frames: usize) -> Pcm {
        let frames = self.frames();
        if fade_frames == 0 || frames < fade_frames * 3 {
            return self.clone();
        }
        let channels = usize::from(self.channels);
        let keep = frames - fade_frames;
        let mut samples = self.samples[..keep * channels].to_vec();
        for frame in 0..fade_frames {
            let t = (frame as f32 + 0.5) / fade_frames as f32;
            let (head, tail) = ((t * std::f32::consts::FRAC_PI_2).sin(), (t * std::f32::consts::FRAC_PI_2).cos());
            for channel in 0..channels {
                let tail_sample = self.samples[(keep + frame) * channels + channel];
                let head_sample = &mut samples[frame * channels + channel];
                *head_sample = *head_sample * head + tail_sample * tail;
            }
        }
        Pcm { sample_rate: self.sample_rate, channels: self.channels, samples, loop_range: None }
    }
}

/// Sum several clips (each with a gain) into one that is as long as the longest, repeating the shorter ones, with the
/// channel count of the widest. All must have one sample rate.
pub fn mix_down(parts: &[(&Pcm, f32)]) -> Result<Pcm> {
    let Some((first, _)) = parts.first() else { bail!("nothing to mix") };
    ensure!(parts.iter().all(|(pcm, _)| pcm.sample_rate == first.sample_rate), "clips of different sample rates");
    let channels = parts.iter().map(|(pcm, _)| pcm.channels).max().unwrap_or(1);
    let frames = parts.iter().map(|(pcm, _)| pcm.frames()).max().unwrap_or(0);
    ensure!(frames > 0, "empty clip");
    let wide = usize::from(channels);
    let mut samples = vec![0.0_f32; frames * wide];
    for (pcm, gain) in parts {
        let own = pcm.frames();
        ensure!(own > 0, "empty clip");
        let own_channels = usize::from(pcm.channels);
        for frame in 0..frames {
            for channel in 0..wide {
                let source_channel = channel.min(own_channels - 1);
                samples[frame * wide + channel] += pcm.samples[(frame % own) * own_channels + source_channel] * gain;
            }
        }
    }
    Ok(Pcm { sample_rate: first.sample_rate, channels, samples, loop_range: None })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(frames: usize, rate: u32) -> Pcm {
        let samples = (0..frames).map(|i| (i as f32 * 0.1).sin() * 0.5).collect();
        Pcm { sample_rate: rate, channels: 1, samples, loop_range: None }
    }

    #[test]
    fn a_wav_survives_a_round_trip() {
        let pcm = tone(500, 22050);
        let back = parse(&encode16(&pcm)).unwrap();
        assert_eq!(back.sample_rate, 22050);
        assert_eq!(back.channels, 1);
        assert_eq!(back.frames(), 500);
        assert!(pcm.samples.iter().zip(&back.samples).all(|(a, b)| (a - b).abs() < 1e-3));
    }

    #[test]
    fn a_float_wav_and_a_loop_chunk_are_read() {
        let mut file = Vec::new();
        file.extend_from_slice(b"RIFF\0\0\0\0WAVEfmt ");
        file.extend_from_slice(&16u32.to_le_bytes());
        for field in [3u16, 1] {
            file.extend_from_slice(&field.to_le_bytes());
        }
        file.extend_from_slice(&44100u32.to_le_bytes());
        file.extend_from_slice(&(44100u32 * 4).to_le_bytes());
        file.extend_from_slice(&4u16.to_le_bytes());
        file.extend_from_slice(&32u16.to_le_bytes());
        file.extend_from_slice(b"data");
        file.extend_from_slice(&(10u32 * 4).to_le_bytes());
        for i in 0..10 {
            file.extend_from_slice(&(i as f32 / 20.0).to_le_bytes());
        }
        file.extend_from_slice(b"smpl");
        file.extend_from_slice(&60u32.to_le_bytes());
        let mut smpl = vec![0u8; 60];
        smpl[28..32].copy_from_slice(&1u32.to_le_bytes());
        smpl[44..48].copy_from_slice(&2u32.to_le_bytes());
        smpl[48..52].copy_from_slice(&7u32.to_le_bytes());
        file.extend_from_slice(&smpl);
        let pcm = parse(&file).unwrap();
        assert_eq!(pcm.frames(), 10);
        assert_eq!(pcm.loop_range, Some((2, 8)));
        assert_eq!(pcm.loop_body().frames(), 6);
    }

    /// A WAV file of one fmt chunk (`tag`, mono, `bits` wide) and a data chunk of `data`.
    fn raw_wav(tag: u16, bits: u16, data: &[u8]) -> Vec<u8> {
        let width = u32::from(bits / 8);
        let mut file = Vec::new();
        file.extend_from_slice(b"RIFF\0\0\0\0WAVEfmt ");
        file.extend_from_slice(&16u32.to_le_bytes());
        file.extend_from_slice(&tag.to_le_bytes());
        file.extend_from_slice(&1u16.to_le_bytes());
        file.extend_from_slice(&8000u32.to_le_bytes());
        file.extend_from_slice(&(8000 * width).to_le_bytes());
        file.extend_from_slice(&(width as u16).to_le_bytes());
        file.extend_from_slice(&bits.to_le_bytes());
        file.extend_from_slice(b"data");
        file.extend_from_slice(&(data.len() as u32).to_le_bytes());
        file.extend_from_slice(data);
        file
    }

    #[test]
    fn eight_sixteen_and_twenty_four_bit_samples_are_read_at_their_scale() {
        let eight = parse(&raw_wav(1, 8, &[0, 128, 255])).unwrap();
        assert!((eight.samples[0] + 1.0).abs() < 1e-6 && eight.samples[1] == 0.0 && eight.samples[2] > 0.99);
        let sixteen = parse(&raw_wav(1, 16, &[0x00, 0x40, 0x00, 0xC0])).unwrap();
        assert_eq!(sixteen.samples, [0.5, -0.5]);
        // 24-bit: the bytes 00 00 40 are half scale and 00 00 C0 are minus half.
        let twenty_four = parse(&raw_wav(1, 24, &[0x00, 0x00, 0x40, 0x00, 0x00, 0xC0])).unwrap();
        assert_eq!(twenty_four.samples, [0.5, -0.5]);
    }

    #[test]
    fn a_float_sample_that_is_not_a_number_reads_as_silence() {
        let mut data = Vec::new();
        data.extend_from_slice(&f32::NAN.to_le_bytes());
        data.extend_from_slice(&2.0f32.to_le_bytes());
        let pcm = parse(&raw_wav(3, 32, &data)).unwrap();
        assert_eq!(pcm.samples, [0.0, 1.0]);
    }

    #[test]
    fn an_unsupported_format_and_a_loop_past_the_end_are_dealt_with() {
        assert!(parse(&raw_wav(2, 16, &[0, 0])).is_err(), "ADPCM is not read");
        assert!(parse(&raw_wav(1, 12, &[0, 0])).is_err(), "12 bit is not read");
        let mut file = raw_wav(1, 16, &[0u8; 20]);
        file.extend_from_slice(b"smpl");
        file.extend_from_slice(&60u32.to_le_bytes());
        let mut smpl = vec![0u8; 60];
        smpl[28..32].copy_from_slice(&1u32.to_le_bytes());
        smpl[44..48].copy_from_slice(&2u32.to_le_bytes());
        smpl[48..52].copy_from_slice(&99u32.to_le_bytes());
        file.extend_from_slice(&smpl);
        let pcm = parse(&file).unwrap();
        assert_eq!((pcm.frames(), pcm.loop_range), (10, None));
    }

    #[test]
    fn a_wave_format_extensible_header_reads_its_sub_format() {
        let mut file = Vec::new();
        file.extend_from_slice(b"RIFF\0\0\0\0WAVEfmt ");
        file.extend_from_slice(&40u32.to_le_bytes());
        file.extend_from_slice(&0xFFFEu16.to_le_bytes());
        file.extend_from_slice(&1u16.to_le_bytes());
        file.extend_from_slice(&8000u32.to_le_bytes());
        file.extend_from_slice(&16_000u32.to_le_bytes());
        file.extend_from_slice(&2u16.to_le_bytes());
        file.extend_from_slice(&16u16.to_le_bytes());
        file.extend_from_slice(&22u16.to_le_bytes());
        file.extend_from_slice(&16u16.to_le_bytes());
        file.extend_from_slice(&4u32.to_le_bytes());
        // The sub-format GUID starts with the format tag (1, PCM).
        file.extend_from_slice(&[1, 0, 0, 0, 0, 0, 0x10, 0, 0x80, 0, 0, 0xAA, 0, 0x38, 0x9B, 0x71]);
        file.extend_from_slice(b"data");
        file.extend_from_slice(&2u32.to_le_bytes());
        file.extend_from_slice(&[0x00, 0x20]);
        let pcm = parse(&file).unwrap();
        assert_eq!((pcm.sample_rate, pcm.samples.len()), (8000, 1));
        assert!((pcm.samples[0] - 0.25).abs() < 1e-6);
    }

    #[test]
    fn garbage_is_refused() {
        assert!(parse(b"nothing").is_err());
    }

    #[test]
    fn normalize_sets_the_peak() {
        let mut pcm = tone(100, 8000);
        pcm.normalize(0.8);
        assert!((pcm.peak() - 0.8).abs() < 1e-5);
    }

    #[test]
    fn a_seamless_loop_is_shorter_by_the_fade_and_starts_near_the_old_end() {
        let pcm = tone(1000, 8000);
        let looped = pcm.seamless_loop(100);
        assert_eq!(looped.frames(), 900);
        let first = looped.samples[0];
        let blended = pcm.samples[900];
        assert!((first - blended).abs() < 0.1);
    }

    #[test]
    fn mixing_repeats_the_short_clip_and_widens() {
        let a = tone(10, 8000);
        let stereo = Pcm { sample_rate: 8000, channels: 2, samples: vec![0.1; 40], loop_range: None };
        let mixed = mix_down(&[(&a, 1.0), (&stereo, 0.5)]).unwrap();
        assert_eq!(mixed.channels, 2);
        assert_eq!(mixed.frames(), 20);
        assert!(mix_down(&[]).is_err());
    }
}
