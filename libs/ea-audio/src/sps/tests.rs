use super::bits::{BitReader, BitWriter};
use super::*;

#[test]
fn bits_round_trip_in_msb_first_order() {
    let mut out = BitWriter::default();
    out.write(0b101, 3);
    out.write(0x1ff, 9);
    out.write(1, 1);
    let bytes = out.into_bytes();
    let mut input = BitReader::new(&bytes);
    assert_eq!((input.read(3).unwrap(), input.read(9).unwrap(), input.read(1).unwrap()), (0b101, 0x1ff, 1));
}

#[test]
fn the_header_of_a_48_khz_stereo_layer3_stream_reads_back() {
    // `H`, size 12, then version 1, codec 5, 2 channels, 48000 Hz, kind 1, no loop, 5,782,138 samples.
    let data = [0x48, 0x00, 0x00, 0x0c, 0x15, 0x04, 0xbb, 0x80, 0x40, 0x58, 0x3a, 0x7a];
    let header = read_header(&data).unwrap();
    assert_eq!(header.codec, Codec::EaLayer3);
    assert_eq!((header.channels, header.sample_rate, header.samples, header.loop_start), (2, 48_000, 5_782_138, None));
}

#[test]
fn a_looping_header_has_its_loop_start() {
    let data = [0x48, 0, 0, 0x14, 0x15, 0x00, 0x3e, 0x80, 0x60, 0x05, 0x63, 0x52, 0, 0, 0, 7, 0, 0, 0, 0x14];
    let header = read_header(&data).unwrap();
    assert_eq!((header.channels, header.sample_rate, header.samples, header.loop_start), (1, 16_000, 353_106, Some(7)));
}

#[test]
fn a_stream_without_an_h_chunk_is_refused() {
    assert!(read_header(b"SCHl\0\0\0\0").is_err());
}

/// One EA frame: a granule of a stereo 48 kHz stream with `part2_3_length` 0 in both channels.
fn empty_granule(index: u8) -> Vec<u8> {
    let mut out = BitWriter::default();
    out.write(0, 8); // flag
    out.write(0b11_01_00_00, 8); // MPEG-1, 48 kHz, stereo, no extension
    out.write(u64::from(index), 1);
    if index == 1 {
        out.write(0, 8);
    }
    out.write(0, 59);
    out.write(0, 59);
    out.into_bytes()
}

fn stream(frames: &[Vec<u8>]) -> Vec<u8> {
    let payload: Vec<u8> = frames.concat();
    let size = 8 + payload.len();
    let mut data = vec![0x48, 0, 0, 0x0c, 0x15, 0x04, 0xbb, 0x80, 0x40, 0x00, 0x04, 0x00];
    data.extend_from_slice(&[b'D', 0, (size >> 8) as u8, size as u8]);
    data.extend_from_slice(&1152u32.to_be_bytes());
    data.extend_from_slice(&payload);
    data
}

#[test]
fn two_granules_make_one_mpeg1_frame() {
    let mp3 = to_mp3(&stream(&[empty_granule(0), empty_granule(1)])).unwrap();
    assert_eq!(mp3.cores[0].frames, 1);
    // 4-byte header, 32 bytes of side information, no main data: every frame is 320 kbit/s at 48 kHz = 960 bytes.
    assert_eq!(mp3.cores[0].bytes.len(), 960);
    assert_eq!(&mp3.cores[0].bytes[..4], &[0xFF, 0xFB, 0xE4, 0x00], "MPEG-1 Layer III, 320 kbit/s, 48 kHz, stereo");
}

#[test]
fn a_lone_granule_is_paired_with_silence() {
    let mp3 = to_mp3(&stream(&[empty_granule(0)])).unwrap();
    assert_eq!(mp3.cores[0].frames, 1);
}

#[test]
fn main_data_bits_are_copied_after_the_side_information() {
    // Channel 0 claims 16 main data bits (0xA5 0x5A), channel 1 none.
    let mut out = BitWriter::default();
    out.write(0, 8);
    out.write(0b11_01_00_00, 8);
    out.write(0, 1);
    out.write(16u64 << 47, 59);
    out.write(0, 59);
    out.write(0xA55A, 16);
    let mp3 = to_mp3(&stream(&[out.into_bytes(), empty_granule(1)])).unwrap();
    assert_eq!(mp3.cores[0].frames, 1);
    let side_end = 4 + 32;
    assert_eq!(&mp3.cores[0].bytes[side_end..side_end + 2], &[0xA5, 0x5A]);
}

#[test]
fn a_stream_in_another_codec_is_refused() {
    let mut data = stream(&[empty_granule(0)]);
    data[4] = 0x14; // codec 4
    assert!(matches!(to_mp3(&data), Err(Error::UnsupportedCodec(4))));
}

#[test]
fn a_truncated_frame_is_an_error() {
    let mut frame = empty_granule(0);
    frame.truncate(10);
    assert!(to_mp3(&stream(&[frame])).is_err());
}

#[test]
fn the_frames_of_a_multichannel_stream_alternate_between_its_cores() {
    // Six channels: three cores, each given one granule 0 and one granule 1, interleaved core by core.
    let mut frames = Vec::new();
    for index in [0, 1] {
        for _ in 0..3 {
            frames.push(empty_granule(index));
        }
    }
    let mut data = stream(&frames);
    data[5] = 0x15;
    data[6] = 0x14;
    let mp3 = to_mp3(&data).unwrap();
    assert_eq!(mp3.header.channels, 6);
    assert_eq!(mp3.cores.iter().map(|core| (core.channels, core.frames)).collect::<Vec<_>>(), [(2, 1), (2, 1), (2, 1)]);
}
