use blackbox_net_core::bits::{BitReader, BitWriter};
use nfsmw_sim::{EntityKind, OwnState};

use super::super::*;

fn block() -> OwnBlock {
    let own = OwnState { rpm: 7_250.0, gear: 5, nitrous: 0.6, boost: 0.25, flags: 0b0100_0001 };
    let report = InputReport { early: 3, on_time: 25, late: 2, missing: 40, earliness: 2 };
    OwnBlock::new(&super::view(EntityKind::PlayerCar), own, CorrectionCause::Contact, report)
}

#[test]
fn own_block_round_trips_within_its_steps() {
    let schema = GameSchema::new();
    let original = block();
    let mut buf = [0u8; OWN_BLOCK_BITS / 8];
    let mut w = BitWriter::new(&mut buf);
    schema.write_own(&original, &mut w).unwrap();
    assert_eq!(w.bits_written(), OWN_BLOCK_BITS);
    let back = schema.read_own(&mut BitReader::new(&buf)).unwrap();
    for axis in 0..3 {
        assert!((back.position[axis] - original.position[axis]).abs() <= schema.position_step());
        assert!((back.linear_velocity[axis] - original.linear_velocity[axis]).abs() <= LINEAR_VELOCITY.step());
        assert!((back.angular_velocity[axis] - original.angular_velocity[axis]).abs() <= ANGULAR_VELOCITY.step());
    }
    assert!((back.own.rpm - 7_250.0).abs() <= 5.0);
    assert_eq!(back.own.gear, 5);
    assert!((back.own.nitrous - 0.6).abs() <= NITROUS.step() * 0.5 + 1e-6);
    assert!((back.own.boost - 0.25).abs() <= BOOST.step() * 0.5 + 1e-6);
    assert_eq!(back.own.flags, original.own.flags);
    assert_eq!(back.cause, CorrectionCause::Contact);
    assert_eq!(back.report, InputReport { missing: 31, ..original.report });
}

#[test]
fn own_block_refuses_reserved_bits_and_bad_causes() {
    let schema = GameSchema::new();
    let mut buf = [0u8; OWN_BLOCK_BITS / 8];
    schema.write_own(&block(), &mut BitWriter::new(&mut buf)).unwrap();
    let mut reserved = buf;
    reserved[31] |= 1;
    assert_eq!(schema.read_own(&mut BitReader::new(&reserved)), Err(SchemaError::Bits));
    // The cause takes bits 199 to 201; 7 is not a cause.
    let mut cause = buf;
    for bit in 199..202 {
        cause[bit / 8] |= 0x80 >> (bit % 8);
    }
    assert_eq!(schema.read_own(&mut BitReader::new(&cause)), Err(SchemaError::Bits));
    assert_eq!(schema.read_own(&mut BitReader::new(&buf[..31])), Err(SchemaError::Bits));
}

#[test]
fn own_block_needs_room_and_finite_values() {
    let schema = GameSchema::new();
    let mut small = [0u8; 31];
    assert_eq!(schema.write_own(&block(), &mut BitWriter::new(&mut small)), Err(SchemaError::Bits));
    let mut nan = block();
    nan.own.rpm = f32::NAN;
    let mut buf = [0u8; 32];
    assert_eq!(schema.write_own(&nan, &mut BitWriter::new(&mut buf)), Err(SchemaError::NotFinite));
}
