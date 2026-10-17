//! Plays one custom file: a decoder thread reads the file and sends its frames in blocks, as the EA Trax songs are sent
//! (`crate::audio::radio::stream`). The decoder is symphonia, for MP3, Ogg Vorbis, FLAC and WAV
//! (`docs/specs/custom-music.md`, section 4).

use std::fs::File;
use std::path::Path;
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::thread;

use kira::Frame;
use symphonia::core::codecs::CodecParameters;
use symphonia::core::codecs::audio::{AudioCodecParameters, AudioDecoder, AudioDecoderOptions};
use symphonia::core::errors::{Error, Result as DecodeResult};
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::{MetadataOptions, StandardTag};
use symphonia::core::packet::Packet;

use crate::audio::radio::stream::Block;

/// Blocks the thread may run ahead of playback.
const AHEAD: usize = 16;
/// The length of a block in seconds of frames, as the EA Trax blocks are.
const BLOCK_SECS: f64 = 0.2;

/// What the file says about itself. Blank or missing texts are `None`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tags {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
}

/// A file that opened: its rate, length and tags, and the blocks of its frames.
pub struct Opened {
    pub sample_rate: u32,
    /// The length from the container, when it says.
    pub length_secs: Option<f32>,
    pub tags: Tags,
    pub blocks: Receiver<Block>,
}

/// What the decoder thread sends back before any block: the file's facts.
struct Header {
    sample_rate: u32,
    length_secs: Option<f32>,
    tags: Tags,
}

/// Open `path` and start its decoder. A file that cannot be read, is not audio, or uses an unknown codec is an error
/// here, before anything plays. The error names the file.
pub fn open(path: &Path) -> Result<Opened, String> {
    let (header_tx, header_rx) = sync_channel(1);
    let (blocks_tx, blocks) = sync_channel(AHEAD);
    let owned = path.to_path_buf();
    thread::Builder::new()
        .name("custom-decoder".into())
        .spawn(move || run(&owned, &header_tx, &blocks_tx))
        .map_err(|e| format!("cannot start the decoder: {e}"))?;
    let header = header_rx.recv().map_err(|_| format!("{}: the decoder stopped", path.display()))??;
    Ok(Opened { sample_rate: header.sample_rate, length_secs: header.length_secs, tags: header.tags, blocks })
}

/// The decoder thread: prepare the file, report its header, then send blocks until the end.
fn run(path: &Path, header: &SyncSender<Result<Header, String>>, blocks: &SyncSender<Block>) {
    let stream = match Stream::prepare(path) {
        Ok(stream) => stream,
        Err(e) => {
            // The caller may have gone; then there is nobody to tell.
            let _ = header.send(Err(e));
            return;
        }
    };
    if header.send(Ok(stream.header())).is_err() {
        return;
    }
    stream.play(blocks);
}

/// A probed file with its decoder, made on the decoder thread (the symphonia objects stay there).
struct Stream {
    path: String,
    format: Box<dyn FormatReader>,
    decoder: Box<dyn AudioDecoder>,
    track_id: u32,
    sample_rate: u32,
    length_secs: Option<f32>,
    tags: Tags,
    /// Samples of the packet being decoded, interleaved.
    samples: Vec<f32>,
}

impl Stream {
    fn prepare(path: &Path) -> Result<Self, String> {
        let shown = path.display().to_string();
        let file = File::open(path).map_err(|e| format!("{shown}: {e}"))?;
        let mut hint = Hint::new();
        if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(extension);
        }
        let source = MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions::default());
        let mut format = symphonia::default::get_probe()
            .probe(&hint, source, FormatOptions::default(), MetadataOptions::default())
            .map_err(|e| format!("{shown}: {e}"))?;
        let tags = read_tags(format.as_mut());
        let (track_id, params, frames) =
            audio_track(format.as_ref()).ok_or_else(|| format!("{shown}: no audio track"))?;
        let sample_rate = params.sample_rate.ok_or_else(|| format!("{shown}: the sample rate is unknown"))?;
        let decoder = symphonia::default::get_codecs()
            .make_audio_decoder(&params, &AudioDecoderOptions::default())
            .map_err(|e| format!("{shown}: {e}"))?;
        let length_secs = frames.map(|frames| (frames as f64 / f64::from(sample_rate.max(1))) as f32);
        Ok(Self { path: shown, format, decoder, track_id, sample_rate, length_secs, tags, samples: Vec::new() })
    }

    fn header(&self) -> Header {
        Header { sample_rate: self.sample_rate, length_secs: self.length_secs, tags: self.tags.clone() }
    }

    /// Decode the audio track and send blocks of about `BLOCK_SECS` until the end, an error, or a gone receiver.
    fn play(mut self, blocks: &SyncSender<Block>) {
        let target = (f64::from(self.sample_rate) * BLOCK_SECS).ceil() as usize;
        let mut block: Block = Vec::with_capacity(target);
        while let Some(packet) = self.next_packet() {
            if packet.track_id != self.track_id {
                continue;
            }
            let channels = match decode_into(self.decoder.as_mut(), &packet, &mut self.samples) {
                Ok(channels) => channels,
                // A damaged packet is skipped; the next one may decode.
                Err(Error::DecodeError(_)) => continue,
                Err(e) => {
                    log::warn!("custom music: {} stopped: {e}", self.path);
                    break;
                }
            };
            push_frames(&self.samples, channels, &mut block);
            if block.len() < target {
                continue;
            }
            let full = std::mem::replace(&mut block, Vec::with_capacity(target));
            if blocks.send(full).is_err() {
                return;
            }
        }
        if !block.is_empty() {
            let _ = blocks.send(block);
        }
    }

    /// The next packet, or `None` at the end. A read error ends the file with a log line.
    fn next_packet(&mut self) -> Option<Packet> {
        match self.format.next_packet() {
            Ok(packet) => packet,
            Err(e) => {
                log::warn!("custom music: {} stopped early: {e}", self.path);
                None
            }
        }
    }
}

/// The first audio track: its id, its codec parameters and its length in frames when the container gives one.
fn audio_track(format: &dyn FormatReader) -> Option<(u32, AudioCodecParameters, Option<u64>)> {
    format.tracks().iter().find_map(|track| match &track.codec_params {
        Some(CodecParameters::Audio(params)) => Some((track.id, params.clone(), track.num_frames)),
        _ => None,
    })
}

/// The title, artist and album of the file's tags (the first of each kind that is not blank).
fn read_tags(format: &mut dyn FormatReader) -> Tags {
    let mut tags = Tags::default();
    let metadata = format.metadata();
    let Some(revision) = metadata.current() else { return tags };
    for tag in &revision.media.tags {
        let (slot, text) = match &tag.std {
            Some(StandardTag::TrackTitle(text)) => (&mut tags.title, text),
            Some(StandardTag::Artist(text)) => (&mut tags.artist, text),
            Some(StandardTag::Album(text)) => (&mut tags.album, text),
            _ => continue,
        };
        let text = text.trim();
        if slot.is_none() && !text.is_empty() {
            *slot = Some(text.to_owned());
        }
    }
    tags
}

/// Decode `packet` into `samples` (interleaved); returns the channel count of the decoded buffer.
fn decode_into(decoder: &mut dyn AudioDecoder, packet: &Packet, samples: &mut Vec<f32>) -> DecodeResult<usize> {
    let buffer = decoder.decode(packet)?;
    buffer.copy_to_vec_interleaved(samples);
    Ok(buffer.spec().channels().count())
}

/// Append stereo frames of interleaved `samples`: mono is duplicated, and of more channels the first two are kept.
fn push_frames(samples: &[f32], channels: usize, block: &mut Block) {
    let channels = channels.max(1);
    block.extend(samples.chunks_exact(channels).map(|frame| {
        let left = frame[0];
        Frame::new(left, frame.get(1).copied().unwrap_or(left))
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::radio::custom::test_files::{folder, write_wav};

    fn drain(opened: &Opened) -> usize {
        opened.blocks.iter().map(|block| block.len()).sum()
    }

    #[test]
    fn a_mono_wav_opens_with_its_rate_and_length_and_every_frame_arrives() {
        let dir = folder("decode-mono");
        let path = write_wav(&dir, "mono", 8000, 1, 8000);
        let opened = open(&path).expect("a WAV opens");
        assert_eq!(opened.sample_rate, 8000);
        assert!((opened.length_secs.unwrap() - 1.0).abs() < 1e-3);
        assert_eq!(drain(&opened), 8000);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_stereo_wav_keeps_both_channels_as_frames() {
        let dir = folder("decode-stereo");
        let path = write_wav(&dir, "stereo", 22050, 2, 5000);
        let opened = open(&path).expect("a WAV opens");
        assert_eq!(drain(&opened), 5000);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_file_that_is_not_audio_is_an_error_that_names_it() {
        let path = std::env::temp_dir().join(format!("nfsmw-custom-decode-{}-junk.wav", std::process::id()));
        std::fs::write(&path, b"this is not audio at all").unwrap();
        let err = open(&path).err().expect("junk does not open");
        assert!(err.contains("junk.wav"), "{err}");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_missing_file_is_an_error() {
        let path = std::env::temp_dir().join("nfsmw-custom-decode-does-not-exist.mp3");
        assert!(open(&path).is_err());
    }

    #[test]
    fn mono_is_duplicated_and_more_channels_keep_the_front_pair() {
        let mut block = Vec::new();
        push_frames(&[0.5, -0.5], 1, &mut block);
        push_frames(&[0.25, 0.5, 9.0, 9.0], 4, &mut block);
        assert_eq!(block, [Frame::new(0.5, 0.5), Frame::new(-0.5, -0.5), Frame::new(0.25, 0.5)]);
    }
}
