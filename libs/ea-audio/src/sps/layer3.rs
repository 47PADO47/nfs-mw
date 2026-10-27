//! EA's Layer III frames (`EL31`), repacked as ordinary MPEG-1 Layer III frames.
//!
//! An EA frame is one granule (576 samples per channel) of a standard MPEG-1 frame, with the 4-byte MP3 header squeezed
//! into one byte and no bit reservoir. The game's reader (`EaLayer3Cores`) reads, bit by bit:
//!
//! 1. a flag byte, `0xEE` when raw PCM follows the frame and `0x00` otherwise;
//! 2. the header byte: version (2), sample-rate index (2), channel mode (2), mode extension (2);
//! 3. one bit, the granule number (0 or 1);
//! 4. for granule 1, the scalefactor-sharing bits (4 per channel);
//! 5. per channel, the standard 59 bits of granule side information, whose first 12 bits are `part2_3_length`;
//! 6. the main data: `part2_3_length` bits for each channel in turn;
//!
//! then it skips to the next byte. Two consecutive granules make one MPEG-1 frame, so the repacking writes the standard
//! header and side information (no reservoir) and the two main data runs, and any MP3 decoder reads the result.

use super::bits::{BitReader, BitWriter};
use crate::error::{Error, Result};

/// Bits of side information per channel and granule in an MPEG-1 Layer III frame.
const GRANULE_BITS: u32 = 59;
/// MPEG-1 Layer III bit rates in kbit/s, by index 1 to 14.
const BIT_RATES: [u32; 14] = [32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320];
/// Sample rates of MPEG-1 by index.
const SAMPLE_RATES: [u32; 3] = [44_100, 48_000, 32_000];
/// The flag of a frame that has raw PCM after it.
const PCM_FLAG: u8 = 0xEE;

/// One granule as read from the stream.
pub struct Granule {
    sample_rate_index: u8,
    channel_mode: u8,
    mode_extension: u8,
    /// 0 or 1: which granule of an MPEG-1 frame it is.
    index: u8,
    /// Four scalefactor-sharing bits per channel (granule 1 only), channel 0 first.
    scfsi: Vec<u8>,
    /// The 59 side information bits of each channel.
    side: Vec<u64>,
    /// The main data of all channels, as a run of bytes and its length in bits.
    main: Vec<u8>,
    main_bits: usize,
}

impl Granule {
    fn channels(&self) -> usize {
        match self.channel_mode {
            3 => 1,
            _ => 2,
        }
    }
}

/// Builds the MPEG-1 stream of one core (one stereo or mono pair of channels) from its granules, in order. A granule 0
/// that is still waiting for its granule 1 is kept until the next one arrives; [`Repacker::finish`] writes the frames.
///
/// Every frame is written at the largest bit rate, 320 kbit/s, and its main data placed through the bit reservoir: EA's
/// frames vary in size (a granule can take more than a 320 kbit/s frame holds on its own), so each frame's data sits
/// right after the previous frame's, up to 511 bytes ahead of its own frame, and `main_data_begin` says where.
#[derive(Default)]
pub struct Repacker {
    pub bytes: Vec<u8>,
    pub frames: usize,
    pending: Option<Granule>,
    pairs: Vec<(Granule, Granule)>,
}

/// The largest `main_data_begin` the side information can hold.
const MAX_RESERVOIR: usize = 511;
/// The bit rate index of 320 kbit/s.
const TOP_RATE_INDEX: usize = BIT_RATES.len();

impl Repacker {
    /// Write a granule 0 that never got its partner as a frame with a silent second granule, then build the stream.
    pub fn finish(&mut self) -> Result<()> {
        if let Some(first) = self.pending.take() {
            let silent = silence_for(&first);
            self.pairs.push((first, silent));
        }
        self.build()
    }

    pub fn push_granule(&mut self, granule: Granule) -> Result<()> {
        let Some(first) = self.pending.take() else {
            if granule.index == 0 {
                self.pending = Some(granule);
                return Ok(());
            }
            // A granule 1 with no granule 0 before it: pair it with silence.
            let silent = silence_for(&granule);
            self.pairs.push((silent, granule));
            return Ok(());
        };
        if granule.index == 0 {
            // Two granule 0 in a row: the first one lacks its partner.
            let silent = silence_for(&first);
            self.pairs.push((first, silent));
            self.pending = Some(granule);
            return Ok(());
        }
        self.pairs.push((first, granule));
        Ok(())
    }

    fn build(&mut self) -> Result<()> {
        let Some((head, _)) = self.pairs.first() else { return Ok(()) };
        let channels = head.channels();
        let side_bytes = match channels {
            1 => 17,
            _ => 32,
        };
        let sample_rate = SAMPLE_RATES[usize::from(head.sample_rate_index)];
        let top_bits = 144 * BIT_RATES[TOP_RATE_INDEX - 1] * 1000;
        if !top_bits.is_multiple_of(sample_rate) {
            return Err(Error::Unsupported("a sample rate whose frames are not a whole number of bytes"));
        }
        let frame_len = (top_bits / sample_rate) as usize;
        let capacity = frame_len - 4 - side_bytes;

        // Place each frame's main data in a stream of frame-sized areas, then cut the stream into the frames.
        let mut areas = vec![0u8; capacity * self.pairs.len()];
        let mut begins = Vec::with_capacity(self.pairs.len());
        let mut data_end = 0;
        for (k, (first, second)) in self.pairs.iter().enumerate() {
            let area = k * capacity;
            let at = data_end.max(area.saturating_sub(MAX_RESERVOIR));
            let mut bits = BitWriter::default();
            bits.copy(&first.main, 0, first.main_bits);
            bits.copy(&second.main, 0, second.main_bits);
            let data = bits.into_bytes();
            if at + data.len() > area + capacity {
                return Err(Error::Corrupt("a frame holds more data than the reservoir can carry"));
            }
            areas[at..at + data.len()].copy_from_slice(&data);
            begins.push(area - at);
            data_end = at + data.len();
        }

        for (k, (first, second)) in self.pairs.iter().enumerate() {
            let mut out = BitWriter::default();
            out.write(0xFFFB, 16);
            out.write(TOP_RATE_INDEX as u64, 4);
            out.write(u64::from(first.sample_rate_index), 2);
            out.write(0, 2);
            out.write(u64::from(first.channel_mode), 2);
            out.write(u64::from(first.mode_extension), 2);
            out.write(0, 4);
            // Side information: where the data starts, no private bits, the sharing bits of granule 1, both granules.
            out.write(begins[k] as u64, 9);
            out.write(0, if channels == 1 { 5 } else { 3 });
            for channel in 0..channels {
                out.write(u64::from(second.scfsi.get(channel).copied().unwrap_or(0)), 4);
            }
            for granule in [first, second] {
                for channel in 0..channels {
                    out.write(granule.side.get(channel).copied().unwrap_or(0), GRANULE_BITS);
                }
            }
            let mut frame = out.into_bytes();
            frame.extend_from_slice(&areas[k * capacity..(k + 1) * capacity]);
            self.bytes.extend_from_slice(&frame);
        }
        self.frames = self.pairs.len();
        self.pairs.clear();
        Ok(())
    }
}

/// A granule with no data, laid out like `like`, which a decoder reads as silence.
fn silence_for(like: &Granule) -> Granule {
    Granule {
        sample_rate_index: like.sample_rate_index,
        channel_mode: like.channel_mode,
        mode_extension: like.mode_extension,
        index: 1 - like.index,
        scfsi: vec![0; like.channels()],
        side: vec![0; like.channels()],
        main: Vec::new(),
        main_bits: 0,
    }
}

/// Read one frame at the start of `data`: the granule and the bytes the frame takes.
pub fn read_granule(data: &[u8]) -> Result<(Granule, usize)> {
    let flag = *data.first().ok_or(Error::Truncated { offset: 0, needed: 1 })?;
    if flag != 0 && flag != PCM_FLAG {
        return Err(Error::Corrupt("an unknown frame flag"));
    }
    let mut bits = BitReader::new(&data[1..]);
    let header = bits.read(8)?;
    if (header >> 6) & 3 != 3 {
        return Err(Error::Unsupported("a Layer III stream that is not MPEG-1"));
    }
    let sample_rate_index = ((header >> 4) & 3) as u8;
    if sample_rate_index > 2 {
        return Err(Error::Corrupt("an invalid sample rate index"));
    }
    let channel_mode = ((header >> 2) & 3) as u8;
    let mode_extension = (header & 3) as u8;
    let index = bits.read(1)? as u8;
    let channels = if channel_mode == 3 { 1 } else { 2 };
    let mut scfsi = Vec::new();
    if index == 1 {
        for _ in 0..channels {
            scfsi.push(bits.read(4)? as u8);
        }
    }
    let mut side = Vec::new();
    let mut main_bits = 0usize;
    for _ in 0..channels {
        let word = bits.read_wide(GRANULE_BITS)?;
        main_bits += (word >> (GRANULE_BITS - 12)) as usize;
        side.push(word);
    }
    let start = bits.position();
    let total = start + main_bits;
    if total > (data.len() - 1) * 8 {
        return Err(Error::Truncated { offset: data.len() as u64, needed: (total / 8).saturating_sub(data.len() - 1) });
    }
    let mut copy = BitWriter::default();
    copy.copy(&data[1..], start, main_bits);
    let granule = Granule {
        sample_rate_index,
        channel_mode,
        mode_extension,
        index,
        scfsi,
        side,
        main: copy.into_bytes(),
        main_bits,
    };
    let mut used = 1 + total.div_ceil(8);
    if flag == PCM_FLAG {
        used += pcm_block_len(&data[used..], channels)?;
    }
    Ok((granule, used))
}

/// The bytes of the raw PCM that follows a frame flagged `0xEE`: a count, an offset, then that many 16-bit samples per
/// channel. The game writes them over the decoded samples (a splice for looping); the repacking leaves them out.
fn pcm_block_len(data: &[u8], channels: usize) -> Result<usize> {
    let count = data.get(..4).ok_or(Error::Truncated { offset: 0, needed: 4 })?;
    let count = u32::from_be_bytes([count[0], count[1], count[2], count[3]]) as usize;
    let len = 8 + count * channels * 2;
    if len > data.len() {
        return Err(Error::Truncated { offset: data.len() as u64, needed: len - data.len() });
    }
    Ok(len)
}
