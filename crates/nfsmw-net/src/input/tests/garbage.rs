//! Hostile input payloads: decoding never panics, refuses each malformed shape, and accepts only canonical bytes.

use blackbox_net_core::bits::BitWriter;
use blackbox_net_core::rng::SplitMix64;

use super::code;
use crate::input::*;

/// A payload written field by field: `(value, bits)` pairs, then zero padding.
fn handmade(fields: &[(u64, u32)]) -> Vec<u8> {
    let mut buf = [0u8; 64];
    let mut w = BitWriter::new(&mut buf);
    for (value, bits) in fields {
        w.write_bits(*value, *bits).unwrap();
    }
    w.align().unwrap();
    let len = w.bytes_written();
    buf[..len].to_vec()
}

/// One full input without gear or clutch: same 0, throttle, brake, steer, handbrake, nos, up, down.
fn full(steer: u64, up: u64, down: u64) -> Vec<(u64, u32)> {
    vec![(0, 1), (5, 7), (0, 7), (steer, 9), (0, 4), (0, 1), (up, 1), (down, 1)]
}

fn frame_of(count: u64, flags: u64, inputs: &[Vec<(u64, u32)>]) -> Vec<u8> {
    let mut fields = vec![(7, 16), (count, 4), (flags, 4)];
    for input in inputs {
        fields.extend_from_slice(input);
    }
    handmade(&fields)
}

#[test]
fn malformed_shapes_have_their_errors() {
    assert!(InputFrame::decode(&frame_of(1, 0, &[full(255, 0, 0)])).is_ok());
    assert_eq!(InputFrame::decode(&frame_of(0, 0, &[])), Err(InputError::BadCount));
    assert_eq!(InputFrame::decode(&frame_of(9, 0, &[full(255, 0, 0)])), Err(InputError::BadCount));
    assert_eq!(InputFrame::decode(&frame_of(1, 0b0100, &[full(255, 0, 0)])), Err(InputError::Reserved));
    assert_eq!(InputFrame::decode(&frame_of(1, 0, &[full(511, 0, 0)])), Err(InputError::BadValue));
    assert_eq!(InputFrame::decode(&frame_of(1, 0, &[full(255, 1, 1)])), Err(InputError::BadValue));
    assert_eq!(InputFrame::decode(&frame_of(1, 0, &[vec![(1, 1)]])), Err(InputError::SameOnFirst));
    let twice = [full(255, 0, 0), full(255, 0, 0)];
    assert_eq!(InputFrame::decode(&frame_of(2, 0, &twice)), Err(InputError::NotCanonical));
    let mut no_gear = full(255, 0, 0);
    no_gear.push((0, 1));
    assert_eq!(InputFrame::decode(&frame_of(1, 0b0010, &[no_gear])), Err(InputError::NotCanonical));
    assert_eq!(InputFrame::decode(&frame_of(2, 0, &[full(255, 0, 0)])), Err(InputError::Truncated));
}

#[test]
fn padding_and_trailing_bytes_are_refused() {
    let good = frame_of(1, 0, &[full(255, 0, 0)]);
    let mut padded = good.clone();
    *padded.last_mut().unwrap() |= 1;
    assert_eq!(InputFrame::decode(&padded), Err(InputError::BadPadding));
    let mut trailing = good;
    trailing.push(0);
    assert_eq!(InputFrame::decode(&trailing), Err(InputError::TrailingBytes));
    assert_eq!(InputFrame::decode(&[0; MAX_INPUT_PAYLOAD + 1]), Err(InputError::TrailingBytes));
    assert_eq!(InputFrame::decode(&[]), Err(InputError::Truncated));
}

#[test]
fn seeded_garbage_never_panics_and_accepts_only_canonical_bytes() {
    let mut rng = SplitMix64(0x1A9E_0001);
    let mut accepted = 0;
    for _ in 0..50_000 {
        let len = rng.below(MAX_INPUT_PAYLOAD as u64 + 4) as usize;
        let mut bytes: Vec<u8> = (0..len).map(|_| rng.next_u64() as u8).collect();
        // Make a valid count likely so the decoder gets past the header.
        if bytes.len() > 2 {
            bytes[2] = (bytes[2] & 0x0F) | ((1 + rng.below(8) as u8) << 4);
        }
        let frame = match InputFrame::decode(&bytes) {
            Ok(frame) => frame,
            Err(_) => continue,
        };
        accepted += 1;
        let mut out = [0u8; MAX_INPUT_PAYLOAD];
        let n = frame.encode(&mut out).unwrap();
        assert_eq!(&out[..n], &bytes[..]);
        assert!(frame.inputs().iter().all(InputCode::is_valid));
    }
    assert!(accepted > 0);
}

#[test]
fn mutated_frames_never_panic() {
    let mut rng = SplitMix64(0x1A9E_0002);
    let mut frame = InputFrame::new(77, true);
    for i in 0..8u8 {
        frame.push(InputCode { gear_select: Some(i % 3), clutch: i, ..code(i * 9, 200 + u16::from(i)) }).unwrap();
    }
    let mut base = [0u8; MAX_INPUT_PAYLOAD];
    let n = frame.encode(&mut base).unwrap();
    for _ in 0..20_000 {
        let mut bytes = base[..n].to_vec();
        let at = rng.below(n as u64) as usize;
        bytes[at] ^= 1 << rng.below(8);
        if let Ok(decoded) = InputFrame::decode(&bytes) {
            let mut out = [0u8; MAX_INPUT_PAYLOAD];
            let len = decoded.encode(&mut out).unwrap();
            assert_eq!(&out[..len], &bytes[..]);
        }
    }
}
