use crate::abk::{Bank, SoundKind};
use crate::error::Error;
use crate::tests::build::*;

fn put(v: &mut Vec<u8>, at: usize, bytes: &[u8]) {
    if v.len() < at + bytes.len() {
        v.resize(at + bytes.len(), 0);
    }
    v[at..at + bytes.len()].copy_from_slice(bytes);
}

fn put32(v: &mut Vec<u8>, at: usize, value: u32) {
    put(v, at, &value.to_le_bytes());
}

const BNK: usize = 0x300;

/// One module with two players. Both point at the first sample table, except the second, which points at its own.
/// The `BNKl` holds a dummy, a mono sound and a looping stereo sound.
fn bank() -> Vec<u8> {
    let mut v = vec![0u8; 0x800];
    put(&mut v, 0, b"ABKC");
    put(&mut v, 4, &[1, 1, 1, 0]);
    put(&mut v, 0x0A, &1u16.to_le_bytes());
    put32(&mut v, 0x1C, 0x40);
    put32(&mut v, 0x20, BNK as u32);
    // Module at 0x40: three players, one class controller.
    v[0x40 + 0x24] = 3;
    v[0x40 + 0x27] = 1;
    put32(&mut v, 0x40 + 0x2C, 0x100);
    for (j, offset) in [0u32, 0x10, 0x20].into_iter().enumerate() {
        put32(&mut v, 0x40 + 0x3C + 4 * j, offset);
    }
    // Player records at 0x100: +4 is the sample table.
    put32(&mut v, 0x104, 0x200);
    put32(&mut v, 0x114, 0x200);
    put32(&mut v, 0x124, 0x240);
    // Sample table 1: a dummy and two RAM entries.
    put32(&mut v, 0x200, 3);
    put(&mut v, 0x204, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    put(&mut v, 0x210, &[0, 7, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0]);
    put(&mut v, 0x21C, &[0, 7, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0]);
    // Sample table 2: one streamed entry.
    put32(&mut v, 0x240, 1);
    put(&mut v, 0x244, &[1, 3, 0, 0, 0x34, 0x12, 0, 0, 0x78, 0x56, 0, 0]);
    // BNKl: version 5, three entries.
    put(&mut v, BNK, b"BNKl");
    v[BNK + 4] = 5;
    put(&mut v, BNK + 6, &3u16.to_le_bytes());
    let (h1, h2) = (0x40usize, 0x80usize);
    put32(&mut v, BNK + 0x14, 0);
    put32(&mut v, BNK + 0x18, (h1 - 0x18) as u32);
    put32(&mut v, BNK + 0x1C, (h2 - 0x1C) as u32);
    let mono = pt_header(0, &[tag(0x80, 2), tag(0x85, 28), tag(0x84, 44100), tag(0x88, 0x100)]);
    put(&mut v, BNK + h1, &mono);
    let stereo = pt_header(
        0,
        &[tag(0x80, 2), tag(0x82, 2), tag(0x85, 28), tag(0x86, 3), tag(0x87, 20), tag(0x88, 0x140), tag(0x89, 0x180)],
    );
    put(&mut v, BNK + h2, &stereo);
    put(&mut v, BNK + 0x100, &xa_pcm_frame(0, 0, &ramp(0)));
    put(&mut v, BNK + 0x140, &xa_pcm_frame(0, 0, &ramp(100)));
    put(&mut v, BNK + 0x180, &xa_pcm_frame(0, 0, &ramp(-100)));
    v
}

#[test]
fn the_sound_list_skips_dummy_entries() {
    let data = bank();
    let bank = Bank::parse(&data).unwrap();
    let indexes: Vec<usize> = bank.sounds().iter().map(|s| s.index).collect();
    assert_eq!(indexes, [1, 2]);
    assert_eq!(bank.sound(2).unwrap().header.channels, 2);
    assert!(bank.sound(0).is_none());
}

#[test]
fn modules_players_and_tables_are_linked_without_duplicates() {
    let data = bank();
    let bank = Bank::parse(&data).unwrap();
    assert_eq!(bank.modules().len(), 1);
    let players: Vec<usize> = bank.modules()[0].players.iter().map(|p| p.sample_table).collect();
    assert_eq!(players, [0, 0, 1], "two players share one table; the class controller has no player slot");
    let tables = bank.sample_tables();
    assert_eq!(tables.len(), 2);
    assert_eq!(tables[0].entries.len(), 3);
    assert!(tables[0].entries[0].is_dummy());
    assert_eq!((tables[0].entries[1].index, tables[0].entries[1].priority), (1, 7));
    let streamed = tables[1].entries[0];
    assert_eq!(streamed.kind, SoundKind::Streamed);
    assert_eq!((streamed.index, streamed.loop_offset, streamed.priority), (0x1234, 0x5678, 3));
    assert!(!streamed.is_dummy());
}

#[test]
fn sounds_decode_per_channel_from_the_bnk_relative_offsets() {
    let data = bank();
    let bank = Bank::parse(&data).unwrap();
    let mono = bank.decode(1).unwrap();
    assert_eq!((mono.sample_rate, mono.channels, mono.loop_range), (44100, 1, None));
    assert_eq!(mono.samples, ramp(0));
    let stereo = bank.decode(2).unwrap();
    assert_eq!((stereo.sample_rate, stereo.channels), (22050, 2), "no rate tag: the PC default");
    assert_eq!(stereo.loop_range, Some((3, 21)));
    assert_eq!(stereo.samples[0], 100);
    assert_eq!(stereo.samples[1], -100);
    assert_eq!(stereo.samples[2], 110);
}

#[test]
fn unknown_and_dummy_indexes_are_errors() {
    let data = bank();
    let bank = Bank::parse(&data).unwrap();
    assert_eq!(bank.decode(0).unwrap_err(), Error::NoSuchEntry(0));
    assert_eq!(bank.decode(9).unwrap_err(), Error::NoSuchEntry(9));
}

#[test]
fn damaged_banks_are_errors_not_panics() {
    let data = bank();
    assert!(matches!(Bank::parse(&data[..3]), Err(Error::BadMagic { .. })));
    assert!(matches!(Bank::parse(b"XXXX"), Err(Error::BadMagic { .. })));
    assert!(Bank::parse(&data[..0x10]).is_err());
    assert!(Bank::parse(&data[..0x250]).is_err(), "BNKl cut off");
    let mut bad_version = data.clone();
    bad_version[BNK + 4] = 9;
    assert!(matches!(Bank::parse(&bad_version), Err(Error::Unsupported(_))));
    let mut bad_table = data.clone();
    put32(&mut bad_table, 0x200, 0xFFFF_FFF0);
    assert!(matches!(Bank::parse(&bad_table), Err(Error::Corrupt(_))));
    let mut huge = data.clone();
    let header = pt_header(0, &[tag(0x80, 2), tag(0x85, 0x7FFF_FFFF), tag(0x88, 0x100)]);
    put(&mut huge, BNK + 0x40, &header);
    assert!(matches!(Bank::parse(&huge).unwrap().decode(1), Err(Error::Corrupt(_))));
    let mut no_offset = data.clone();
    let header = pt_header(0, &[tag(0x80, 2), tag(0x85, 28)]);
    put(&mut no_offset, BNK + 0x40, &header);
    assert!(matches!(Bank::parse(&no_offset).unwrap().decode(1), Err(Error::BadHeader(_))));
}

#[test]
fn a_bank_without_a_bnk_has_no_sounds() {
    let mut data = bank();
    put32(&mut data, 0x20, 0);
    let bank = Bank::parse(&data).unwrap();
    assert!(bank.sounds().is_empty());
    assert_eq!(bank.sample_tables().len(), 2);
}

#[test]
fn other_codecs_in_a_bank_are_reported() {
    let mut data = bank();
    let header = pt_header(0, &[tag(0x80, 2), tag(0x85, 28), tag(0x88, 0x100), tag(0xA0, 4)]);
    put(&mut data, BNK + 0x40, &header);
    assert!(matches!(Bank::parse(&data).unwrap().decode(1), Err(Error::Unsupported(_))));
    let header = pt_header(0, &[tag(0x80, 2), tag(0x85, 28), tag(0x88, 0x100), tag(0xA0, 0x12)]);
    put(&mut data, BNK + 0x40, &header);
    assert_eq!(Bank::parse(&data).unwrap().decode(1).unwrap_err(), Error::UnsupportedCodec(0x12));
}
