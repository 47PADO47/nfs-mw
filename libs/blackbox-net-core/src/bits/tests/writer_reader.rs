use super::{mask, random_bits, random_width};
use crate::bits::{BitError, BitReader, BitWriter};
use crate::rng::SplitMix64;

/// One operation of the mixed stream test.
#[derive(Debug, Clone, Copy)]
enum Op {
    Bits(u64, u32),
    Bool(bool),
    Varint(u64),
    VarintN(u64, u32),
    Zigzag(i64),
    Align,
}

fn random_op(rng: &mut SplitMix64) -> Op {
    let kind = rng.below(6);
    let width = random_width(rng);
    let value = random_bits(rng, width);
    match kind {
        0 => Op::Bits(value, width),
        1 => Op::Bool((value & 1) == 1),
        2 => Op::Varint(value),
        3 => Op::VarintN(value, [1, 2, 3, 7, 13, 31, 63][(width % 7) as usize]),
        4 => Op::Zigzag(value as i64),
        _ => Op::Align,
    }
}

fn write_op(w: &mut BitWriter<'_>, op: Op) -> Result<(), BitError> {
    match op {
        Op::Bits(v, bits) => w.write_bits(v, bits),
        Op::Bool(v) => w.write_bool(v),
        Op::Varint(v) => w.write_varint(v),
        Op::VarintN(v, n) => w.write_varint_n(v, n),
        Op::Zigzag(v) => w.write_zigzag(v),
        Op::Align => w.align(),
    }
}

fn check_op(r: &mut BitReader<'_>, op: Op) {
    match op {
        Op::Bits(v, bits) => assert_eq!(r.read_bits(bits).unwrap(), v),
        Op::Bool(v) => assert_eq!(r.read_bool().unwrap(), v),
        Op::Varint(v) => assert_eq!(r.read_varint().unwrap(), v),
        Op::VarintN(v, n) => assert_eq!(r.read_varint_n(n).unwrap(), v),
        Op::Zigzag(v) => assert_eq!(r.read_zigzag().unwrap(), v),
        Op::Align => r.align().unwrap(),
    }
}

#[test]
fn random_widths_round_trip() {
    let mut rng = SplitMix64(1);
    let ops: Vec<(u64, u32)> = (0..10_000)
        .map(|_| {
            let bits = random_width(&mut rng);
            (random_bits(&mut rng, bits), bits)
        })
        .collect();
    let total: usize = ops.iter().map(|(_, bits)| *bits as usize).sum();
    // Prefilled with garbage: the writer must clear each byte as it reaches it.
    let mut buf = vec![0xAAu8; total.div_ceil(8) + 1];
    {
        let mut w = BitWriter::new(&mut buf);
        for (value, bits) in &ops {
            w.write_bits(*value, *bits).unwrap();
        }
        assert_eq!(w.bits_written(), total);
        assert_eq!(w.bytes_written(), total.div_ceil(8));
    }
    let mut r = BitReader::new(&buf);
    for (value, bits) in &ops {
        assert_eq!(r.read_bits(*bits).unwrap(), *value);
    }
    assert_eq!(r.bits_read(), total);
}

#[test]
fn mixed_stream_round_trip() {
    let mut rng = SplitMix64(7);
    let ops: Vec<Op> = (0..10_000).map(|_| random_op(&mut rng)).collect();
    let mut buf = vec![0xFFu8; 1 << 20];
    let written = {
        let mut w = BitWriter::new(&mut buf);
        for op in &ops {
            write_op(&mut w, *op).unwrap();
        }
        w.bits_written()
    };
    let mut r = BitReader::new(&buf);
    for op in &ops {
        check_op(&mut r, *op);
    }
    assert_eq!(r.bits_read(), written);
    assert_eq!(r.remaining_bits(), buf.len() * 8 - written);
}

#[test]
fn every_width_at_every_offset() {
    let pattern = 0xA5C3_9E71_2B4D_6F08u64;
    for width in 0..=64u32 {
        for offset in 0..8u32 {
            let value = pattern & mask(width);
            let mut buf = [0xFFu8; 16];
            let mut w = BitWriter::new(&mut buf);
            w.write_bits(mask(offset), offset).unwrap();
            w.write_bits(value, width).unwrap();
            assert_eq!(w.bits_written(), (offset + width) as usize);
            let mut r = BitReader::new(&buf);
            assert_eq!(r.read_bits(offset).unwrap(), mask(offset));
            assert_eq!(r.read_bits(width).unwrap(), value);
        }
    }
}

#[test]
fn values_wider_than_the_field_are_bad_value() {
    let mut buf = [0u8; 8];
    let mut w = BitWriter::new(&mut buf);
    assert_eq!(w.write_bits(0b100, 2), Err(BitError::BadValue));
    assert_eq!(w.write_bits(1, 0), Err(BitError::BadValue));
    assert_eq!(w.write_bits(0, 0), Ok(()));
    assert_eq!(w.write_bits(0, 65), Err(BitError::BadValue));
    assert_eq!(w.write_bits(u64::MAX, 64), Ok(()));
    assert_eq!(w.bits_written(), 64);
}

#[test]
fn overflow_leaves_the_writer_unchanged() {
    let mut buf = [0xFFu8; 2];
    let mut w = BitWriter::new(&mut buf);
    w.write_bits(0x3FF, 10).unwrap();
    assert_eq!(w.write_bits(0x7F, 7), Err(BitError::Overflow));
    assert_eq!(w.write_varint(1000), Err(BitError::Overflow));
    assert_eq!(w.bits_written(), 10);
    assert_eq!(w.remaining_bits(), 6);
    w.write_bits(0x3F, 6).unwrap();
    assert_eq!(w.write_bool(true), Err(BitError::Overflow));
    assert_eq!(buf, [0xFF, 0xFF]);
}

#[test]
fn underflow_leaves_the_reader_unchanged() {
    let buf = [0xF0u8];
    let mut r = BitReader::new(&buf);
    assert_eq!(r.read_bits(4), Ok(0xF));
    assert_eq!(r.read_bits(5), Err(BitError::Underflow));
    assert_eq!(r.bits_read(), 4);
    assert_eq!(r.read_bits(0), Ok(0));
    assert_eq!(r.read_bits(4), Ok(0));
    assert_eq!(r.read_bool(), Err(BitError::Underflow));
    assert_eq!(r.read_bits(65), Err(BitError::BadValue));
    assert_eq!(r.remaining_bits(), 0);
}

#[test]
fn align_pads_with_zeros_and_checks_padding() {
    let mut buf = [0xFFu8; 2];
    let mut w = BitWriter::new(&mut buf);
    w.write_bits(0b101, 3).unwrap();
    w.align().unwrap();
    assert_eq!(w.bits_written(), 8);
    w.align().unwrap();
    assert_eq!(w.bits_written(), 8);
    w.write_bits(0xFF, 8).unwrap();
    assert_eq!(buf, [0xA0, 0xFF]);

    let mut r = BitReader::new(&buf);
    assert_eq!(r.read_bits(3), Ok(0b101));
    assert_eq!(r.align(), Ok(()));
    assert_eq!(r.bits_read(), 8);
    assert_eq!(r.read_bits(8), Ok(0xFF));

    let dirty = [0b1010_0001u8];
    let mut r = BitReader::new(&dirty);
    r.read_bits(3).unwrap();
    assert_eq!(r.align(), Err(BitError::BadPadding));
    assert_eq!(r.bits_read(), 3);
}
