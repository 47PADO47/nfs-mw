use super::*;

/// Wraps a payload in an 8-byte chunk header (id, length).
fn chunk_bytes(id: u32, payload: &[u8]) -> Vec<u8> {
    let mut v = id.to_le_bytes().to_vec();
    v.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    v.extend_from_slice(payload);
    v
}

/// A 0x28-byte camera record header: start at 0x0C, length at 0x10, element count at 0x14, name at 0x17.
fn record_header(name: &str, length: f32, count: i16) -> Vec<u8> {
    let mut h = vec![0u8; 0x28];
    h[0x0C..0x10].copy_from_slice(&0.0f32.to_le_bytes());
    h[0x10..0x14].copy_from_slice(&length.to_le_bytes());
    h[0x14..0x16].copy_from_slice(&count.to_le_bytes());
    h[0x17..0x17 + name.len()].copy_from_slice(name.as_bytes());
    h
}

/// One 0x84-byte key: flag at +0, time at +0x10, zeroed elsewhere (both space bytes are 0, the car space).
fn key_bytes(time: f32) -> Vec<u8> {
    let mut e = vec![0u8; 0x84];
    e[0] = 1;
    e[0x10..0x14].copy_from_slice(&time.to_le_bytes());
    e
}

/// A record: its header, then `keys` keys.
fn record(name: &str, length: f32, keys: usize) -> Vec<u8> {
    let mut r = record_header(name, length, keys as i16);
    for i in 0..keys {
        r.extend(key_bytes(i as f32 / keys as f32));
    }
    r
}

/// A camera set: its key, its record count, then the records.
fn camera_set(key: u32, records: &[Vec<u8>]) -> Vec<u8> {
    let mut v = key.to_le_bytes().to_vec();
    v.extend_from_slice(&(records.len() as u32).to_le_bytes());
    for r in records {
        v.extend_from_slice(r);
    }
    v
}

/// A pack chunk with the given id and sets.
fn pack(id: u32, sets: &[Vec<u8>]) -> Vec<u8> {
    let payload: Vec<u8> = sets.iter().flat_map(|s| chunk_bytes(0x0003_0001, s)).collect();
    chunk_bytes(id, &payload)
}

/// `InGameB.bun` in miniature: a NIS pack and an FMV pack, with one scene each and a set with no tracks.
fn ingame() -> Vec<u8> {
    let intro = camera_set(bstring_hash("IntroNis05"), &[record("Track 0", 3.0, 1), record("Track 1", 4.0, 2)]);
    let empty = camera_set(bstring_hash("ArrestF02"), &[]);
    let fmv = camera_set(bstring_hash("FMVroc02"), &[record("Track 0", 2.0, 1)]);
    [pack(ids::ICE_NIS_CAMERAS, &[intro, empty]), pack(ids::ICE_FMV_CAMERAS, &[fmv])].concat()
}

#[test]
fn the_sets_are_found_by_scene_name_in_their_own_pack() {
    let cameras = CameraLibrary::from_bytes(&ingame()).unwrap();
    assert_eq!(cameras.nis_count(), 2);
    assert_eq!(cameras.fmv_count(), 1);
    assert_eq!(cameras.nis("IntroNis05").map(<[_]>::len), Some(2));
    assert_eq!(cameras.nis("ArrestF02"), Some(&[][..]), "a set with no tracks is still a set");
    assert_eq!(cameras.fmv("FMVroc02").map(<[_]>::len), Some(1));
    assert!(cameras.nis("FMVroc02").is_none(), "the FMV pack is not searched for NIS sets");
    assert!(cameras.fmv("IntroNis05").is_none());
    assert!(cameras.nis("NoSuchScene").is_none());
    assert_eq!(cameras.nis_set(bstring_hash("IntroNis05")), cameras.nis("IntroNis05"));
    assert_eq!(cameras.fmv_set(bstring_hash("FMVroc02")), cameras.fmv("FMVroc02"));
}

#[test]
fn a_track_is_found_by_its_number() {
    let cameras = CameraLibrary::from_bytes(&ingame()).unwrap();
    let second = cameras.track("IntroNis05", 1).unwrap();
    assert_eq!(second.name, "Track 1");
    assert_eq!(second.length, 4.0);
    assert_eq!(second.keys.len(), 2);
    assert_eq!(cameras.track("IntroNis05", 0).map(|t| t.length), Some(3.0));
    assert!(cameras.track("IntroNis05", 2).is_none(), "the set has no Track 2");
    assert!(cameras.track("ArrestF02", 0).is_none());
    assert_eq!(cameras.fmv_track("FMVroc02", 0).map(|t| t.length), Some(2.0));
    assert!(cameras.fmv_track("FMVroc02", 1).is_none());
}

#[test]
fn the_sets_iterate_in_key_order_with_their_tracks() {
    let cameras = CameraLibrary::from_bytes(&ingame()).unwrap();
    let keys: Vec<u32> = cameras.nis_sets().map(|(key, _)| key).collect();
    assert!(keys.windows(2).all(|pair| pair[0] < pair[1]), "{keys:?}");
    let sizes: BTreeMap<u32, usize> = cameras.nis_sets().map(|(key, tracks)| (key, tracks.len())).collect();
    let expected = BTreeMap::from([(bstring_hash("IntroNis05"), 2), (bstring_hash("ArrestF02"), 0)]);
    assert_eq!(sizes, expected);
    assert_eq!(cameras.fmv_sets().count(), 1);
}

#[test]
fn bytes_without_packs_give_an_empty_library() {
    let cameras = CameraLibrary::from_bytes(&[]).unwrap();
    assert_eq!((cameras.nis_count(), cameras.fmv_count()), (0, 0));
    assert!(cameras.track("IntroNis05", 0).is_none());
}

#[test]
fn a_set_can_be_added_by_hand() {
    let mut cameras = CameraLibrary::default();
    let track = Track { name: "Track 0".into(), start: 0.0, length: 1.0, keys: Vec::new() };
    cameras.insert_nis(bstring_hash("Hand"), vec![track]);
    assert_eq!(cameras.track("Hand", 0).map(|t| t.length), Some(1.0));
    assert_eq!(cameras.nis_count(), 1);
}
