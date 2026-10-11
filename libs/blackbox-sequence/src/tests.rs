use blackbox_carp::{tag, tag4};

use crate::Package;

fn words(values: &[u32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

fn blob(records: &[(u32, u32, Vec<u8>)]) -> Vec<u8> {
    let mut out = words(&[tag4(b"CARP"), 0x1a, records.len() as u32, 1]);
    out.resize(16 + records.len() * 16, 0);
    for (i, (key, count, data)) in records.iter().enumerate() {
        let at = 16 + i * 16;
        let header = words(&[*key, (data.len() as u32) << 8 | 2, *count, (out.len() - at) as u32]);
        out[at..at + 16].copy_from_slice(&header);
        out.extend(data);
    }
    out
}

fn records() -> Vec<(u32, u32, Vec<u8>)> {
    vec![
        (tag(*b"iN", 0), 77, b"Synthetic\0".to_vec()),
        (tag(*b"eE", 0), 77, words(&[tag(*b"iN", 0), 1, 10, tag(*b"eS", 0)])),
        (tag(*b"eS", 0), 10, words(&[20, 20, 1, 20, tag(*b"eT", 0)])),
        (tag(*b"eT", 0), 20, words(&[1, 0, 30, 20, tag(*b"eA", 0)])),
        (
            tag(*b"eA", 0),
            40,
            words(&[
                40,
                1,
                2,
                2.0_f32.to_bits(),
                50,
                0,
                1.0_f32.to_bits(),
                tag(*b"el", 0),
                tag(*b"el", 0),
                tag(*b"el", 1),
            ]),
        ),
        (tag(*b"el", 0), 60, words(&[2, 0, 0, 0, 91, 8, 32, 0, 92, 4, 24, 0, 91, 1234, 92])),
        (tag(*b"el", 1), 61, words(&[0, 0, 0, 0])),
    ]
}

#[test]
fn resolves_tag_references_and_descriptor_relative_event_data() {
    let p = Package::parse(&blob(&records())).unwrap();
    let engine = &p.engines[0];
    assert_eq!((engine.name.as_str(), engine.name_hash), ("Synthetic", 77));
    assert_eq!(engine.systems[&10].states[&20].responses[&30].action, tag(*b"eA", 0));
    let action = &p.actions[&tag(*b"eA", 0)];
    assert_eq!(action.times, [(0.0, tag(*b"el", 0)), (1.0, tag(*b"el", 1))]);
    let events = &p.event_lists[&tag(*b"el", 0)];
    assert_eq!(events.iter().map(|e| e.id).collect::<Vec<_>>(), [91, 92]);
    assert_eq!(events[0].data, words(&[91, 1234]));
}

#[test]
fn every_truncation_is_rejected_without_a_panic() {
    let bytes = blob(&records());
    for end in 0..bytes.len() {
        assert!(Package::parse(&bytes[..end]).is_err(), "accepted {end}/{} bytes", bytes.len());
    }
}

#[test]
fn rejects_missing_references_duplicate_tags_and_bad_event_ranges() {
    let mut r = records();
    r.remove(0);
    assert!(Package::parse(&blob(&r)).is_err());
    let mut r = records();
    r.push(r[0].clone());
    assert!(Package::parse(&blob(&r)).is_err());
    for (offset, value) in [(20, u32::MAX), (24, u32::MAX), (24, 0), (48, 999)] {
        let mut r = records();
        r[5].2[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        assert!(Package::parse(&blob(&r)).is_err(), "accepted event mutation at {offset}");
    }
}

#[test]
fn rejects_nonfinite_and_unordered_times() {
    for (offset, value) in [(12, f32::NAN), (20, 2.0), (24, f32::INFINITY)] {
        let mut r = records();
        r[4].2[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        assert!(Package::parse(&blob(&r)).is_err());
    }
}
