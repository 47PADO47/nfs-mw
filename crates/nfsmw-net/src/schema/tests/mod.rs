mod bounds;
mod own;

use blackbox_net_core::bits::{BitReader, BitWriter};
use blackbox_net_core::replicate::{Frame, decode_delta, encode_delta};
use nfsmw_sim::{EntityKind, EntityView, PlayerId};

use super::*;

pub fn view(kind: EntityKind) -> EntityView {
    let yaw: f32 = 0.7;
    EntityView {
        id: 12,
        kind,
        owner: match kind {
            EntityKind::PlayerCar => Some(PlayerId(3)),
            _ => None,
        },
        position: [1234.567, -987.654, 12.345],
        orientation: quat_of_yaw(yaw),
        linear_velocity: [20.0 * yaw.cos(), 20.0 * yaw.sin(), 0.0],
        angular_velocity: [0.0, 0.0, 0.5],
        type_id: 0x1234,
        flags: 0b1010_0101,
    }
}

#[test]
fn kinds_register_in_wire_order() {
    let schema = GameSchema::new();
    assert_eq!(schema.schema().kind_count(), 7);
    for kind in WireKind::ALL {
        assert_eq!(WireKind::from_kind_id(kind.kind_id()), Some(kind));
        assert_eq!(kind.kind_id().0 + 1, kind as u8);
        assert_eq!(schema.schema().fields(kind.kind_id()).unwrap(), kind.fields());
    }
    assert_eq!(WireKind::from_kind_id(blackbox_net_core::replicate::KindId(7)), None);
}

#[test]
fn full_records_are_near_the_bandwidth_figures() {
    let schema = GameSchema::new();
    let bits = |kind: WireKind| schema.schema().full_bits(kind.kind_id()).unwrap();
    // bandwidth.md 1.1: 250 bits plus the flags, type and owner this layout adds.
    assert_eq!(bits(WireKind::PlayerCar), 286);
    assert!(bits(WireKind::TrafficCar) <= 120);
    assert!(bits(WireKind::PlayerCarReduced) <= 100);
}

#[test]
fn schema_id_follows_the_bounds() {
    let a = GameSchema::new();
    let b = GameSchema::with_half_extent(GameSchema::DEFAULT_HALF_EXTENT).unwrap();
    let c = GameSchema::with_half_extent(4096.0).unwrap();
    assert_eq!(a.schema_id(), b.schema_id());
    assert_ne!(a.schema_id(), c.schema_id());
    assert_eq!(GameSchema::with_half_extent(0.0).unwrap_err(), SchemaError::BadBounds);
    assert_eq!(GameSchema::with_half_extent(f32::NAN).unwrap_err(), SchemaError::BadBounds);
}

#[test]
fn every_kind_round_trips_through_a_frame() {
    let schema = GameSchema::new();
    let mut frame = Frame::new();
    frame.clear(1);
    let kinds = [
        EntityKind::PlayerCar,
        EntityKind::TrafficCar,
        EntityKind::PoliceCar,
        EntityKind::Helicopter,
        EntityKind::Roadblock,
        EntityKind::MovingProp,
    ];
    let mut record = Record::default();
    for (index, kind) in kinds.iter().enumerate() {
        let mut v = view(*kind);
        v.id = index as u16 + 1;
        schema.encode(&v, None, &mut record).unwrap();
        frame.push(schema.schema(), v.id, record.kind(), record.values()).unwrap();
    }
    for (id, kind, values) in frame.iter() {
        let decoded = schema.decode(id, kind, values).unwrap();
        let original = view(decoded.view.kind);
        assert_eq!(decoded.view.id, id);
        assert_eq!(decoded.view.type_id, original.type_id);
        assert_eq!(decoded.view.flags, original.flags);
        assert_eq!(decoded.view.owner, original.owner);
        assert_eq!(decoded.tier, Tier::Full);
        for axis in 0..3 {
            assert!((decoded.view.position[axis] - original.position[axis]).abs() <= schema.position_step());
        }
        assert!((yaw_of(decoded.view.orientation) - 0.7).abs() < 0.03, "{kind:?}");
    }
}

#[test]
fn a_player_car_keeps_its_detail() {
    let schema = GameSchema::new();
    let detail = CarDetail {
        wheel_compression: [0.1, 0.5, 0.9, 1.0],
        wheel_on_ground: [true, false, true, true],
        wheel_steer: -0.4,
        skid: 0b1001_0110,
        rpm: 6543.0,
        gear: 4,
        throttle: 0.8,
        brake: 0.0,
        steer: 0.25,
    };
    let mut record = Record::default();
    schema.encode(&view(EntityKind::PlayerCar), Some(&detail), &mut record).unwrap();
    let decoded = schema.decode(12, record.kind(), record.values()).unwrap();
    let got = decoded.car.unwrap();
    assert_eq!(got.wheel_on_ground, detail.wheel_on_ground);
    assert_eq!(got.skid, detail.skid);
    assert_eq!(got.gear, 4);
    assert!((got.rpm - detail.rpm).abs() <= 5.0);
    assert!((got.throttle - detail.throttle).abs() <= 1.0 / 63.0);
    assert!((got.steer - detail.steer).abs() <= 1.0 / 255.0);
    for (a, b) in got.wheel_compression.iter().zip(detail.wheel_compression) {
        assert!((a - b).abs() <= 0.5 / 63.0 + 1e-6);
    }
}

#[test]
fn reduced_tier_keeps_position_heading_and_owner() {
    let schema = GameSchema::new();
    let mut record = Record::default();
    let car = view(EntityKind::PlayerCar);
    schema.encode_reduced(&car, &mut record).unwrap();
    assert_eq!(record.kind(), WireKind::PlayerCarReduced.kind_id());
    let decoded = schema.decode(12, record.kind(), record.values()).unwrap();
    assert_eq!(decoded.tier, Tier::Reduced);
    assert_eq!(decoded.view.kind, EntityKind::PlayerCar);
    assert_eq!(decoded.view.owner, Some(PlayerId(3)));
    let speed = decoded.view.linear_velocity[0].hypot(decoded.view.linear_velocity[1]);
    assert!((speed - 20.0).abs() <= 0.25);
}

#[test]
fn flag_bits_beyond_the_width_are_dropped() {
    let schema = GameSchema::new();
    let mut traffic = view(EntityKind::TrafficCar);
    traffic.flags = 0xFFFF_FF0F;
    let mut record = Record::default();
    schema.encode(&traffic, None, &mut record).unwrap();
    let decoded = schema.decode(1, record.kind(), record.values()).unwrap();
    assert_eq!(decoded.view.flags, 0x0F);
}

#[test]
fn bad_views_are_refused() {
    let schema = GameSchema::new();
    let mut record = Record::default();
    let mut far = view(EntityKind::PlayerCar);
    far.position[0] = 9000.0;
    assert_eq!(schema.encode(&far, None, &mut record), Err(SchemaError::OutOfBounds));
    let mut nan = view(EntityKind::TrafficCar);
    nan.linear_velocity[1] = f32::NAN;
    assert_eq!(schema.encode(&nan, None, &mut record), Err(SchemaError::NotFinite));
    let mut zero = view(EntityKind::MovingProp);
    zero.orientation = [0.0; 4];
    assert_eq!(schema.encode(&zero, None, &mut record), Err(SchemaError::BadQuaternion));
    let mut owner = view(EntityKind::PlayerCar);
    owner.owner = Some(PlayerId(63));
    assert_eq!(schema.encode(&owner, None, &mut record), Err(SchemaError::BadOwner));
}

#[test]
fn bad_records_are_refused() {
    let schema = GameSchema::new();
    let kind = WireKind::TrafficCar.kind_id();
    assert_eq!(schema.decode(1, kind, &[0; 3]), Err(SchemaError::BadRecord));
    assert_eq!(schema.decode(1, blackbox_net_core::replicate::KindId(40), &[0; 7]), Err(SchemaError::BadRecord));
}

#[test]
fn a_moving_car_delta_is_small_and_exact() {
    let schema = GameSchema::new();
    let mut car = view(EntityKind::PlayerCar);
    let mut before = Record::default();
    schema.encode(&car, None, &mut before).unwrap();
    // One 20 Hz snapshot at 20 m/s: 1 m forward.
    car.position[0] += 1.0;
    let mut after = Record::default();
    schema.encode(&car, None, &mut after).unwrap();
    let mut buf = [0u8; 64];
    let mut w = BitWriter::new(&mut buf);
    assert!(encode_delta(schema.schema(), after.kind(), before.values(), after.values(), &mut w).unwrap());
    let bits = w.bits_written();
    assert!(bits <= 20 + 24, "{bits} bits");
    let mut out = [0u32; blackbox_net_core::replicate::MAX_FIELDS];
    let count =
        decode_delta(schema.schema(), after.kind(), before.values(), &mut BitReader::new(&buf), &mut out).unwrap();
    assert_eq!(&out[..count], after.values());
}
