//! Garbage in, never a panic: random bytes, random bodies behind a valid header, and corrupted real snapshots all go into
//! the client reader.

use super::snapshot::{all_candidates, fill, round_schema, snapshot, step};
use crate::bits::{BitReader, BitWriter};
use crate::replicate::{ClientState, ClientViews, EncodeCache, History};
use crate::rng::SplitMix64;

/// Cases run through the reader: 20,000, in three kinds.
const CASES: u32 = 20_000;

/// Valid snapshots of a short chain (each based on the one before), to corrupt.
fn valid_chain(seed: u64) -> Vec<Vec<u8>> {
    let schema = round_schema();
    let mut rng = SplitMix64(seed);
    let mut entities = std::collections::BTreeMap::new();
    let mut history = History::new(64);
    let mut cache = EncodeCache::new();
    let mut server = ClientState::new();
    let mut chain = Vec::new();
    for tick in 0..6u32 {
        step(&mut rng, &mut entities, 40);
        fill(&schema, &mut history, tick, &entities);
        cache.reset(tick);
        let mut buf = vec![0u8; 2048];
        let (_, bytes) =
            snapshot(&schema, &history, &mut cache, &mut server, tick, &all_candidates(&entities), 2048 * 8, &mut buf);
        buf.truncate(bytes);
        chain.push(buf);
        server.on_delivered(tick);
    }
    chain
}

/// A buffer of random bytes, with the header of a snapshot of `tick` naming no baseline and `count` entries written over
/// its start, so that the entry decoder runs on random bits.
fn header_over_noise(rng: &mut SplitMix64, tick: u32, count: u64) -> Vec<u8> {
    let len = 8 + rng.below(56) as usize;
    let mut buf: Vec<u8> = (0..len).map(|_| rng.next_u64() as u8).collect();
    let mut w = BitWriter::new(&mut buf);
    assert!(w.write_bits(u64::from(tick), 32).is_ok());
    assert!(w.write_varint(0).is_ok());
    assert!(w.write_varint(count).is_ok());
    buf
}

#[test]
fn garbage_snapshots_never_panic() {
    let schema = round_schema();
    let mut rng = SplitMix64(0x6A2B_A6E0);
    let chain = valid_chain(0xC0FFEE);
    let mut client = ClientViews::new();
    let mut errors = 0u32;
    let mut decoded = 0u32;
    for case in 0..CASES {
        let buf: Vec<u8> = match case % 3 {
            0 => {
                let len = rng.below(64) as usize;
                (0..len).map(|_| rng.next_u64() as u8).collect()
            }
            1 => {
                let tick = rng.next_u64() as u32;
                let count = rng.below(48);
                header_over_noise(&mut rng, tick, count)
            }
            _ => {
                let mut copy = chain[rng.below(chain.len() as u64) as usize].clone();
                for _ in 0..1 + rng.below(3) {
                    let bit = rng.below(copy.len() as u64 * 8) as usize;
                    copy[bit / 8] ^= 1 << (bit % 8);
                }
                if rng.below(4) == 0 {
                    let keep = rng.below(copy.len() as u64) as usize;
                    copy.truncate(keep);
                }
                copy
            }
        };
        // Some of the garbage decodes, and a decoded view can then be the baseline of a later case: that is fine.
        let mut r = BitReader::new(&buf);
        match client.read_snapshot(&schema, &mut r) {
            Ok(_) => decoded += 1,
            Err(_) => errors += 1,
        }
        if case % 5_000 == 4_999 {
            client.reset();
        }
    }
    assert!(errors > CASES / 2, "most garbage must be refused: {errors} errors");
    assert!(decoded > 0, "some corrupted snapshots still decode");
}

#[test]
fn an_intact_chain_decodes_after_the_garbage() {
    let schema = round_schema();
    let chain = valid_chain(0xC0FFEE);
    let mut client = ClientViews::new();
    for (tick, bytes) in chain.iter().enumerate() {
        let mut r = BitReader::new(bytes);
        assert_eq!(client.read_snapshot(&schema, &mut r), Ok(tick as u32));
    }
}
