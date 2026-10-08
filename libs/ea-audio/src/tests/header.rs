use crate::error::Error;
use crate::schl::header::{self, Codec, PLATFORM_GENERIC};
use crate::tests::build::*;

#[test]
fn pc_header_reads_the_usual_tags() {
    let bytes = pt_header(0, &[tag(0x06, 101), tag(0x80, 2), tag(0x82, 2), tag(0x84, 44100), tag(0x85, 1_321_022)]);
    let h = header::parse(&bytes).unwrap();
    assert_eq!(h.platform, 0);
    assert_eq!(h.version, 2);
    assert_eq!(h.channels, 2);
    assert_eq!(h.sample_rate, 44100);
    assert_eq!(h.sample_count, 1_321_022);
    assert_eq!(h.priority, Some(101));
    assert_eq!(h.codec, Codec::EaXa);
    assert!(!h.big_endian);
    assert!(h.pcm_blocks, "version 2 on PC is the PCM-block revision");
    assert_eq!(h.loop_range(), None);
}

#[test]
fn missing_tags_take_the_platform_defaults() {
    let h = header::parse(&pt_header(0, &[tag(0x85, 100)])).unwrap();
    assert_eq!((h.version, h.channels, h.sample_rate), (0, 1, 22050));
    assert!(!h.pcm_blocks);
    let h = header::parse(&gstr_header(&[tag(0x85, 100)])).unwrap();
    assert_eq!((h.platform, h.version, h.sample_rate), (PLATFORM_GENERIC, 2, 48000));
    assert!(h.big_endian);
    assert!(h.pcm_blocks);
}

#[test]
fn a_zero_sample_rate_means_the_default() {
    let h = header::parse(&pt_header(0, &[tag(0x84, 0)])).unwrap();
    assert_eq!(h.sample_rate, 22050);
}

#[test]
fn codec_tags_pick_the_decoder() {
    let parse = |tags: &[Vec<u8>]| header::parse(&pt_header(0, tags)).unwrap().codec;
    assert_eq!(parse(&[tag(0xA0, 4)]), Codec::MicroTalk);
    assert_eq!(parse(&[tag(0xA0, 0x16)]), Codec::MicroTalk);
    assert_eq!(parse(&[tag(0xA0, 0x0A)]), Codec::EaXa);
    assert_eq!(parse(&[tag(0xA0, 0x03)]), Codec::EaXaStereo);
    assert_eq!(parse(&[tag(0xA0, 0x17)]), Codec::Other(0x17));
    assert_eq!(parse(&[tag(0x83, 9)]), Codec::MicroTalk);
    assert_eq!(parse(&[tag(0x83, 7)]), Codec::EaXaStereo, "codec1 EA-XA on PC is the stereo flavour");
    assert_eq!(parse(&[tag(0x83, 0)]), Codec::Other(0));
    assert_eq!(parse(&[tag(0x83, 9), tag(0xA0, 0x0A)]), Codec::EaXa, "codec2 wins over codec1");
}

#[test]
fn loop_tags_give_a_half_open_range() {
    let h = header::parse(&pt_header(0, &[tag(0x86, 1000), tag(0x87, 4999)])).unwrap();
    assert_eq!(h.loop_range(), Some((1000, 5000)));
    let h = header::parse(&pt_header(0, &[tag(0x87, 99)])).unwrap();
    assert_eq!(h.loop_range(), Some((0, 100)));
}

#[test]
fn channel_offsets_land_in_their_slots() {
    let tags = [tag(0x88, 10), tag(0x89, 20), tag(0x94, 30), tag(0xA3, 60)];
    let h = header::parse(&pt_header(0, &tags)).unwrap();
    assert_eq!(h.channel_offsets, [Some(10), Some(20), Some(30), None, None, Some(60)]);
}

#[test]
fn valueless_tags_and_unknown_tags_are_skipped() {
    let mut bytes = vec![b'P', b'T', 0, 0, 0xFC, 0xFD];
    bytes.extend(tag(0x77, 0x1234)); // unknown tag with a 2-byte value
    bytes.extend([0x8A, 4, 0, 0, 0, 0]); // padding
    bytes.extend([0x14, 6, 1, 2, 3, 4, 5, 6]); // user data longer than 4 bytes
    bytes.extend([0x14, 0xFF, 0, 0, 0, 2, 9, 9]); // 32-bit sized user data (4 + 2 bytes after the 0xFF)
    bytes.extend([0x00, 0]); // tag 0 with an empty value
    bytes.extend(tag(0x85, 77));
    bytes.push(0xFF);
    assert_eq!(header::parse(&bytes).unwrap().sample_count, 77);
}

#[test]
fn tag_fe_ends_the_header() {
    let mut bytes = pt_header(0, &[tag(0x85, 5)]);
    bytes.pop();
    bytes.push(0xFE);
    bytes.extend(tag(0x85, 99)); // belongs to a sub-section and is ignored
    assert_eq!(header::parse(&bytes).unwrap().sample_count, 5);
}

#[test]
fn bad_headers_are_errors() {
    assert_eq!(header::parse(b"XXXX").unwrap_err(), Error::BadMagic { expected: "PT or GSTR" });
    assert!(matches!(header::parse(b"PT"), Err(Error::Truncated { .. })));
    assert!(matches!(header::parse(&[b'P', b'T', 0, 0, 0x85]), Err(Error::Truncated { .. })));
    assert!(matches!(header::parse(&[b'P', b'T', 0, 0, 0x85, 4, 0]), Err(Error::Truncated { .. })));
    let seven = pt_header(0, &[tag(0x82, 7)]);
    assert_eq!(header::parse(&seven).unwrap_err(), Error::BadHeader("more than 6 channels"));
    let unknown_platform = pt_header(0x40, &[]);
    assert_eq!(header::parse(&unknown_platform).unwrap_err(), Error::BadHeader("unknown platform"));
}
