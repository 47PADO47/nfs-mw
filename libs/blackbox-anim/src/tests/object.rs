use super::elf_builder::Builder;
use crate::{Error, Object};

/// Generic bone names; they stand in for the rig names of real objects.
const ROOT: &str = "__Bone:::rig.root";
const TIP: &str = "__Bone:::rig.tip";

/// A 32-byte `.data`: a word of 4 at offset 0 (relocated against `ROOT`, so it becomes 4 + 0x10), a plain word of
/// 42 at offset 4, a zero word at offset 8, and the string "root" at offset 0x14.
fn sample() -> Vec<u8> {
    let mut data = vec![0u8; 0x20];
    data[0..4].copy_from_slice(&4u32.to_le_bytes());
    data[4..8].copy_from_slice(&42u32.to_le_bytes());
    data[0x14..0x19].copy_from_slice(b"root\0");
    data
}

fn sample_builder() -> Builder {
    Builder::new(&sample())
        .symbol("__Skeleton:::rig", 0)
        .symbol(ROOT, 0x10)
        .symbol(TIP, 0x18)
        .symbol("__AnimationBank:::bank", 0)
        .reloc_to(0, ROOT)
}

#[test]
fn reads_symbols_and_applies_the_relocation() {
    let object = Object::read(&sample_builder().padded()).unwrap();
    assert_eq!(object.symbols.len(), 6, "null, section symbol and four user symbols");
    let root = object.symbol(ROOT).expect("bone symbol");
    assert_eq!((root.value, root.size, root.section), (0x10, 4, 1));
    assert!(object.symbol("__Bone:::rig.missing").is_none());
    let bones: Vec<&str> = object.symbols_with_prefix("__Bone:::").map(|symbol| symbol.name.as_str()).collect();
    assert_eq!(bones, [ROOT, TIP]);
    assert_eq!(object.u32_at(0), Ok(0x14));
    assert_eq!(object.pointers, [0]);
}

#[test]
fn detects_pointers_and_reads_targets() {
    let object = Object::read(&sample_builder().padded()).unwrap();
    assert!(object.is_pointer(0));
    assert!(!object.is_pointer(4));
    assert_eq!(object.ptr_at(0), Ok(Some(0x14)));
    assert_eq!(object.ptr_at(8), Ok(None), "a zero word that is not relocated is no pointer");
    assert_eq!(object.cstr_at(0x14), Ok("root"));
}

#[test]
fn pad_is_optional() {
    let builder = sample_builder();
    let padded = Object::read(&builder.padded()).unwrap();
    let bare = Object::read(&builder.bare()).unwrap();
    assert_eq!(padded, bare);
}

#[test]
fn reads_little_endian_scalars() {
    let mut data = vec![0xAB, 0xCD, 0x12, 0x34];
    data.extend(1.5f32.to_le_bytes());
    let object = Object::read(&Builder::new(&data).padded()).unwrap();
    assert_eq!(object.u8_at(0), Ok(0xAB));
    assert_eq!(object.u16_at(0), Ok(0xCDAB));
    assert_eq!(object.u32_at(0), Ok(0x3412_CDAB));
    assert_eq!(object.f32_at(4), Ok(1.5));
    assert_eq!(object.u32_at(6), Err(Error::Truncated { what: "EAGL object data", offset: 6 }));
}

#[test]
fn cstr_needs_a_terminator_and_ascii() {
    let object = Object::read(&Builder::new(b"abc").bare()).unwrap();
    assert!(matches!(object.cstr_at(0), Err(Error::Truncated { .. })));

    let object = Object::read(&Builder::new(b"a\xFF\0").bare()).unwrap();
    assert!(matches!(object.cstr_at(0), Err(Error::Invalid(_))));
}

#[test]
fn rejects_bad_magic() {
    assert_eq!(Object::read(&[]), Err(Error::BadMagic));
    assert_eq!(Object::read(&[0x7F, b'E']), Err(Error::BadMagic));
    assert_eq!(Object::read(&[0x11; 8]), Err(Error::BadMagic));

    let mut padded = sample_builder().padded();
    padded[8 + 1] = b'X';
    assert_eq!(Object::read(&padded), Err(Error::BadMagic));

    let mut bare = sample_builder().bare();
    bare[0] = 0;
    assert_eq!(Object::read(&bare), Err(Error::BadMagic));
}

#[test]
fn rejects_truncated_header() {
    let bare = sample_builder().bare();
    assert_eq!(Object::read(&bare[..30]), Err(Error::Truncated { what: "ELF header", offset: 30 }));
}

#[test]
fn rejects_truncated_section_table() {
    let mut bare = sample_builder().bare();
    bare.truncate(bare.len() - 20);
    assert!(matches!(Object::read(&bare), Err(Error::Truncated { what: "section header table", .. })));
}

#[test]
fn rejects_other_classes_and_types() {
    let mut bare = sample_builder().bare();
    bare[4] = 2;
    assert!(matches!(Object::read(&bare), Err(Error::Unsupported(_))));

    let mut bare = sample_builder().bare();
    bare[16] = 2;
    assert!(matches!(Object::read(&bare), Err(Error::Unsupported(_))));
}

#[test]
fn rejects_relocation_past_the_end_of_data() {
    let object = Builder::new(&sample()).reloc(0x20).padded();
    assert!(matches!(Object::read(&object), Err(Error::Invalid(_))));
}

#[test]
fn rejects_two_relocations_at_one_offset() {
    let object = Builder::new(&sample()).reloc(0).reloc(0).padded();
    assert!(matches!(Object::read(&object), Err(Error::Invalid(_))));
}

#[test]
fn rejects_relocation_types_other_than_r_mips_32() {
    let object = Builder::new(&sample()).reloc_kind(0, 1).padded();
    assert!(matches!(Object::read(&object), Err(Error::Unsupported(_))));
}

#[test]
fn rejects_pointer_outside_data() {
    let data = [0x00, 0x01, 0, 0, 0, 0, 0, 0];
    let object = Object::read(&Builder::new(&data).reloc(0).padded()).unwrap();
    assert_eq!(object.u32_at(0), Ok(0x100));
    assert!(matches!(object.ptr_at(0), Err(Error::Invalid(_))));
}
