//! Plays one movie against a clock: video frames decoded when they come due, the audio decoded up front.

use std::io::Cursor;
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use blackbox_movie::{Demuxer, MovieHeader, Packet, Timeline, Vp6Decoder, YuvFrame};
use ea_audio::Pcm;
use ea_audio::schl::{StreamDecoder, header};

type Source = Demuxer<Cursor<Arc<[u8]>>>;

/// What a step of the clock did.
#[derive(Debug, PartialEq, Eq)]
pub enum Step {
    /// The frame on screen is still the right one.
    Same,
    /// A new frame is in [`Movie::rgba`].
    Frame,
    /// The movie is over.
    Finished,
}

pub struct Movie {
    demuxer: Source,
    decoder: Vp6Decoder,
    timeline: Timeline,
    header: MovieHeader,
    frame: YuvFrame,
    rgba: Vec<u8>,
    clock: f64,
    over: bool,
}

impl Movie {
    pub fn open(bytes: Vec<u8>) -> Result<(Self, Option<Pcm>)> {
        let bytes: Arc<[u8]> = bytes.into();
        let audio = decode_audio(Demuxer::new(Cursor::new(bytes.clone())).context("reading the movie")?)?;
        let demuxer = Demuxer::new(Cursor::new(bytes)).context("reading the movie")?;
        let header = *demuxer.header();
        let decoder = Vp6Decoder::new(header.width, header.height).map_err(|e| anyhow!("the video decoder: {e}"))?;
        let rgba = vec![0; usize::from(header.width) * usize::from(header.height) * 4];
        let movie = Self {
            demuxer,
            decoder,
            timeline: Timeline::from_header(&header),
            header,
            frame: YuvFrame::default(),
            rgba,
            clock: 0.0,
            over: false,
        };
        Ok((movie, audio))
    }

    pub fn size(&self) -> [u32; 2] {
        [u32::from(self.header.width), u32::from(self.header.height)]
    }

    pub fn duration(&self) -> f64 {
        self.header.duration()
    }

    /// The frame last stepped to, RGBA8.
    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }

    /// Moves the clock on by `dt` seconds and decodes up to the frame that is due.
    pub fn step(&mut self, dt: f64) -> Result<Step> {
        if self.over {
            return Ok(Step::Finished);
        }
        self.clock += dt.max(0.0);
        let update = self.timeline.update(self.clock);
        let Some(target) = update.present else {
            self.over = update.finished;
            return Ok(if update.finished { Step::Finished } else { Step::Same });
        };
        // Frames before the due one are decoded and dropped: later frames predict from them.
        loop {
            let Some(packet) = self.demuxer.next_packet().map_err(|e| anyhow!("reading a frame: {e}"))? else {
                self.over = true;
                return Ok(Step::Finished);
            };
            let Packet::Video(video) = packet else { continue };
            self.decoder.decode_into(&video.data, &mut self.frame).map_err(|e| anyhow!("decoding a frame: {e}"))?;
            if video.index >= target {
                self.frame.to_rgba(&mut self.rgba);
                return Ok(Step::Frame);
            }
        }
    }
}

/// The sound of the movie, decoded: the interleaved samples of every audio block.
fn decode_audio(mut demuxer: Source) -> Result<Option<Pcm>> {
    let Some(raw) = demuxer.audio_header().map(|h| h.to_vec()) else { return Ok(None) };
    let stream = header::parse(&raw).map_err(|e| anyhow!("the movie's audio header: {e}"))?;
    let mut decoder = StreamDecoder::new(&stream).map_err(|e| anyhow!("the movie's audio: {e}"))?;
    let mut samples = Vec::new();
    while let Some(packet) = demuxer.next_packet().map_err(|e| anyhow!("reading the movie: {e}"))? {
        if let Packet::Audio(block) = packet {
            decoder.decode_block(&block.data, &mut samples).map_err(|e| anyhow!("decoding the movie's audio: {e}"))?;
        }
    }
    Ok(Some(Pcm { sample_rate: stream.sample_rate, channels: stream.channels, samples, loop_range: None }))
}
