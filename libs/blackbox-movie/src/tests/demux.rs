use super::build::{block, mvhd, scdl};
use crate::{Demuxer, MovieError, Packet};

fn with_audio_header() -> Vec<u8> {
    let mut m = mvhd(1, 25, 1);
    m.extend(block(b"SCHl", b"GSTR"));
    m
}

fn sample_movie() -> Vec<u8> {
    let mut m = mvhd(3, 30000, 1000);
    m.extend(block(b"SCHl", b"GSTR header bytes"));
    m.extend(block(b"MV0K", &[0x01, 0x02]));
    m.extend(block(b"SCCl", &2u32.to_be_bytes()));
    m.extend(block(b"MV0F", &[0x80]));
    m.extend(scdl(1600, 1));
    m.extend(block(b"MV0F", &[0x81]));
    m.extend(scdl(0x0100_0005, 2));
    m.extend(block(b"SCEl", &[]));
    m.extend(block(b"ZZZZ", &[9, 9]));
    m
}

#[test]
fn reads_header_and_audio_header() {
    let m = sample_movie();
    let d = Demuxer::new(&m[..]).unwrap();
    let h = d.header();
    assert_eq!((h.width, h.height, h.frame_count), (64, 32, 3));
    assert!(h.is_vp6());
    assert!((h.fps() - 30.0).abs() < 1e-9);
    assert_eq!(d.audio_header(), Some(&b"GSTR header bytes"[..]));
}

#[test]
fn yields_packets_in_file_order() {
    let m = sample_movie();
    let packets: Vec<Packet> = Demuxer::new(&m[..]).unwrap().map(Result::unwrap).collect();
    assert_eq!(packets.len(), 8);
    let Packet::Video(k) = &packets[0] else { panic!("{:?}", packets[0]) };
    assert!(k.key);
    assert_eq!((k.index, k.data.as_slice()), (0, &[1, 2][..]));
    assert_eq!(packets[1], Packet::AudioCount(2));
    let Packet::Video(f) = &packets[2] else { panic!() };
    assert!(!f.key);
    assert_eq!(f.index, 1);
    assert!((f.time - 1.0 / 30.0).abs() < 1e-9);
    let Packet::Audio(a0) = &packets[3] else { panic!() };
    assert_eq!((a0.index, a0.first_sample, a0.samples), (0, 0, 1600));
    assert_eq!(a0.data.len(), 12);
    let Packet::Video(f2) = &packets[4] else { panic!() };
    assert_eq!(f2.index, 2);
    let Packet::Audio(a1) = &packets[5] else { panic!() };
    assert_eq!((a1.index, a1.first_sample, a1.samples), (1, 1600, 5));
    assert_eq!(packets[6], Packet::AudioEnd);
    assert_eq!(packets[7], Packet::Unknown { tag: *b"ZZZZ", data: vec![9, 9] });
}

#[test]
fn movie_without_audio_header_keeps_the_next_block() {
    let mut m = mvhd(1, 25, 1);
    m.extend(block(b"MV0K", &[7]));
    let mut d = Demuxer::new(&m[..]).unwrap();
    assert_eq!(d.audio_header(), None);
    let Some(Packet::Video(v)) = d.next_packet().unwrap() else { panic!() };
    assert!(v.key);
    assert_eq!(d.next_packet().unwrap(), None);
}

#[test]
fn header_only_movie_is_empty() {
    let m = mvhd(0, 25, 1);
    let mut d = Demuxer::new(&m[..]).unwrap();
    assert_eq!(d.next_packet().unwrap(), None);
}

#[test]
fn rejects_data_that_is_not_a_movie() {
    let m = block(b"RIFF", &[0; 24]);
    assert!(matches!(Demuxer::new(&m[..]), Err(MovieError::NotAMovie(t)) if &t == b"RIFF"));
    assert!(matches!(Demuxer::new(&[][..]), Err(MovieError::Truncated { .. })));
}

#[test]
fn rejects_a_zero_frame_rate() {
    let m = mvhd(1, 0, 1);
    assert!(matches!(Demuxer::new(&m[..]), Err(MovieError::BadFrameRate { rate: 0, .. })));
}

#[test]
fn rejects_a_short_header() {
    let m = block(b"MVhd", &[0; 10]);
    assert!(matches!(Demuxer::new(&m[..]), Err(MovieError::ShortPayload { .. })));
}

#[test]
fn truncated_block_is_an_error_then_the_iterator_stops() {
    let mut m = with_audio_header();
    let mut cut = block(b"MV0K", &[1, 2, 3, 4]);
    cut.truncate(cut.len() - 2);
    m.extend(cut);
    let mut d = Demuxer::new(&m[..]).unwrap();
    assert!(matches!(d.next(), Some(Err(MovieError::Truncated { .. }))));
    assert!(d.next().is_none());
}

#[test]
fn truncated_block_header_is_an_error() {
    let mut m = with_audio_header();
    m.extend_from_slice(b"MV0F\x10");
    let mut d = Demuxer::new(&m[..]).unwrap();
    assert!(matches!(d.next_packet(), Err(MovieError::Truncated { got: 5, .. })));
}

#[test]
fn block_size_below_eight_is_an_error() {
    let mut m = with_audio_header();
    m.extend_from_slice(b"MV0F\x04\x00\x00\x00");
    let mut d = Demuxer::new(&m[..]).unwrap();
    assert!(matches!(d.next_packet(), Err(MovieError::BadBlockSize { size: 4, .. })));
}

#[test]
fn huge_declared_size_does_not_allocate_it() {
    let mut m = with_audio_header();
    m.extend_from_slice(b"MV0F\xF0\xFF\xFF\xFF\x01\x02");
    let mut d = Demuxer::new(&m[..]).unwrap();
    assert!(matches!(d.next_packet(), Err(MovieError::Truncated { got: 10, .. })));
}

#[test]
fn audio_block_without_a_sample_count_is_an_error() {
    let mut m = with_audio_header();
    m.extend(block(b"SCDl", &[1, 2]));
    let mut d = Demuxer::new(&m[..]).unwrap();
    assert!(matches!(d.next_packet(), Err(MovieError::ShortPayload { .. })));
}
