use super::{random_bits, random_width};
use crate::bits::{BitError, BitReader, BitWriter, IdGapDecoder, IdGapEncoder};
use crate::rng::SplitMix64;

const GROUP_SIZES: [u32; 5] = [1, 3, 7, 13, 63];

#[test]
fn varint_layout_is_lowest_group_first() {
    let mut buf = [0u8; 4];
    let mut w = BitWriter::new(&mut buf);
    w.write_varint(0).unwrap();
    w.write_varint(127).unwrap();
    w.write_varint(128).unwrap();
    assert_eq!(w.bits_written(), 32);
    assert_eq!(buf, [0x00, 0x7F, 0x80, 0x01]);
}

#[test]
fn varint_round_trip_random_cases() {
    let mut rng = SplitMix64(11);
    let cases: Vec<(u64, u32)> = (0..10_000)
        .map(|i| {
            let width = random_width(&mut rng);
            (random_bits(&mut rng, width), GROUP_SIZES[i % GROUP_SIZES.len()])
        })
        .collect();
    let mut buf = vec![0u8; 1 << 20];
    {
        let mut w = BitWriter::new(&mut buf);
        for (value, group) in &cases {
            w.write_varint_n(*value, *group).unwrap();
        }
    }
    let mut r = BitReader::new(&buf);
    for (value, group) in &cases {
        assert_eq!(r.read_varint_n(*group).unwrap(), *value);
    }
}

#[test]
fn varint_extremes_for_every_group_size() {
    for group in GROUP_SIZES {
        for value in [0, 1, u64::MAX / 2, u64::MAX] {
            let mut buf = [0u8; 32];
            let mut w = BitWriter::new(&mut buf);
            w.write_varint_n(value, group).unwrap();
            let used = w.bits_written();
            let mut r = BitReader::new(&buf);
            assert_eq!(r.read_varint_n(group).unwrap(), value);
            assert_eq!(r.bits_read(), used);
        }
    }
}

#[test]
fn varint_group_size_out_of_range_is_bad_value() {
    let mut buf = [0u8; 4];
    let mut w = BitWriter::new(&mut buf);
    assert_eq!(w.write_varint_n(1, 0), Err(BitError::BadValue));
    assert_eq!(w.write_varint_n(1, 64), Err(BitError::BadValue));
    assert_eq!(w.bits_written(), 0);
    let mut r = BitReader::new(&buf);
    assert_eq!(r.read_varint_n(0), Err(BitError::BadValue));
    assert_eq!(r.read_varint_n(64), Err(BitError::BadValue));
}

#[test]
fn non_canonical_varints_are_rejected() {
    // A trailing zero group: value 0 written as two groups.
    let buf = [0x80u8, 0x00];
    assert_eq!(BitReader::new(&buf).read_varint(), Err(BitError::BadVarint));

    // Ten groups with the continuation bit set: more than ceil(64 / 7) groups.
    let mut buf = [0u8; 16];
    {
        let mut w = BitWriter::new(&mut buf);
        for _ in 0..10 {
            w.write_bits(0b1000_0001, 8).unwrap();
        }
    }
    assert_eq!(BitReader::new(&buf).read_varint(), Err(BitError::BadVarint));

    // Nine continuation groups, then a last group carrying 2: that is bit 64.
    let mut buf = [0u8; 16];
    {
        let mut w = BitWriter::new(&mut buf);
        for _ in 0..9 {
            w.write_bits(0b1000_0000, 8).unwrap();
        }
        w.write_bits(0b0000_0010, 8).unwrap();
    }
    assert_eq!(BitReader::new(&buf).read_varint(), Err(BitError::BadVarint));
}

#[test]
fn tenth_group_carrying_bit_63_is_accepted() {
    let mut buf = [0x80u8; 10];
    buf[9] = 0x01;
    assert_eq!(BitReader::new(&buf).read_varint(), Ok(1u64 << 63));
}

#[test]
fn truncated_varint_is_underflow() {
    let buf = [0x81u8];
    assert_eq!(BitReader::new(&buf).read_varint(), Err(BitError::Underflow));
}

#[test]
fn zigzag_layout_and_extremes() {
    let mut buf = [0u8; 128];
    let mut w = BitWriter::new(&mut buf);
    w.write_zigzag(0).unwrap();
    w.write_zigzag(-1).unwrap();
    w.write_zigzag(1).unwrap();
    assert_eq!(buf[..3], [0, 1, 2]);

    let values = [i64::MIN, i64::MAX, -1, 0, 1, i64::MIN + 1];
    let mut buf = [0u8; 128];
    {
        let mut w = BitWriter::new(&mut buf);
        for v in values {
            w.write_zigzag(v).unwrap();
        }
    }
    let mut r = BitReader::new(&buf);
    for v in values {
        assert_eq!(r.read_zigzag().unwrap(), v);
    }
}

#[test]
fn zigzag_round_trip_random_cases() {
    let mut rng = SplitMix64(13);
    let values: Vec<i64> = (0..10_000).map(|_| rng.next_u64() as i64).collect();
    let mut buf = vec![0u8; 1 << 20];
    {
        let mut w = BitWriter::new(&mut buf);
        for v in &values {
            w.write_zigzag(*v).unwrap();
        }
    }
    let mut r = BitReader::new(&buf);
    for v in &values {
        assert_eq!(r.read_zigzag().unwrap(), *v);
    }
}

#[test]
fn id_gaps_round_trip_random_sets() {
    let mut rng = SplitMix64(23);
    for _ in 0..10_000 {
        let count = rng.below(40) as usize;
        let mut ids: Vec<u16> = (0..count).map(|_| rng.next_u64() as u16).collect();
        ids.push(0);
        ids.push(u16::MAX);
        ids.sort_unstable();
        ids.dedup();
        let mut buf = [0u8; 256];
        let used = {
            let mut w = BitWriter::new(&mut buf);
            let mut enc = IdGapEncoder::new();
            for id in &ids {
                enc.write(&mut w, *id).unwrap();
            }
            w.bits_written()
        };
        let mut r = BitReader::new(&buf);
        let mut dec = IdGapDecoder::new();
        for id in &ids {
            assert_eq!(dec.read(&mut r).unwrap(), *id);
        }
        assert_eq!(r.bits_read(), used);
    }
}

#[test]
fn consecutive_ids_cost_four_bits() {
    let mut buf = [0u8; 4];
    let mut w = BitWriter::new(&mut buf);
    let mut enc = IdGapEncoder::new();
    enc.write(&mut w, 5).unwrap();
    assert_eq!(w.bits_written(), 8);
    enc.write(&mut w, 6).unwrap();
    assert_eq!(w.bits_written(), 12);
}

#[test]
fn non_increasing_ids_are_bad_value_and_write_nothing() {
    let mut buf = [0u8; 4];
    let mut w = BitWriter::new(&mut buf);
    let mut enc = IdGapEncoder::new();
    enc.write(&mut w, 5).unwrap();
    assert_eq!(enc.write(&mut w, 5), Err(BitError::BadValue));
    assert_eq!(enc.write(&mut w, 3), Err(BitError::BadValue));
    assert_eq!(w.bits_written(), 8);
    enc.write(&mut w, 6).unwrap();
    assert_eq!(w.bits_written(), 12);
}

#[test]
fn id_decoder_rejects_values_past_u16() {
    let mut buf = [0u8; 8];
    {
        let mut w = BitWriter::new(&mut buf);
        w.write_varint_n(70_000, 7).unwrap();
    }
    let mut r = BitReader::new(&buf);
    assert_eq!(IdGapDecoder::new().read(&mut r), Err(BitError::BadValue));

    // Previous id 65535, then a gap of 0: the sum is 65536.
    let mut buf = [0u8; 8];
    {
        let mut w = BitWriter::new(&mut buf);
        w.write_varint_n(65_535, 7).unwrap();
        w.write_varint_n(0, 3).unwrap();
    }
    let mut r = BitReader::new(&buf);
    let mut dec = IdGapDecoder::new();
    assert_eq!(dec.read(&mut r), Ok(65_535));
    assert_eq!(dec.read(&mut r), Err(BitError::BadValue));
}
