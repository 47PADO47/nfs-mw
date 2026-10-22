//! Compound reader tests on synthetic records built by `fixture`. No install data is read here.

use super::fixture::{Data, attribute_block, child, compound};
use crate::compound::DEFAULT_FPS;
use crate::{ChildRef, Compound, Error, Object};

fn read(data: &Data, at: u32) -> crate::Result<Compound> {
    let object = Object::read(&data.builder(None).padded()).expect("the synthetic object reads");
    Compound::read(&object, at)
}

#[test]
fn reads_frames_fps_and_both_children() {
    let mut data = Data::default();
    let first = child(&mut data, 11, 0x1414);
    let second = child(&mut data, 10, 0x1414);
    let block = attribute_block(&mut data, &[(1, 30)]);
    let at = compound(&mut data, 0x1414, 41, Some(block), &[first, second]);
    let compound = read(&data, at).unwrap();
    assert_eq!(compound.checksum, 0x1414);
    assert_eq!(compound.frames, 41);
    assert_eq!(compound.fps, 30.0);
    assert_eq!(compound.children, [ChildRef { type_id: 11, offset: first }, ChildRef { type_id: 10, offset: second }]);
}

#[test]
fn duration_is_the_frame_count_over_the_frame_rate() {
    let mut data = Data::default();
    let only = child(&mut data, 10, 0);
    let block = attribute_block(&mut data, &[(1, 30)]);
    let at = compound(&mut data, 0, 41, Some(block), &[only]);
    let compound = read(&data, at).unwrap();
    assert!((compound.duration_secs() - 41.0 / 30.0).abs() < 1e-6, "41 frames at 30 fps is 41/30 s");
}

#[test]
fn the_fps_entry_sets_the_frame_rate() {
    let mut data = Data::default();
    let only = child(&mut data, 10, 0);
    let block = attribute_block(&mut data, &[(1, 25)]);
    let at = compound(&mut data, 0, 50, Some(block), &[only]);
    assert_eq!(read(&data, at).unwrap().fps, 25.0);
}

#[test]
fn a_null_attribute_pointer_gives_the_default_fps() {
    let mut data = Data::default();
    let only = child(&mut data, 10, 0);
    let at = compound(&mut data, 0, 7, None, &[only]);
    assert_eq!(read(&data, at).unwrap().fps, DEFAULT_FPS);
}

#[test]
fn a_block_without_an_fps_entry_gives_the_default_fps() {
    let mut data = Data::default();
    let only = child(&mut data, 10, 0);
    let block = attribute_block(&mut data, &[(2, 25)]);
    let at = compound(&mut data, 0, 7, Some(block), &[only]);
    assert_eq!(read(&data, at).unwrap().fps, DEFAULT_FPS);
}

#[test]
fn a_zero_fps_entry_gives_the_default_fps() {
    let mut data = Data::default();
    let only = child(&mut data, 10, 0);
    let block = attribute_block(&mut data, &[(1, 0)]);
    let at = compound(&mut data, 0, 7, Some(block), &[only]);
    assert_eq!(read(&data, at).unwrap().fps, DEFAULT_FPS);
}

#[test]
fn an_unreadable_attribute_pointer_gives_the_default_fps() {
    // The attribute word is not relocated, and it is not zero: it is no pointer, so the frame rate falls back.
    let mut data = Data::default();
    let only = child(&mut data, 10, 0);
    let at = data.put_u16(15);
    data.put_u16(0);
    data.put_u32(0x1234);
    data.put_u16(1);
    data.put_u16(7);
    data.put_pointer(only);
    assert_eq!(read(&data, at).unwrap().fps, DEFAULT_FPS);
}

#[test]
fn a_record_that_is_not_a_compound_is_an_error() {
    let mut data = Data::default();
    let at = child(&mut data, 10, 0x1414);
    assert!(matches!(read(&data, at), Err(Error::Invalid(_))));
}

#[test]
fn a_child_pointer_outside_data_is_an_error() {
    let mut data = Data::default();
    let at = compound(&mut data, 0, 1, None, &[0x1000]);
    assert!(matches!(read(&data, at), Err(Error::Invalid(_))));
}

#[test]
fn a_child_count_larger_than_the_pointers_is_an_error() {
    // The count says 2, but the second child pointer is zero and not relocated.
    let mut data = Data::default();
    let only = child(&mut data, 10, 0);
    data.align();
    let at = data.put_u16(15);
    data.put_u16(0);
    data.put_u32(0);
    data.put_u16(2);
    data.put_u16(1);
    data.put_pointer(only);
    data.put_u32(0);
    assert!(matches!(read(&data, at), Err(Error::Invalid(_))));
}

#[test]
fn a_child_of_an_unknown_type_is_an_error() {
    let mut data = Data::default();
    let odd = child(&mut data, 99, 0);
    let at = compound(&mut data, 0, 1, None, &[odd]);
    assert!(matches!(read(&data, at), Err(Error::Invalid(_))));
}
