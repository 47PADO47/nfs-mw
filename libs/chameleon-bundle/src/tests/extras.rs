//! The header's second part, the entry tail, ids, import kinds and debug names, on synthetic bundles.

use crate::{
    Bundle, FLAG_HAS_DEFAULT_RESOURCE, FLAG_MULTISTREAM, IdClass, ImportKind, ResourceId, ResourceKind,
    parse_debug_names,
};

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// A plain bundle with the table at 0x70: a default resource, two named streams and one resource in stream 1 whose
/// main block holds a single import with the handle bit set.
fn build_multistream() -> Vec<u8> {
    let data_start = 0x70 + 0x48;
    let main: Vec<u8> =
        [&[0u8; 8][..], &0x0100_0000_0000_0005u64.to_le_bytes(), &(0x8000_0004u32).to_le_bytes(), &[0; 4]].concat();
    let end = data_start + main.len();
    let mut out = Vec::new();
    out.extend_from_slice(b"bnd2");
    out.extend_from_slice(&5u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    for value in [end as u32, 1, 0x70, data_start as u32, end as u32, end as u32, end as u32] {
        push_u32(&mut out, value);
    }
    push_u32(&mut out, FLAG_MULTISTREAM | FLAG_HAS_DEFAULT_RESOURCE);
    out.extend_from_slice(&0x0100_0000_0000_0042u64.to_le_bytes());
    push_u32(&mut out, 0);
    for name in [&b"GRAPHICS"[..], b"SOUND", b"", b""] {
        let mut slot = [0u8; 15];
        slot[..name.len()].copy_from_slice(name);
        out.extend_from_slice(&slot);
    }
    assert_eq!(out.len(), 0x70);
    out.extend_from_slice(&0x0000_0001_0000_BEEFu64.to_le_bytes());
    for size in [main.len() as u32 | 0x4000_0000, 0, 0, 0, main.len() as u32, 0, 0, 0, 0, 0, 0, 0] {
        push_u32(&mut out, size);
    }
    push_u32(&mut out, 8);
    push_u32(&mut out, ResourceKind::GenesysObject.id());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&[0, 1]);
    push_u32(&mut out, 0);
    out.extend_from_slice(&main);
    out
}

#[test]
fn reads_the_default_resource_and_the_stream_names() {
    let file = build_multistream();
    let bundle = Bundle::parse(&file).unwrap();
    let header = bundle.header();
    assert!(header.has_flags(FLAG_MULTISTREAM | FLAG_HAS_DEFAULT_RESOURCE));
    assert_eq!((header.default_resource, header.default_stream), (0x0100_0000_0000_0042, 0));
    assert_eq!(header.stream_name(0), Some("GRAPHICS"));
    assert_eq!(header.stream_name(1), Some("SOUND"));
    assert_eq!(header.stream_name(2), None);
    assert_eq!(header.stream_name(9), None);
    let entry = bundle.entries()[0];
    assert_eq!((entry.flags, entry.stream_index), (0, 1));
}

#[test]
fn a_short_header_has_no_streams() {
    let mut file = build_multistream();
    file.truncate(0x28);
    let header = crate::Header::parse(&file).unwrap();
    assert_eq!((header.default_resource, header.default_stream, header.stream_name(0)), (0, -1, None));
}

#[test]
fn separates_the_handle_bit_from_the_import_offset() {
    let file = build_multistream();
    let bundle = Bundle::parse(&file).unwrap();
    let resource = bundle.resource(0).unwrap();
    assert_eq!(resource.imports.len(), 1);
    let import = resource.imports[0];
    assert_eq!((import.id, import.offset, import.slot()), (0x0100_0000_0000_0005, 0x8000_0004, 4));
    assert_eq!(import.kind(), ImportKind::ResourceHandle);
    assert_eq!(crate::Import { id: 1, offset: 4 }.kind(), ImportKind::Pointer);
}

#[test]
fn takes_an_id_apart() {
    let id = ResourceId(0x0100_0000_0019_4C91);
    assert_eq!((id.key(), id.sub_type(), id.index(), id.class()), (0x19_4C91, 0, 0, IdClass::GameChanger));
    let id = ResourceId::from_parts(0xDEAD_BEEF, 0x53, 2, 0);
    assert_eq!(id.0, 0x0002_0053_DEAD_BEEF);
    assert_eq!((id.key(), id.sub_type(), id.index(), id.class()), (0xDEAD_BEEF, 0x53, 2, IdClass::Normal));
    assert_eq!(ResourceId(0xC0 << 56).class(), IdClass::DeltaBundle);
    assert_eq!(ResourceId(0x42 << 56).class(), IdClass::Other(0x42));
    assert_eq!(id.to_string(), "00020053DEADBEEF");
}

#[test]
fn every_kind_round_trips_its_id() {
    for &kind in ResourceKind::ALL {
        assert_eq!(ResourceKind::from_id(kind.id()), Some(kind));
        assert!(!kind.name().is_empty());
    }
    assert_eq!(ResourceKind::from_id(0x201).map(ResourceKind::name), Some("LocalisedText"));
    assert_eq!(ResourceKind::from_id(0xFFFF), None);
    let ids: std::collections::HashSet<u32> = ResourceKind::ALL.iter().map(|kind| kind.id()).collect();
    assert_eq!(ids.len(), ResourceKind::ALL.len());
}

#[test]
fn reads_the_debug_name_table() {
    let xml = "<ResourceStringTable>\n\t<Resource id=\"0100000000194c91\" type=\"Texture\" name=\"GameChanger (ID=1658001,type=0,index=0)\"/>\n\
               \t<Resource id=\"DEADBEEF\" type=\"Material\" name=\"a &amp; b\" streamIndex=\"2\"/>\n\
               \t<Resource id=\"123\" type=\"Texture\" name=\"short id\"/>\n\
               \t<Resource id=\"0000000000000001\" type=\"Texture\" streamIndex=\"1\"/>\n\
               \t<Resource id=\"0000000000000002\" type=\"Model\" name=\"m\" streamIndex=\"importEntry\"/>\n\
               </ResourceStringTable>";
    let names = parse_debug_names(xml);
    assert_eq!(names.len(), 3);
    assert_eq!(names[0].id.class(), IdClass::GameChanger);
    assert_eq!((names[0].kind.as_str(), names[0].stream), ("Texture", None));
    assert_eq!((names[1].id.0, names[1].name.as_str(), names[1].stream), (0xDEAD_BEEF, "a & b", Some(2)));
    assert_eq!((names[2].kind.as_str(), names[2].stream), ("Model", None));
}
