//! Decoding `SCDl` blocks into PCM.

use super::header::{Codec, StreamHeader};
use crate::bytes::{Endian, slice, u32_in};
use crate::codec::microtalk;
use crate::codec::xa::{self, Revision, XaState};
use crate::error::{Error, Result};

/// Version 0 blocks: after the sample count come 8 bytes of ADPCM history, then the data.
const LEGACY_XA_START: usize = 4 + 8;
/// Version 0 MicroTalk blocks: after the sample count comes the flag byte.
const LEGACY_MT_START: usize = 4 + 1;

/// Decoder state of one channel.
enum Channel {
    Xa(XaState),
    MicroTalk(Box<microtalk::Decoder>),
}

/// Decodes the `SCDl` blocks of one stream in order. The ADPCM history and the MicroTalk synthesis state carry
/// from block to block, so blocks must be fed in stream order.
pub struct StreamDecoder {
    header: StreamHeader,
    endian: Endian,
    channels: Vec<Channel>,
}

impl StreamDecoder {
    /// A decoder for the stream the header describes.
    pub fn new(header: &StreamHeader) -> Result<Self> {
        let channels = (0..header.channels)
            .map(|_| match header.codec {
                Codec::EaXa | Codec::EaXaStereo => Ok(Channel::Xa(XaState::new())),
                Codec::MicroTalk => Ok(Channel::MicroTalk(Box::new(microtalk::Decoder::new(header.pcm_blocks)))),
                Codec::Other(id) => Err(Error::UnsupportedCodec(id)),
            })
            .collect::<Result<Vec<_>>>()?;
        if header.codec == Codec::EaXaStereo && header.channels != 2 {
            return Err(Error::Unsupported("stereo-flavour EA-XA with other than 2 channels"));
        }
        let endian = if header.big_endian { Endian::Big } else { Endian::Little };
        Ok(Self { header: header.clone(), endian, channels })
    }

    /// The header this decoder was built from.
    pub fn header(&self) -> &StreamHeader {
        &self.header
    }

    /// Decode the payload of one `SCDl` block and append the interleaved samples to `out`. Returns the number
    /// of frames (samples per channel) appended.
    pub fn decode_block(&mut self, payload: &[u8], out: &mut Vec<i16>) -> Result<usize> {
        let count = u32_in(payload, 0, self.endian)? as usize;
        if count == 0 {
            return Ok(0);
        }
        let ch = self.channels.len();
        let starts = self.channel_starts(payload, count)?;
        let max_samples = payload.len().saturating_mul(16).saturating_add(microtalk::FRAME_SAMPLES);
        if count > max_samples {
            return Err(Error::Corrupt("block claims more samples than its data can hold"));
        }
        let mut decoded: Vec<Vec<i16>> = Vec::with_capacity(ch);
        let revision = if self.header.pcm_blocks { Revision::V2 } else { Revision::V1 };
        if self.header.codec == Codec::EaXaStereo {
            decoded = self.decode_stereo(payload, starts[0], count)?;
        } else {
            for (c, &start) in starts.iter().enumerate() {
                let data = slice(payload, start, payload.len().saturating_sub(start))?;
                let mut samples = Vec::with_capacity(count);
                match &mut self.channels[c] {
                    Channel::Xa(state) => {
                        state.decode_run(data, revision, count, &mut samples)?;
                    }
                    Channel::MicroTalk(decoder) => decoder.decode_channel(data, count, &mut samples)?,
                }
                decoded.push(samples);
            }
        }
        out.reserve(count * ch);
        for i in 0..count {
            for channel in &decoded {
                out.push(channel[i]);
            }
        }
        Ok(count)
    }

    /// Byte offset inside the payload at which each channel's data starts.
    fn channel_starts(&self, payload: &[u8], count: usize) -> Result<Vec<usize>> {
        let ch = self.channels.len();
        let table = 4 + 4 * ch;
        let h = &self.header;
        if h.version == 0 {
            return self.legacy_starts(count);
        }
        let skip = match h.codec {
            Codec::MicroTalk => 1,
            Codec::EaXa if !h.pcm_blocks => 4,
            _ => 0,
        };
        (0..ch)
            .map(|c| {
                let offset = u32_in(payload, 4 + 4 * c, self.endian)? as usize;
                let start = table.checked_add(offset).and_then(|s| s.checked_add(skip));
                start.filter(|&s| s <= payload.len()).ok_or(Error::Corrupt("channel offset outside its block"))
            })
            .collect()
    }

    /// Version 0 streams have no offset table: the layout follows from the codec.
    fn legacy_starts(&self, count: usize) -> Result<Vec<usize>> {
        let ch = self.channels.len();
        match self.header.codec {
            Codec::EaXaStereo => Ok(vec![LEGACY_XA_START; ch]),
            Codec::EaXa => {
                Ok((0..ch).map(|c| LEGACY_XA_START + c * (count / xa::FRAME_SAMPLES) * xa::FRAME_BYTES).collect())
            }
            Codec::MicroTalk if ch == 1 => Ok(vec![LEGACY_MT_START]),
            _ => Err(Error::Unsupported("channel layout of a version 0 stream")),
        }
    }

    fn decode_stereo(&mut self, payload: &[u8], start: usize, count: usize) -> Result<Vec<Vec<i16>>> {
        let mut data = slice(payload, start, payload.len().saturating_sub(start))?;
        let [Channel::Xa(left), Channel::Xa(right)] = &mut self.channels[..] else {
            return Err(Error::Unsupported("stereo-flavour EA-XA needs two XA channels"));
        };
        let (mut l, mut r) = (Vec::with_capacity(count), Vec::with_capacity(count));
        let (mut fl, mut fr) = ([0i16; xa::FRAME_SAMPLES], [0i16; xa::FRAME_SAMPLES]);
        for _ in 0..count.div_ceil(xa::FRAME_SAMPLES) {
            let used = xa::decode_stereo_frame(left, right, data, &mut fl, &mut fr)?;
            data = &data[used..];
            l.extend_from_slice(&fl);
            r.extend_from_slice(&fr);
        }
        l.truncate(count);
        r.truncate(count);
        Ok(vec![l, r])
    }
}
