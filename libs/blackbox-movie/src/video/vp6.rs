//! VP6 frame decoding through the MIT-licensed NihAV subset (`nihav_duck`).

use nihav_core::codecs::{DecoderError, NABufferType, NADecoderSupport, NAVideoBuffer, NAVideoInfo, YUV420_FORMAT};
use nihav_duck::codecs::vp6::{VP6BR, VP56Decoder};

use super::yuv::YuvFrame;
use crate::error::MovieError;

/// VP6 version number nihav uses for the plain VP6 bitstream; frames carry their own version field.
const VP6_VERSION: u8 = 6;

/// Decoder for the VP6 frames of a movie.
///
/// Frames must be fed in file order, starting with a key frame: inter frames predict from the
/// previous frame and the last key frame. Frames that are not going to be shown still have to be
/// decoded.
pub struct Vp6Decoder {
    decoder: VP56Decoder,
    parser: VP6BR,
    support: NADecoderSupport,
    width: usize,
    height: usize,
}

impl Vp6Decoder {
    /// A decoder for frames of the given size (from [`crate::MovieHeader`]).
    pub fn new(width: u16, height: u16) -> Result<Self, MovieError> {
        let (width, height) = (usize::from(width), usize::from(height));
        let mut decoder = VP56Decoder::new(VP6_VERSION, false, false);
        let mut support = NADecoderSupport::new();
        let info = NAVideoInfo::new(width, height, false, YUV420_FORMAT);
        decoder.init(&mut support, info).map_err(decode_error)?;
        Ok(Self { decoder, parser: VP6BR::new(), support, width, height })
    }

    /// Forgets the reference frames, so the next frame has to be a key frame (use after a seek).
    pub fn reset(&mut self) {
        self.decoder.flush();
    }

    /// Decodes one frame into `frame`, reusing its buffers.
    pub fn decode_into(&mut self, data: &[u8], frame: &mut YuvFrame) -> Result<(), MovieError> {
        let (buffer, _kind) =
            self.decoder.decode_frame(&mut self.support, data, &mut self.parser).map_err(decode_error)?;
        let NABufferType::Video(video) = buffer else {
            return Err(MovieError::Decode("the decoder returned no video buffer".into()));
        };
        let info = video.get_info();
        if info.get_width() != self.width || info.get_height() != self.height {
            return Err(MovieError::Decode(format!(
                "the frame is {}x{}, the movie header says {}x{}",
                info.get_width(),
                info.get_height(),
                self.width,
                self.height
            )));
        }
        copy_planes(&video, frame);
        Ok(())
    }

    /// Decodes one frame into a new [`YuvFrame`].
    pub fn decode(&mut self, data: &[u8]) -> Result<YuvFrame, MovieError> {
        let mut frame = YuvFrame::default();
        self.decode_into(data, &mut frame)?;
        Ok(frame)
    }
}

/// Copies the three planes out of the decoder's pooled buffer, dropping the row padding and turning
/// the picture the right way up: plain VP6 (unlike Flash's VP6F) stores its rows bottom to top, and
/// the NihAV decoder only records that in a flag instead of flipping.
fn copy_planes(video: &NAVideoBuffer<u8>, frame: &mut YuvFrame) {
    let info = video.get_info();
    frame.width = info.get_width();
    frame.height = info.get_height();
    let data = video.get_data();
    let planes = [&mut frame.y, &mut frame.u, &mut frame.v];
    for (index, plane) in planes.into_iter().enumerate() {
        let (width, height) = video.get_dimensions(index);
        let (stride, offset) = (video.get_stride(index), video.get_offset(index));
        plane.clear();
        plane.reserve(width * height);
        for row in (0..height).rev() {
            let start = offset + row * stride;
            plane.extend_from_slice(&data[start..start + width]);
        }
    }
}

fn decode_error(error: DecoderError) -> MovieError {
    MovieError::Decode(format!("{error:?}"))
}
