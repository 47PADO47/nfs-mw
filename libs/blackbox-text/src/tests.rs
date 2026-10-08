use super::*;

/// A language payload with the given `(label, packed bytes)` strings and histogram entries.
fn payload(strings: &[(&str, &[u8])], entries: &[(usize, u16)]) -> Vec<u8> {
    let mut hist = vec![0u16; HISTOGRAM_ENTRIES];
    for (i, v) in entries {
        hist[*i] = *v;
    }
    let mut records: Vec<(u32, u32)> = Vec::new();
    let mut blob = Vec::new();
    for (label, bytes) in strings {
        records.push((label_hash(label), blob.len() as u32));
        blob.extend_from_slice(bytes);
        blob.push(0);
    }
    records.sort();
    let hist_pos = 0x20usize;
    let rec_pos = hist_pos + 4 + HISTOGRAM_ENTRIES * 2;
    let str_pos = rec_pos + records.len() * 8;
    let mut out = vec![0u8; hist_pos];
    out[0..4].copy_from_slice(&(hist_pos as u32).to_le_bytes());
    out[4..8].copy_from_slice(&(records.len() as u32).to_le_bytes());
    out[8..12].copy_from_slice(&(rec_pos as u32).to_le_bytes());
    out[12..16].copy_from_slice(&(str_pos as u32).to_le_bytes());
    out.extend_from_slice(&0x100u32.to_le_bytes());
    for h in hist {
        out.extend_from_slice(&h.to_le_bytes());
    }
    for (h, o) in records {
        out.extend_from_slice(&h.to_le_bytes());
        out.extend_from_slice(&o.to_le_bytes());
    }
    out.extend(blob);
    out
}

fn chunk(payload: &[u8]) -> Vec<u8> {
    let mut out = LANGUAGE_CHUNK.to_le_bytes().to_vec();
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(payload);
    out
}

#[test]
fn ascii_strings_look_up_by_label_hash() {
    let t = StringTable::parse(&payload(&[("KMH", b"KM/H"), ("MPH", b"MPH")], &[])).unwrap();
    assert_eq!(t.len(), 2);
    assert_eq!(t.get(label_hash("KMH")).as_deref(), Some("KM/H"));
    assert_eq!(t.get(label_hash("MPH")).as_deref(), Some("MPH"));
    assert_eq!(t.get(label_hash("OTHER")), None);
    assert_eq!(t.labels().count(), 2);
}

#[test]
fn histogram_bytes_unpack_to_wide_characters() {
    // 0x90 -> U+00E9; 0x91 is a prefix (1) and the next byte 0xA0 selects entry 0xA0 -> the euro sign;
    // 0x92 has no entry and unpacks as an underscore.
    let t = StringTable::parse(&payload(
        &[("A", &[b'H', b'i', 0x90, b' ', 0x91, 0xA0, 0x92])],
        &[(0x90, 0x00E9), (0x91, 0x0001), (0xA0, 0x20AC)],
    ))
    .unwrap();
    assert_eq!(t.get(label_hash("A")).as_deref(), Some("Hi\u{e9} \u{20ac}_"));
}

#[test]
fn finds_the_chunk_in_a_file() {
    let file = chunk(&payload(&[("X", b"yes")], &[]));
    let t = StringTable::from_file(&file).unwrap();
    assert_eq!(t.get(label_hash("X")).as_deref(), Some("yes"));
    assert!(matches!(StringTable::from_file(&[0; 16]), Err(Error::NoLanguageChunk)));
}

#[test]
fn damaged_input_is_an_error_not_a_panic() {
    assert!(StringTable::parse(&[]).is_err());
    assert!(StringTable::parse(&[0xFF; 40]).is_err());
    let good = payload(&[("X", b"yes")], &[]);
    for cut in [8, 20, 100, good.len() - 2] {
        let _ = StringTable::parse(&good[..cut]);
    }
}
