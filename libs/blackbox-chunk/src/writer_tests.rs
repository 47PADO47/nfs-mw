use crate::*;

#[test]
fn leaves_and_containers_round_trip_through_the_parser() {
    let mut w = ChunkWriter::new();
    w.container(0x0013_4000, |w| {
        w.leaf(0x0013_4011, b"ABCD");
        w.leaf(0x0013_4012, b"12345678");
    });
    let data = w.finish().unwrap();
    let top: Vec<_> = chunks(&data).collect::<Result<_>>().unwrap();
    assert_eq!(top.len(), 1);
    assert_eq!(top[0].id, 0x8013_4000, "the container bit is set for the caller");
    let kids: Vec<_> = top[0].children().collect::<Result<_>>().unwrap();
    assert_eq!(
        kids.iter().map(|c| (c.id, c.payload)).collect::<Vec<_>>(),
        [(0x0013_4011, &b"ABCD"[..]), (0x0013_4012, &b"12345678"[..])]
    );
}

#[test]
fn aligned_leaves_start_their_payload_on_the_alignment() {
    let mut w = ChunkWriter::new();
    w.leaf(1, b"abc");
    for align in [0x10, 0x80] {
        w.leaf_aligned(2, align, b"data");
        let payload_at = w.position() - 4;
        assert_eq!(payload_at % align, 0, "payload of an {align:#x}-aligned leaf");
    }
    let data = w.finish().unwrap();
    let all: Vec<_> = chunks(&data).collect::<Result<_>>().unwrap();
    assert_eq!(all[1].aligned_payload(0x10), b"data");
    assert_eq!(all[2].aligned_payload(0x80), b"data");
    assert!(all[1].payload[..all[1].payload.len() - 4].iter().all(|&b| b == PAD_BYTE));
}

#[test]
fn an_already_aligned_leaf_gets_no_fill() {
    let mut w = ChunkWriter::new();
    w.leaf(1, &[]);
    // The payload of the next chunk would start at 16.
    w.leaf_aligned(2, 0x10, b"xy");
    let data = w.finish().unwrap();
    let second = chunks(&data).nth(1).unwrap().unwrap();
    assert_eq!(second.payload, b"xy");
}

#[test]
fn padding_aligns_the_next_header() {
    let mut w = ChunkWriter::new();
    w.leaf(1, &[]);
    w.pad_to(0x10);
    assert_eq!(w.position(), 0x10);
    w.pad_to(0x10);
    assert_eq!(w.position(), 0x10, "already aligned: nothing added");
    w.leaf(1, &[7]);
    // 7 bytes short of 0x20: too few for a chunk header, so a whole alignment more is added.
    w.pad_to(0x10);
    assert_eq!(w.position(), 0x30);
    let data = w.finish().unwrap();
    let all: Vec<_> = chunks(&data).collect::<Result<_>>().unwrap();
    assert_eq!(all.iter().map(|c| (c.id, c.payload.len())).collect::<Vec<_>>(), [(1, 0), (0, 0), (1, 1), (0, 15)]);
    assert!(all[1].is_padding() && all[3].payload.iter().all(|&b| b == 0));
}

#[test]
fn append_keeps_a_prebuilt_stream() {
    let mut inner = ChunkWriter::new();
    inner.leaf_aligned(5, 0x80, b"v");
    let inner = inner.finish().unwrap();
    let mut w = ChunkWriter::new();
    w.pad_to(0x80);
    w.append(&inner);
    let data = w.finish().unwrap();
    assert_eq!(chunks(&data).count(), 1);
    assert_eq!(chunks(&data).next().unwrap().unwrap().aligned_payload(0x80), b"v");
}
