use crate::error::Error;
use crate::mus::Mpf;
use crate::tests::build::*;

fn put32(v: &mut Vec<u8>, at: usize, value: u32) {
    if v.len() < at + 4 {
        v.resize(at + 4, 0);
    }
    v[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

const TRACKS_TABLE: usize = 0x100;
const TRACK_ENTRY: usize = 0x108;
const SAMPLES: usize = 0x140;

/// A version 5 map with two tracks and four streams; the offsets are in 0x80 units.
fn mpf() -> Vec<u8> {
    let mut v = vec![0u8; 0x200];
    v[..4].copy_from_slice(b"xDFP");
    v[4] = 5;
    v[5] = 1;
    v[0x0D] = 2;
    put32(&mut v, 0x2C, TRACKS_TABLE as u32);
    put32(&mut v, 0x34, SAMPLES as u32);
    put32(&mut v, 0x38, (SAMPLES + 4 * 8) as u32);
    // Track entries live at TRACK_ENTRY and TRACK_ENTRY + 0x20; the table holds their offsets / 4.
    put32(&mut v, TRACKS_TABLE, (TRACK_ENTRY / 4) as u32);
    put32(&mut v, TRACKS_TABLE + 4, ((TRACK_ENTRY + 0x20) / 4) as u32);
    put32(&mut v, TRACK_ENTRY, 0);
    v[TRACK_ENTRY + 8..TRACK_ENTRY + 12].copy_from_slice(&[0xFA, 0xCE, 0xA5, 0x8C]);
    put32(&mut v, TRACK_ENTRY + 0x20, 2);
    v[TRACK_ENTRY + 0x20 + 4] = 3; // sub-banks: a RAM track
    for (k, (offset, ms)) in [(2u32, 1500u32), (4, 2500), (6, 3500), (8, 4500)].into_iter().enumerate() {
        put32(&mut v, SAMPLES + 8 * k, offset);
        put32(&mut v, SAMPLES + 8 * k + 4, ms);
    }
    v
}

/// A `.mus` with the checksum word first and streams of different lengths at offsets 0x100 and 0x200.
fn mus_file() -> Vec<u8> {
    let header = gstr_header(&[tag(0x80, 3), tag(0x84, 36000), tag(0x85, 28)]);
    let one = stream(&header, &[scdl(28, &[0], &xa_pcm_frame(0, 0, &ramp(7)), true)]);
    let two = stream(&header, &vec![scdl(28, &[0], &xa_pcm_frame(0, 0, &ramp(9)), true); 2]);
    let mut file = vec![0xFA, 0xCE, 0xA5, 0x8C];
    file.resize(0x100, 0);
    file.extend(one);
    file.resize(0x200, 0);
    file.extend(two);
    file
}

#[test]
fn the_map_lists_tracks_and_streams() {
    let mpf = Mpf::parse(&mpf()).unwrap();
    assert_eq!((mpf.version, mpf.sub_version), (5, 1));
    assert_eq!(mpf.tracks.len(), 2);
    assert_eq!(mpf.tracks[0].checksum, 0xFACE_A58C);
    assert_eq!(mpf.tracks[1].first_stream, 2);
    assert_eq!(mpf.tracks[1].sub_banks, 3);
    assert_eq!(mpf.streams.len(), 4);
    let s = mpf.streams[1];
    assert_eq!((s.raw, s.offset, s.duration_ms), (4, 0x200, 2500));
    assert_eq!(mpf.streams.iter().map(|s| s.track).collect::<Vec<_>>(), [0, 0, 1, 1]);
    assert!((mpf.total_secs() - 12.0).abs() < 1e-9);
}

#[test]
fn the_checksum_ties_the_map_to_its_stream_file() {
    let mpf = Mpf::parse(&mpf()).unwrap();
    assert!(mpf.matches_mus(&mus_file()));
    assert!(!mpf.matches_mus(&[0, 1, 2, 3]));
    assert!(!mpf.matches_mus(&[0xFA]));
}

#[test]
fn streams_are_measured_and_decoded_from_the_stream_file() {
    let mpf = Mpf::parse(&mpf()).unwrap();
    let mus = mus_file();
    // Streams 0 and 1 sit at 0x100 and 0x200 (raw 2 and 4 times 0x80); 2 and 3 point past the end of this tiny file.
    assert_eq!(mpf.stream_len(&mus[..], 0).unwrap() as usize, one_len());
    assert_eq!(mpf.stream_len(&mus[..], 1).unwrap() as usize, mus.len() - 0x200);
    assert!(mpf.stream_len(&mus[..], 2).is_err());
    let pcm = mpf.decode(&mus[..], 1).unwrap();
    assert_eq!((pcm.sample_rate, pcm.frames()), (36000, 56));
    assert_eq!(&pcm.samples[..28], &ramp(9));
    let mut reader = mpf.open(&mus[..], 0).unwrap();
    let mut out = Vec::new();
    assert_eq!(reader.next_chunk(&mut out).unwrap(), Some(28));
    assert_eq!(&out, &ramp(7));
    assert_eq!(reader.next_chunk(&mut out).unwrap(), None);
}

fn one_len() -> usize {
    let header = gstr_header(&[tag(0x80, 3), tag(0x84, 36000), tag(0x85, 28)]);
    stream(&header, &[scdl(28, &[0], &xa_pcm_frame(0, 0, &ramp(7)), true)]).len()
}

#[test]
fn bad_maps_and_indexes_are_errors() {
    let good = mpf();
    assert!(matches!(Mpf::parse(&good[..2]), Err(Error::BadMagic { .. })));
    assert!(matches!(Mpf::parse(b"PFDx0000"), Err(Error::BadMagic { .. })));
    let mut v4 = good.clone();
    v4[4] = 4;
    assert!(matches!(Mpf::parse(&v4), Err(Error::Unsupported(_))));
    let mut outside = good.clone();
    put32(&mut outside, 0x38, 0x10_0000);
    assert!(matches!(Mpf::parse(&outside), Err(Error::Corrupt(_))));
    assert!(Mpf::parse(&good[..0x120]).is_err());
    let mpf = Mpf::parse(&good).unwrap();
    assert_eq!(mpf.decode(&mus_file()[..], 9).unwrap_err(), Error::NoSuchEntry(9));
    assert_eq!(mpf.stream_len(&mus_file()[..], 9).unwrap_err(), Error::NoSuchEntry(9));
    assert!(mpf.open(&mus_file()[..], 9).is_err());
}
