use crate::*;

fn chunk(id: u32, payload: &[u8]) -> Vec<u8> {
    let mut v = id.to_le_bytes().to_vec();
    v.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    v.extend_from_slice(payload);
    v
}

#[test]
fn nested_container() {
    let inner = [chunk(0, b""), chunk(0x0013_4011, b"ABCD"), chunk(0x0013_4012, b"12345678")].concat();
    let data = chunk(0x8013_4000, &inner);
    let top: Vec<_> = chunks(&data).collect::<Result<_>>().unwrap();
    assert_eq!(top.len(), 1);
    assert!(top[0].is_container());
    let kids: Vec<_> = top[0].children().collect::<Result<_>>().unwrap();
    assert_eq!(kids.iter().map(|c| c.id).collect::<Vec<_>>(), [0, 0x0013_4011, 0x0013_4012]);
    assert!(kids[0].is_padding());
    assert_eq!(kids[1].payload, b"ABCD");
    assert_eq!(kids[1].offset, 16);
    assert_eq!(find(&data, 0x0013_4012).unwrap().payload, b"12345678");
}

#[test]
fn overrun_is_error() {
    let mut data = chunk(0x0003_4600, b"abcd");
    data[4] = 99;
    assert!(matches!(chunks(&data).next(), Some(Err(Error::Overrun { .. }))));
}

#[test]
fn trailing_bytes_are_error() {
    let mut data = chunk(1, b"");
    data.extend_from_slice(&[1, 2, 3]);
    let items: Vec<_> = chunks(&data).collect();
    assert!(items[0].is_ok());
    assert!(matches!(items[1], Err(Error::Trailing(3, 8))));
}

#[test]
fn aligned_payload_skips_padding_by_address() {
    // Payload starts at 8; aligned to 0x10 it starts at 16, so 8 pad bytes are skipped
    // even though the real data also begins with 0x11.
    let mut payload = vec![PAD_BYTE; 8];
    payload.extend_from_slice(&[0x11, 0x22]);
    let data = chunk(0x0013_4011, &payload);
    let c = chunks(&data).next().unwrap().unwrap();
    assert_eq!(c.aligned_payload(0x10), &[0x11, 0x22]);
}

#[test]
fn bare_jdlz_blob_is_one_item() {
    let mut blob = b"JDLZ".to_vec();
    blob.extend_from_slice(&[2, 0x10, 0, 0]);
    blob.extend_from_slice(&1u32.to_le_bytes());
    blob.extend_from_slice(&19u32.to_le_bytes());
    blob.extend_from_slice(&[0, 0, b'z']);
    let data = [blob.clone(), chunk(5, b"")].concat();
    let items: Vec<_> = chunks(&data).collect::<Result<_>>().unwrap();
    assert!(items[0].is_bare_jdlz());
    assert_eq!(items[0].payload, &blob[..]);
    assert_eq!(items[1].id, 5);
}
