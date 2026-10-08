use crate::big;
use crate::source::ReadAt;
use crate::tests::build::*;

fn small_stream(start: i16, blocks: usize) -> Vec<u8> {
    let header = pt_header(0, &[tag(0x80, 2), tag(0x85, 28 * blocks as u32)]);
    let frame = xa_pcm_frame(0, 0, &ramp(start));
    let data: Vec<Vec<u8>> = (0..blocks).map(|_| scdl(28, &[0], &frame, false)).collect();
    stream(&header, &data)
}

/// Three streams on 0x100 boundaries with a table (not a stream) after the second one.
fn container() -> Vec<u8> {
    let mut file = small_stream(1, 1);
    file.resize(0x200, 0);
    file.extend(small_stream(2, 3));
    file.extend([0xDE, 0xAD, 0xBE, 0xEF, 1, 2, 3, 4, 5, 6, 7]); // a table right after the stream end
    file.resize(0x600, 0);
    file.extend(small_stream(3, 1));
    file.resize(file.len().div_ceil(0x100) * 0x100 + 0x30, 0);
    file
}

#[test]
fn scan_finds_every_stream_and_skips_tables_and_padding() {
    let file = container();
    let entries = big::scan(&file[..]).unwrap();
    let offsets: Vec<u64> = entries.iter().map(|e| e.offset).collect();
    assert_eq!(offsets, [0, 0x200, 0x600]);
    assert_eq!(entries[0].len as usize, small_stream(1, 1).len());
    assert_eq!(entries[1].len as usize, small_stream(2, 3).len());
}

#[test]
fn entries_decode_from_any_source() {
    let file = container();
    let entries = big::scan(&file[..]).unwrap();
    let pcm = entries[1].decode(&file[..]).unwrap();
    assert_eq!(pcm.frames(), 84);
    assert_eq!(&pcm.samples[..28], &ramp(2));
    assert_eq!(entries[2].decode(&file[..]).unwrap().samples, ramp(3));
}

#[test]
fn an_empty_or_streamless_file_has_no_entries() {
    assert!(big::scan(&[][..]).unwrap().is_empty());
    assert!(big::scan(&vec![0u8; 0x400][..]).unwrap().is_empty());
}

#[test]
fn a_damaged_stream_is_an_error() {
    let mut file = container();
    file.truncate(0x200 + 40);
    assert!(big::scan(&file[..]).is_err());
}

#[test]
fn read_at_never_reads_past_the_end() {
    let data = vec![1u8, 2, 3];
    let mut buf = [0u8; 8];
    assert_eq!(data.as_slice().read_at(1, &mut buf), 2);
    assert_eq!(&buf[..2], &[2, 3]);
    assert_eq!(data.read_at(3, &mut buf), 0);
    assert_eq!(data.read_at(u64::MAX, &mut buf), 0);
    assert_eq!(ReadAt::len(&data), 3);
    assert!(!data.is_empty());
    assert!(crate::source::read_vec(&data, 2, 5).is_err());
    assert_eq!(crate::source::read_vec(&data, 1, 2).unwrap(), [2, 3]);
    assert!(crate::source::read_array::<4, _>(&data, 0).is_err());
}
