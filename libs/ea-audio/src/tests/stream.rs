use crate::codec::microtalk;
use crate::error::Error;
use crate::schl::{self, Blocks, Codec, Stream, StreamReader};
use crate::tests::build::*;

fn mono_pc_header() -> Vec<u8> {
    pt_header(0, &[tag(0x80, 2), tag(0x84, 22050), tag(0x85, 56)])
}

fn two_block_mono() -> Vec<u8> {
    let first = xa_pcm_frame(1270, 0, &ramp(1000));
    let mut second = xa_adpcm_frame(0x10, 0x0);
    second.extend(xa_adpcm_frame(0x10, 0x0));
    stream(&mono_pc_header(), &[scdl(28, &[0], &first, false), scdl(28, &[0], &second, false)])
}

#[test]
fn blocks_are_walked_up_to_the_end_tag() {
    let bytes = two_block_mono();
    let tags: Vec<[u8; 4]> = Blocks::new(&bytes).map(|b| b.unwrap().tag).collect();
    assert_eq!(tags, [*b"SCHl", *b"SCCl", *b"SCDl", *b"SCDl", *b"SCEl"]);
}

#[test]
fn a_block_walk_reports_truncation_and_garbage() {
    let bytes = two_block_mono();
    let cut = &bytes[..bytes.len() - 20];
    assert!(Blocks::new(cut).any(|b| matches!(b, Err(Error::Truncated { .. }))));
    let mut zeros = bytes.clone();
    zeros.truncate(zeros.len() - 8);
    zeros.extend([0u8; 16]);
    assert!(Blocks::new(&zeros).any(|b| matches!(b, Err(Error::Corrupt(_)))));
    let tiny = [b'S', b'C', b'D', b'l', 4, 0, 0, 0];
    assert!(matches!(Blocks::new(&tiny).next(), Some(Err(Error::Corrupt(_)))));
}

#[test]
fn decoding_carries_the_adpcm_history_across_blocks() {
    let pcm = schl::decode_stream(&two_block_mono()).unwrap();
    assert_eq!((pcm.sample_rate, pcm.channels, pcm.frames()), (22050, 1, 56));
    assert_eq!(&pcm.samples[..28], &ramp(1000));
    // Second block: pair 1 (240, 0) applied to h1 = 1270 left by the first block's frame header.
    assert_eq!(pcm.samples[28], ((240 * 1270) >> 8) as i16);
}

#[test]
fn the_stream_api_exposes_header_and_blocks() {
    let bytes = two_block_mono();
    let stream = Stream::parse(&bytes).unwrap();
    assert_eq!(stream.header().sample_count, 56);
    assert_eq!(stream.blocks().count(), 4);
    assert_eq!(stream.decode().unwrap().frames(), 56);
    // Trailing bytes after the end block are ignored.
    let mut longer = bytes.clone();
    longer.extend([0xAA; 40]);
    assert_eq!(schl::decode_stream(&longer).unwrap().frames(), 56);
    assert_eq!(crate::decode(&longer).unwrap().frames(), 56);
}

#[test]
fn wrong_magic_is_rejected() {
    assert!(matches!(Stream::parse(&block(b"SCDl", &[0; 8])), Err(Error::BadMagic { .. })));
    assert!(matches!(crate::decode(b"WXYZ1234"), Err(Error::BadMagic { .. })));
    assert!(matches!(crate::decode(b"ABKC1234"), Err(Error::Unsupported(_))));
    assert!(Stream::parse(&[]).is_err());
}

#[test]
fn stereo_channels_have_their_own_offsets_and_interleave() {
    let left = xa_pcm_frame(0, 0, &ramp(0));
    let right = xa_pcm_frame(0, 0, &ramp(5000));
    let mut data = left.clone();
    data.extend(&right);
    let header = pt_header(0, &[tag(0x80, 2), tag(0x82, 2), tag(0x85, 28)]);
    let bytes = stream(&header, &[scdl(28, &[0, left.len() as u32], &data, false)]);
    let pcm = schl::decode_stream(&bytes).unwrap();
    assert_eq!(pcm.channels, 2);
    assert_eq!(pcm.frames(), 28);
    for i in 0..28 {
        assert_eq!(pcm.samples[2 * i], ramp(0)[i]);
        assert_eq!(pcm.samples[2 * i + 1], ramp(5000)[i]);
    }
}

#[test]
fn generic_streams_are_big_endian_inside_the_blocks() {
    let frame = xa_pcm_frame(0, 0, &ramp(-300));
    let header = gstr_header(&[tag(0x80, 3), tag(0x84, 36000), tag(0x85, 28)]);
    let bytes = stream(&header, &[scdl(28, &[0], &frame, true)]);
    let pcm = schl::decode_stream(&bytes).unwrap();
    assert_eq!(pcm.sample_rate, 36000);
    assert_eq!(&pcm.samples, &ramp(-300));
}

#[test]
fn old_adpcm_streams_skip_four_history_bytes_per_channel() {
    // Version 1 on PC: offsets present, revision-1 frames, 4 bytes of history in front of each channel.
    let mut data = vec![0xAA; 4];
    data.extend(xa_adpcm_frame(0x10, 0x0));
    let header = pt_header(0, &[tag(0x80, 1), tag(0x85, 28)]);
    let bytes = stream(&header, &[scdl(28, &[0], &data, false)]);
    let pcm = schl::decode_stream(&bytes).unwrap();
    assert_eq!(pcm.samples, vec![0; 28]);
}

#[test]
fn version_zero_streams_use_the_codec_layout() {
    // Version 0 split mono EA-XA, two channels: 8 bytes of history, then channel 0's frames, then channel 1's.
    let mut data = vec![0u8; 8];
    data.extend(xa_adpcm_frame(0x10, 0x0));
    data.extend(xa_adpcm_frame(0x20, 0x0));
    let header = pt_header(0, &[tag(0x82, 2), tag(0xA0, 0x0A), tag(0x85, 28)]);
    let bytes = stream(
        &header,
        &[block(b"SCDl", &{
            let mut p = 28u32.to_le_bytes().to_vec();
            p.extend(data);
            p
        })],
    );
    let pcm = schl::decode_stream(&bytes).unwrap();
    assert_eq!((pcm.channels, pcm.frames()), (2, 28));
}

#[test]
fn unsupported_codecs_are_reported_by_id() {
    let header = pt_header(0, &[tag(0xA0, 0x17), tag(0x85, 28)]);
    let bytes = stream(&header, &[scdl(28, &[0], &[0; 20], false)]);
    assert_eq!(schl::decode_stream(&bytes).unwrap_err(), Error::UnsupportedCodec(0x17));
}

#[test]
fn bad_blocks_are_errors_not_panics() {
    let header = mono_pc_header();
    let overrun = stream(&header, &[scdl(28, &[400], &[0; 15], false)]);
    assert!(matches!(schl::decode_stream(&overrun), Err(Error::Corrupt(_))));
    let short = stream(&header, &[scdl(28, &[0], &[0x10; 7], false)]);
    assert!(matches!(schl::decode_stream(&short), Err(Error::Truncated { .. })));
    let absurd = stream(&header, &[scdl(u32::MAX, &[0], &[0; 15], false)]);
    assert!(matches!(schl::decode_stream(&absurd), Err(Error::Corrupt(_))));
    let empty = stream(&header, &[scdl(0, &[0], &[], false)]);
    assert_eq!(schl::decode_stream(&empty).unwrap().frames(), 0);
}

#[test]
fn microtalk_blocks_decode_silent_frames() {
    // Codec2 4, PCM-block revision: flag byte, then per frame a marker byte and the bit data.
    let header = pt_header(0, &[tag(0x80, 2), tag(0xA0, 4), tag(0x85, 432)]);
    let mut data = vec![1u8];
    data.extend([0u8; 200]);
    let bytes = stream(&header, &[scdl(432, &[0], &data, false)]);
    let pcm = schl::decode_stream(&bytes).unwrap();
    assert_eq!(pcm.frames(), 432);
    assert_eq!(Stream::parse(&bytes).unwrap().header().codec, Codec::MicroTalk);
    assert!(pcm.samples.iter().all(|&s| s == 0));
    assert_eq!(microtalk::FRAME_SAMPLES, 432);
}

#[test]
fn a_reader_decodes_block_by_block_from_a_source() {
    let mut bytes = vec![0u8; 0x100];
    bytes.extend(two_block_mono());
    let mut reader = StreamReader::open(&bytes[..], 0x100).unwrap();
    assert_eq!(reader.header().sample_rate, 22050);
    let mut out = Vec::new();
    assert_eq!(reader.next_chunk(&mut out).unwrap(), Some(28));
    assert_eq!(out.len(), 28);
    assert_eq!(reader.next_chunk(&mut out).unwrap(), Some(28));
    assert_eq!(reader.next_chunk(&mut out).unwrap(), None);
    assert_eq!(reader.next_chunk(&mut out).unwrap(), None);
    assert_eq!(out, schl::decode_stream(&bytes[0x100..]).unwrap().samples);
    let all = StreamReader::open(&bytes[..], 0x100).unwrap().read_all().unwrap();
    assert_eq!(all.samples, out);
}

#[test]
fn extent_measures_a_stream_from_its_block_prefixes() {
    let bytes = two_block_mono();
    let mut padded = bytes.clone();
    padded.extend([0u8; 100]);
    assert_eq!(schl::extent(&padded[..], 0).unwrap(), bytes.len() as u64);
    assert!(schl::extent(&padded[..], 3).is_err());
    assert!(schl::extent(&bytes[..bytes.len() - 4], 0).is_err());
    assert!(StreamReader::open(&bytes[..], 4).is_err());
}

#[test]
fn loops_are_reported_in_frames() {
    let header = pt_header(0, &[tag(0x80, 2), tag(0x85, 28), tag(0x86, 4), tag(0x87, 19)]);
    let bytes = stream(&header, &[scdl(28, &[0], &xa_pcm_frame(0, 0, &ramp(0)), false)]);
    assert_eq!(schl::decode_stream(&bytes).unwrap().loop_range, Some((4, 20)));
}
