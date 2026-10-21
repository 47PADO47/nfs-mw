//! Bank reader tests on synthetic banks built by `fixture`. No install data is read here.

use super::fixture::{Data, attribute_block, bank, bank_header, child, compound};
use crate::{Bank, Compound, Error, Object};

/// A bank with two sorted animations: `Idle_q` (one DeltaQuat child, 41 frames) and `Walk_t` (one DeltaLerp child,
/// 12 frames, checksum 0x1658). Returns the object and the two record offsets, in that order.
fn two_animation_bank() -> (Object, u32, u32) {
    let mut data = Data::default();
    let idle_child = child(&mut data, 11, 0x1414);
    let idle_block = attribute_block(&mut data, &[(1, 30)]);
    let idle = compound(&mut data, 0x1414, 41, Some(idle_block), &[idle_child]);
    let walk_child = child(&mut data, 10, 0x1658);
    let walk_block = attribute_block(&mut data, &[(1, 30)]);
    let walk = compound(&mut data, 0x1658, 12, Some(walk_block), &[walk_child]);
    let header = bank(&mut data, &[("Idle_q", idle), ("Walk_t", walk)]);
    let object = Object::read(&data.builder(Some(header)).padded()).expect("the synthetic object reads");
    (object, idle, walk)
}

#[test]
fn reads_every_animation_in_name_order() {
    let (object, idle, walk) = two_animation_bank();
    let bank = Bank::read(&object).unwrap();
    assert_eq!(bank.animations.len(), 2);
    let names: Vec<&str> = bank.iter().map(|animation| animation.name.as_str()).collect();
    assert_eq!(names, ["Idle_q", "Walk_t"]);
    let first = &bank.animations[0];
    assert_eq!((first.type_id, first.checksum, first.offset), (15, 0x1414, idle));
    let second = &bank.animations[1];
    assert_eq!((second.type_id, second.checksum, second.offset), (15, 0x1658, walk));
}

#[test]
fn find_returns_the_named_animation_and_none_for_others() {
    let (object, _, walk) = two_animation_bank();
    let bank = Bank::read(&object).unwrap();
    assert_eq!(bank.find("Walk_t").map(|animation| animation.offset), Some(walk));
    assert!(bank.find("Walk").is_none(), "a prefix is not a name");
    assert!(bank.find("Run_q").is_none());
}

#[test]
fn an_animation_record_reads_as_a_compound() {
    let (object, _, walk) = two_animation_bank();
    let bank = Bank::read(&object).unwrap();
    let animation = bank.find("Walk_t").expect("Walk_t is in the bank");
    let compound = Compound::read(&object, animation.offset).unwrap();
    assert_eq!((compound.frames, compound.children.len()), (12, 1));
    assert_eq!(compound.children[0].type_id, 10);
    assert_eq!(animation.offset, walk);
}

#[test]
fn find_falls_back_to_a_scan_when_the_names_are_not_sorted() {
    // In [Zeta_q, Alpha_q] the binary search probes Alpha_q, sees it is smaller and misses Zeta_q. The scan finds it.
    let mut data = Data::default();
    let zeta_child = child(&mut data, 10, 0x1414);
    let zeta = compound(&mut data, 0x1414, 5, None, &[zeta_child]);
    let alpha_child = child(&mut data, 10, 0x1414);
    let alpha = compound(&mut data, 0x1414, 6, None, &[alpha_child]);
    let header = bank(&mut data, &[("Zeta_q", zeta), ("Alpha_q", alpha)]);
    let object = Object::read(&data.builder(Some(header)).padded()).unwrap();
    let bank = Bank::read(&object).unwrap();
    assert_eq!(bank.find("Zeta_q").map(|animation| animation.offset), Some(zeta));
    assert_eq!(bank.find("Alpha_q").map(|animation| animation.offset), Some(alpha));
}

#[test]
fn an_empty_bank_has_null_arrays() {
    let mut data = Data::default();
    let header = bank_header(&mut data, 0, 0, 0);
    let object = Object::read(&data.builder(Some(header)).padded()).unwrap();
    let bank = Bank::read(&object).unwrap();
    assert!(bank.animations.is_empty());
    assert!(bank.find("Idle_q").is_none());
}

#[test]
fn a_missing_bank_symbol_is_an_error() {
    let mut data = Data::default();
    let child_at = child(&mut data, 10, 0);
    compound(&mut data, 0, 1, None, &[child_at]);
    let object = Object::read(&data.builder(None).padded()).unwrap();
    assert!(matches!(Bank::read(&object), Err(Error::Invalid(_))));
}

#[test]
fn a_header_of_the_wrong_size_is_an_error() {
    let mut header = [0u8; 0x1C];
    header[0] = 0x20;
    let mut data = Data::default();
    let at = data.put_bytes(&header);
    let object = Object::read(&data.builder(Some(at)).padded()).unwrap();
    assert!(matches!(Bank::read(&object), Err(Error::Invalid(_))));
}

#[test]
fn a_null_entry_inside_the_count_is_an_error() {
    // The count says 3, but the third entry of both arrays is zero and not relocated.
    let mut data = Data::default();
    let first_child = child(&mut data, 10, 0);
    let first = compound(&mut data, 0, 1, None, &[first_child]);
    let second_child = child(&mut data, 10, 0);
    let second = compound(&mut data, 0, 1, None, &[second_child]);
    let names_first = data.put_bytes(b"A_q\0");
    let names_second = data.put_bytes(b"B_q\0");
    data.align();
    let names = data.here();
    data.put_pointer(names_first);
    data.put_pointer(names_second);
    data.put_u32(0);
    let records = data.here();
    data.put_pointer(first);
    data.put_pointer(second);
    data.put_u32(0);
    let header = bank_header(&mut data, 3, records, names);
    let object = Object::read(&data.builder(Some(header)).padded()).unwrap();
    assert!(matches!(Bank::read(&object), Err(Error::Invalid(_))));
}

#[test]
fn a_count_that_runs_past_data_is_truncated() {
    // The arrays sit at the end of .data; the count says 0x1000, so the read runs past the end.
    let mut data = Data::default();
    let first_child = child(&mut data, 10, 0);
    let first = compound(&mut data, 0, 1, None, &[first_child]);
    let second_child = child(&mut data, 10, 0);
    let second = compound(&mut data, 0, 1, None, &[second_child]);
    let names_first = data.put_bytes(b"A_q\0");
    let names_second = data.put_bytes(b"B_q\0");
    data.align();
    let header = data.here();
    let records = header + 0x1C;
    let names = records + 8;
    bank_header(&mut data, 0x1000, records, names);
    data.put_pointer(first);
    data.put_pointer(second);
    data.put_pointer(names_first);
    data.put_pointer(names_second);
    let object = Object::read(&data.builder(Some(header)).padded()).unwrap();
    assert!(matches!(Bank::read(&object), Err(Error::Truncated { .. })));
}
