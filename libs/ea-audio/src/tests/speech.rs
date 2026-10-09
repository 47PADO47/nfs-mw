use crate::error::Error;
use crate::speech::{BankHeader, SAMPLE_UNIT, SpeechIndex};
use crate::tests::build::*;

/// A bank header: event, speaker, flags, `starts` in units (the first take starts at 0 and is not listed), `extra`.
fn header(event: u16, speaker: u16, flags: u8, units: &[u16], extra: &[u8]) -> Vec<u8> {
    let count = units.len() as u8 + 1;
    let mut out = event.to_le_bytes().to_vec();
    out.extend(speaker.to_le_bytes());
    out.extend([flags, count, count, 0, 0x12, 0, 0, 0, 0, 0]);
    for &u in units {
        out.extend(std::iter::repeat_n(0xAA, usize::from(flags)));
        out.extend(u.to_be_bytes());
    }
    out.extend(extra);
    out
}

/// An index file: `banks` are (header, position of the takes in the `.big`).
fn index(banks: &[(Vec<u8>, u32)]) -> Vec<u8> {
    let mut out = vec![0u8; 0x58];
    out[0] = 1;
    out[4..8].copy_from_slice(&(banks.len() as u32).to_le_bytes());
    out[0x54..0x58].copy_from_slice(&(banks.len() as u32).to_le_bytes());
    let mut at = 0x58 + 16 * banks.len();
    let mut headers: Vec<u8> = Vec::new();
    for (n, (hdr, base)) in banks.iter().enumerate() {
        out.extend((0x0100_0000 + n as u32).to_le_bytes());
        out.extend((hdr.len() as u32).to_le_bytes());
        out.extend((at as u32).to_le_bytes());
        out.extend(base.to_le_bytes());
        at += hdr.len();
        headers.extend(hdr.iter());
    }
    out.extend(headers);
    out
}

#[test]
fn a_header_lists_the_takes_whatever_the_entry_width() {
    for flags in 0..=3 {
        let h = BankHeader::parse(&header(0x1A9, 4, flags, &[9, 16, 22], &[0x70, 0x70])).unwrap();
        assert_eq!((h.event, h.speaker, h.takes()), (0x1A9, 4, 4), "flags {flags}");
        assert_eq!(h.starts, [0, 9 * SAMPLE_UNIT, 16 * SAMPLE_UNIT, 22 * SAMPLE_UNIT]);
        assert_eq!(h.extra, [0x70, 0x70]);
    }
}

#[test]
fn a_bank_of_one_take_has_no_table() {
    let h = BankHeader::parse(&header(8, 0xFFFF, 1, &[], &[])).unwrap();
    assert_eq!((h.speaker, h.starts), (0xFFFF, vec![0]));
}

#[test]
fn damaged_headers_are_errors() {
    let good = header(8, 1, 0, &[5, 9], &[]);
    assert!(matches!(BankHeader::parse(&good[..good.len() - 1]), Err(Error::Truncated { .. })));
    assert!(matches!(BankHeader::parse(&good[..3]), Err(Error::Truncated { .. })));
    let mut empty = good.clone();
    empty[5] = 0;
    assert!(matches!(BankHeader::parse(&empty), Err(Error::BadHeader(_))));
    assert!(matches!(BankHeader::parse(&header(8, 1, 0, &[9, 5], &[])), Err(Error::Corrupt(_))));
    assert!(matches!(BankHeader::parse(&header(8, 1, 0, &[0], &[])), Err(Error::Corrupt(_))));
}

#[test]
fn the_index_finds_banks_by_event() {
    let file = index(&[
        (header(10, 3, 0, &[4], &[]), 0x1000),
        (header(11, 3, 1, &[], &[]), 0x2000),
        (header(10, 4, 0, &[2, 5], &[0x70]), 0x3000),
    ]);
    let idx = SpeechIndex::parse(&file).unwrap();
    assert_eq!(idx.banks().len(), 3);
    assert_eq!(idx.take_count(), 2 + 1 + 3);
    let speakers: Vec<u16> = idx.banks_for(10).map(|b| b.header.speaker).collect();
    assert_eq!(speakers, [3, 4]);
    assert_eq!(idx.banks_for(99).count(), 0);
    let third = &idx.banks()[2];
    assert_eq!((third.kind, third.number, third.base), (1, 2, 0x3000));
    assert_eq!(third.take_offset(2), Some(0x3000 + 5 * SAMPLE_UNIT));
    assert_eq!(third.take_offset(3), None);
}

#[test]
fn an_index_that_points_outside_itself_is_an_error() {
    let mut file = index(&[(header(10, 3, 0, &[4], &[]), 0)]);
    file.truncate(file.len() - 1);
    assert!(matches!(SpeechIndex::parse(&file), Err(Error::Truncated { .. })));
    assert!(SpeechIndex::parse(&[0u8; 8]).is_err());
}

#[test]
fn takes_decode_from_the_big() {
    let stream_of = |start: i16, blocks: usize| {
        let header = pt_header(0, &[tag(0x80, 2), tag(0x85, 28 * blocks as u32)]);
        let frame = xa_pcm_frame(0, 0, &ramp(start));
        let data: Vec<Vec<u8>> = (0..blocks).map(|_| scdl(28, &[0], &frame, false)).collect();
        stream(&header, &data)
    };
    // A bank whose first take sits at 0x100 and whose second starts one unit later.
    let mut big = vec![0u8; 0x100];
    big.extend(stream_of(1, 1));
    big.resize(0x200, 0);
    big.extend(stream_of(2, 2));
    let file = index(&[(header(10, 3, 0, &[1], &[]), 0x100)]);
    let idx = SpeechIndex::parse(&file).unwrap();
    let bank = &idx.banks()[0];
    assert_eq!(bank.decode(&big[..], 0).unwrap().samples, ramp(1));
    assert_eq!(bank.decode(&big[..], 1).unwrap().frames(), 56);
    assert_eq!(bank.decode(&big[..], 2).unwrap_err(), Error::NoSuchEntry(2));
}
