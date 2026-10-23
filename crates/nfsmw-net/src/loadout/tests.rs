use super::*;
use crate::message::{List, Plate, Reader, Wire, Writer};

/// Two cars: 100 (class 1) and 200 (class 2). Parts 1000 + slot fit any car; paints 0 and 5; wheel sets 0 and 9.
/// The rating is 100 times the class plus 10 per part.
struct TestCatalog;

impl Catalog for TestCatalog {
    fn car(&self, car_type: u32) -> Option<CarInfo> {
        match car_type {
            100 => Some(CarInfo { class: 1 }),
            200 => Some(CarInfo { class: 2 }),
            _ => None,
        }
    }
    fn part_fits(&self, _car_type: u32, slot: u16, part: u32) -> bool {
        part == 1000 + u32::from(slot)
    }
    fn paint_known(&self, paint: u32) -> bool {
        matches!(paint, 0 | 5)
    }
    fn wheel_set_known(&self, wheel_set: u32) -> bool {
        matches!(wheel_set, 0 | 9)
    }
    fn performance(&self, loadout: &Loadout) -> Option<u16> {
        let class = self.car(loadout.car_type)?.class;
        Some(u16::from(class) * 100 + 10 * loadout.parts.len() as u16)
    }
}

fn tuned(car_type: u32, slots: &[u16]) -> Loadout {
    let parts = slots.iter().map(|slot| PartChoice { slot: *slot, part: 1000 + u32::from(*slot) }).collect();
    Loadout {
        car_type,
        parts: List::new(parts).unwrap(),
        paint: Paint { id: 5, colours: List::new(vec![1, 2]).unwrap() },
        wheel_set: 9,
        plate: Plate::new("MW 2005").unwrap(),
    }
}

#[test]
fn a_known_loadout_passes() {
    assert_eq!(validate(&tuned(100, &[1, 2, 3]), &TestCatalog, &CarRestriction::Any), Ok(()));
}

#[test]
fn unknown_ids_are_refused_with_their_reason() {
    let any = CarRestriction::Any;
    assert_eq!(validate(&tuned(999, &[]), &TestCatalog, &any), Err(LoadoutReject::UnknownCar));
    let mut bad_part = tuned(100, &[1]);
    bad_part.parts = List::new(vec![PartChoice { slot: 1, part: 7 }]).unwrap();
    assert_eq!(validate(&bad_part, &TestCatalog, &any), Err(LoadoutReject::UnknownPart));
    assert_eq!(validate(&tuned(100, &[4, 4]), &TestCatalog, &any), Err(LoadoutReject::DuplicateSlot));
    let mut bad_paint = tuned(100, &[]);
    bad_paint.paint.id = 6;
    assert_eq!(validate(&bad_paint, &TestCatalog, &any), Err(LoadoutReject::UnknownPaint));
    let mut bad_wheels = tuned(100, &[]);
    bad_wheels.wheel_set = 1;
    assert_eq!(validate(&bad_wheels, &TestCatalog, &any), Err(LoadoutReject::UnknownWheels));
}

#[test]
fn restrictions_by_list_class_and_cap() {
    let list = CarRestriction::Cars(List::new(vec![200]).unwrap());
    assert_eq!(validate(&tuned(100, &[]), &TestCatalog, &list), Err(LoadoutReject::CarNotAllowed));
    assert_eq!(validate(&tuned(200, &[]), &TestCatalog, &list), Ok(()));
    let class = CarRestriction::Class(2);
    assert_eq!(validate(&tuned(100, &[]), &TestCatalog, &class), Err(LoadoutReject::ClassNotAllowed));
    let cap = CarRestriction::PerformanceCap(120);
    assert_eq!(validate(&tuned(100, &[1, 2]), &TestCatalog, &cap), Ok(()));
    assert_eq!(validate(&tuned(100, &[1, 2, 3]), &TestCatalog, &cap), Err(LoadoutReject::OverPerformanceCap));
}

#[test]
fn reject_codes_are_stable() {
    assert_eq!(LoadoutReject::UnknownCar.detail(), 1);
    assert_eq!(LoadoutReject::OverPerformanceCap.detail(), 8);
    assert_eq!(LoadoutReject::from_code(3), Some(LoadoutReject::DuplicateSlot));
    assert_eq!(LoadoutReject::from_code(0), None);
}

#[test]
fn loadout_and_restriction_round_trip() {
    let loadout = tuned(200, &[1, 5, 9]);
    let mut bytes = Vec::new();
    loadout.put(&mut Writer::new(&mut bytes));
    let mut reader = Reader::new(&bytes);
    assert_eq!(Loadout::take(&mut reader).unwrap(), loadout);
    assert!(reader.is_empty());
    for restriction in [
        CarRestriction::Any,
        CarRestriction::Cars(List::new(vec![1, 2, 3]).unwrap()),
        CarRestriction::Class(4),
        CarRestriction::PerformanceCap(650),
    ] {
        let mut bytes = Vec::new();
        restriction.put(&mut Writer::new(&mut bytes));
        assert_eq!(CarRestriction::take(&mut Reader::new(&bytes)).unwrap(), restriction);
    }
    assert!(CarRestriction::take(&mut Reader::new(&[4])).is_err());
}

#[test]
fn too_many_parts_are_refused_on_decode() {
    let mut bytes = Vec::new();
    let mut w = Writer::new(&mut bytes);
    w.varint(100);
    w.varint((MAX_PARTS + 1) as u64);
    for slot in 0..=MAX_PARTS {
        w.varint(slot as u64);
        w.varint(1);
    }
    assert!(Loadout::take(&mut Reader::new(&bytes)).is_err());
}

#[test]
fn to_join_keeps_ids_only() {
    let join = tuned(100, &[3, 1]).to_join(None);
    assert_eq!(join.car_type, 100);
    assert_eq!(join.parts, vec![1003, 1001]);
    assert_eq!(join.spawn, None);
}
