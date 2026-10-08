//! `GinsuSynth`: renders an engine loop at a target frequency by playing the recording forward and
//! jumping whole pitch cycles. Spec: `docs/specs/engine-sound-ginsu.md` §2 to §4.

mod packet;

use std::sync::Arc;

use crate::data::GinsuData;
use crate::round;
use crate::tables::FREQUENCY_PER_HZ;
use crate::{Error, Result};

/// Seconds a packet lasts, and the shortest time between two jumps.
const PACKET_SECONDS: f32 = 0.011;
/// The cross-fade at a jump.
const OVERLAP_SECONDS: f32 = 0.0005;
/// The latency the game passes with every frequency update.
pub const DEFAULT_LATENCY_MS: f32 = 60.0;
/// `1 / 11`: converts a latency in milliseconds into packets (the original multiplies by this constant).
const PACKETS_PER_MS: f32 = 0.090_909_09;
const MIN_SAMPLE_RATE: u32 = 2000;

/// What the caller wants from the synthesiser for a block.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SynthParams {
    /// Target frequency in the file's units (engine "RPM"). Non-finite values are ignored.
    pub frequency: f32,
    /// Linear gain applied to the block, ramped from the previous block's gain. Negative or non-finite
    /// values count as 0.
    pub volume: f32,
    /// Milliseconds the synthesiser takes to reach a new frequency (about 11 ms per packet of lag).
    pub latency_ms: f32,
}

impl SynthParams {
    /// Full volume, the game's usual latency.
    pub fn new(frequency: f32) -> Self {
        Self { frequency, volume: 1.0, latency_ms: DEFAULT_LATENCY_MS }
    }

    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume;
        self
    }
}

/// The granular synthesiser for one loop. Pure and deterministic: the same data and the same sequence of
/// calls always give the same samples, whatever the platform.
#[derive(Debug, Clone)]
pub struct GinsuSynth {
    data: Arc<GinsuData>,
    packet_size: i32,
    no_jump_size: i32,
    overlap_size: i32,
    playback_pos: i32,
    current_pos: i32,
    target_pos: i32,
    countdown: i32,
    no_jump_remaining: i32,
    current_cycle: f32,
    /// Bits of the last (frequency, latency) given to [`Self::update_frequency`] by `render`.
    requested: Option<(u32, u32)>,
    queue: Vec<f32>,
    queue_pos: usize,
    scratch: [Vec<f32>; 2],
    gain: Option<f32>,
}

impl GinsuSynth {
    /// A synthesiser positioned where the recording's pitch equals `start_frequency`.
    pub fn new(data: Arc<GinsuData>, start_frequency: f32) -> Result<Self> {
        let tables = data.tables();
        if tables.sample_rate() < MIN_SAMPLE_RATE {
            return Err(Error::SampleRateTooLow(tables.sample_rate()));
        }
        if tables.cycle_count() < 1 {
            return Err(Error::NoCycles);
        }
        let rate = tables.sample_rate() as f32;
        let packet_size = round::round(rate * PACKET_SECONDS);
        let no_jump_size = round::round(rate * PACKET_SECONDS);
        let overlap_size = round::round(rate * OVERLAP_SECONDS);
        let mut synth = Self {
            data,
            packet_size,
            no_jump_size,
            overlap_size,
            playback_pos: 0,
            current_pos: 0,
            target_pos: 0,
            countdown: 1,
            no_jump_remaining: no_jump_size,
            current_cycle: 0.0,
            requested: None,
            queue: Vec::with_capacity((packet_size + overlap_size) as usize),
            queue_pos: 0,
            scratch: [Vec::new(), Vec::new()],
            gain: None,
        };
        synth.start(start_frequency);
        Ok(synth)
    }

    /// Restarts at `frequency`: the playback position, the glide and the pending samples are reset (the
    /// gain history is kept).
    pub fn start(&mut self, frequency: f32) {
        let tables = self.data.tables();
        let pos = tables.frequency_to_sample(frequency);
        self.playback_pos = pos;
        self.current_pos = pos;
        self.target_pos = pos;
        self.countdown = 1;
        self.no_jump_remaining = self.no_jump_size;
        self.current_cycle = tables.sample_to_cycle(pos);
        self.requested = None;
        self.queue.clear();
        self.queue_pos = 0;
    }

    /// Sets the target frequency and how long to take to reach it. The original calls this with 60 ms on
    /// every game update; each call restarts the countdown, so a steady stream of calls makes the pitch
    /// approach the target gradually. [`Self::render`] calls it only when the requested frequency changes.
    pub fn update_frequency(&mut self, frequency: f32, latency_ms: f32) {
        if !frequency.is_finite() {
            return;
        }
        self.target_pos = self.data.tables().frequency_to_sample(frequency);
        let latency = match latency_ms.is_finite() {
            true => latency_ms.max(0.0),
            false => DEFAULT_LATENCY_MS,
        };
        self.countdown = round::floor(latency * PACKETS_PER_MS) + 1;
    }

    /// The sample rate of the output.
    pub fn sample_rate(&self) -> u32 {
        self.data.tables().sample_rate()
    }

    /// The pitch of the cycle being played, in Hz (0 if undefined).
    pub fn current_pitch(&self) -> f32 {
        let period = self.data.tables().cycle_period(self.current_cycle + 0.5);
        if period <= 0.0 {
            return 0.0;
        }
        self.sample_rate() as f32 / period
    }

    /// [`Self::current_pitch`] in the file's frequency units (Hz x 120): what the game reports back.
    pub fn current_frequency(&self) -> f32 {
        self.current_pitch() * FREQUENCY_PER_HZ
    }

    /// The recording this synthesiser plays.
    pub fn data(&self) -> &Arc<GinsuData> {
        &self.data
    }

    /// Fills `out` with mono samples at [`Self::sample_rate`]. The frequency, when it differs from the last
    /// call's, becomes the new target (as [`Self::update_frequency`]). Packets are made as needed, so any
    /// block size works; the volume ramps linearly across the block.
    pub fn render(&mut self, params: &SynthParams, out: &mut [f32]) {
        self.apply_request(params);
        let mut filled = 0;
        while filled < out.len() {
            if self.queue_pos >= self.queue.len() {
                self.next_packet();
            }
            let take = (self.queue.len() - self.queue_pos).min(out.len() - filled);
            out[filled..filled + take].copy_from_slice(&self.queue[self.queue_pos..self.queue_pos + take]);
            self.queue_pos += take;
            filled += take;
        }
        self.apply_gain(params.volume, out);
    }

    fn apply_request(&mut self, params: &SynthParams) {
        if !params.frequency.is_finite() {
            return;
        }
        let key = (params.frequency.to_bits(), params.latency_ms.to_bits());
        if self.requested == Some(key) {
            return;
        }
        self.requested = Some(key);
        self.update_frequency(params.frequency, params.latency_ms);
    }

    fn apply_gain(&mut self, volume: f32, out: &mut [f32]) {
        let target = match volume.is_finite() {
            true => volume.max(0.0),
            false => 0.0,
        };
        let start = self.gain.unwrap_or(target);
        self.gain = Some(target);
        if start == 1.0 && target == 1.0 {
            return;
        }
        let n = out.len() as f32;
        for (i, sample) in out.iter_mut().enumerate() {
            let t = (i + 1) as f32 / n;
            *sample *= start + (target - start) * t;
        }
    }
}
