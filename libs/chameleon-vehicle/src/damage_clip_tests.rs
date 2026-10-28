//! The damage clips: a list built by hand, and every car of an install (set `CHAMELEON_GAME_DIR` and run with `--ignored`).

use std::path::Path;

use chameleon_bundle::Bundle;

use crate::damage_clip::{BREAK_STEP_M, read_list};
use crate::{damage_clips, hinged_parts};

/// The scale of a packed rotation field.
const PACK_SCALE: f32 = 0.001382;

fn put(block: &mut [u8], at: usize, value: u32) {
    block[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

/// A packed rotation about x by `degrees` (the w component left out).
fn packed(degrees: f32) -> u32 {
    let half = degrees.to_radians() / 2.0;
    let field = |v: f32| ((v / PACK_SCALE).round() as i32 + 511).clamp(0, 1023) as u32;
    (3 << 30) | (field(half.sin()) << 20) | (field(0.0) << 10) | field(0.0)
}

/// An animation list of one clip: three bones, three keys, and bone 1 moved by all three kinds of track.
fn list(format: u16) -> Vec<u8> {
    let mut block = vec![0u8; 0x20 + 0x100];
    block[0..2].copy_from_slice(&2u16.to_le_bytes());
    block[2..4].copy_from_slice(&1u16.to_le_bytes());
    put(&mut block, 8, 0x10);
    put(&mut block, 0x10, 0x20);
    let clip = 0x20;
    for (at, value) in [(0x04, 3), (0x08, 3), (0x1C, 4), (0x20, 16), (0x24, 4), (0x30, 6.0f32.to_bits())] {
        put(&mut block, clip + at, value);
    }
    block[clip + 0x34..clip + 0x36].copy_from_slice(&format.to_le_bytes());
    // Tables: translations 0x80, rotations 0xC0, scalars 0xD0, then the bone maps.
    for (n, at) in [(1, 0x80), (5, 0xC0), (8, 0xD0), (4, 0xE0), (7, 0xE4), (6, 0xE8)] {
        put(&mut block, clip + 0x38 + 4 * n, at);
    }
    for (key, y) in [0.0f32, -0.1, -1.0].into_iter().enumerate() {
        put(&mut block, clip + 0x80 + 16 * key + 4, y.to_bits());
        put(&mut block, clip + 0xC0 + 4 * key, packed(10.0 * key as f32));
        put(&mut block, clip + 0xD0 + 4 * key, if key < 2 { 1.0f32 } else { 0.0 }.to_bits());
    }
    for map in [0xE0, 0xE4, 0xE8] {
        block[clip + map..clip + map + 3].copy_from_slice(&[0xFF, 0, 0xFF]);
    }
    block
}

#[test]
fn a_clip_gives_each_bone_its_slide_turn_and_scalar() {
    let clips = read_list(&list(1));
    assert_eq!(clips.len(), 1);
    let clip = &clips[0];
    assert_eq!((clip.zone, clip.keys, clip.seconds), (1, 3, 6.0));
    assert_eq!(clip.tracks.len(), 1, "bones 0 and 2 have no track");
    let track = clip.track(1).unwrap();
    assert_eq!(track.translation[2], [0.0, -1.0, 0.0]);
    assert_eq!(track.scalar.as_deref(), Some(&[1.0, 1.0, 0.0][..]));
    let half = 5f32.to_radians();
    assert!((track.rotation[1][0] - half.sin()).abs() < 0.002 && (track.rotation[1][3] - half.cos()).abs() < 0.002);
    assert_eq!(track.break_key(), Some(2));
    assert!(clip.track(0).is_none());
}

#[test]
fn the_keys_are_evenly_spaced_over_the_damage() {
    let clip = &read_list(&list(1))[0];
    assert_eq!(clip.position(0.0), 0.0);
    assert_eq!(clip.position(0.5), 1.0);
    assert_eq!(clip.position(2.0), 2.0, "past the end is the end");
    let track = clip.track(1).unwrap();
    let mid = track.at(0.5);
    assert!((mid.translation[1] + 0.05).abs() < 1e-6 && (mid.scalar - 1.0).abs() < 1e-6, "{mid:?}");
    assert!((track.at(1.5).scalar - 0.5).abs() < 1e-6, "a scalar fades between its keys");
    let rotation = track.at(1.0).rotation;
    assert!((rotation.iter().map(|c| c * c).sum::<f32>() - 1.0).abs() < 1e-4, "still a unit quaternion");
}

#[test]
fn a_piece_without_a_scalar_breaks_by_a_big_step() {
    let mut track = read_list(&list(1))[0].tracks[0].clone();
    track.scalar = None;
    track.translation = vec![[0.0; 3], [0.0, -(BREAK_STEP_M - 0.01), 0.0], [0.0, -2.0, 0.0]];
    assert_eq!(track.break_key(), Some(2), "a small step is a dent, a big one a piece leaving");
    track.translation[2] = [0.0, -0.2, 0.0];
    assert_eq!(track.break_key(), None);
}

#[test]
fn a_clip_is_read_as_each_key_less_its_reference_key() {
    let mut block = list(1);
    for (key, y) in [0.0f32, -0.1, -1.0].into_iter().enumerate() {
        put(&mut block, 0x20 + 0x80 + 16 * key + 4, (y + 0.5).to_bits());
    }
    let track = read_list(&block)[0].tracks[0].clone();
    for (got, want) in track.translation.iter().zip([0.0f32, -0.1, -1.0]) {
        assert!((got[1] - want).abs() < 1e-6, "{got:?} is not {want}");
    }
}

#[test]
fn a_piece_that_is_never_whole_never_breaks() {
    let mut track = read_list(&list(1))[0].tracks[0].clone();
    track.scalar = Some(vec![0.0; 3]);
    assert_eq!(track.break_key(), None);
}

#[test]
fn a_list_in_another_format_or_cut_short_reads_as_none() {
    assert!(read_list(&list(0)).is_empty(), "only packed rotations are known");
    let whole = list(1);
    assert!(read_list(&whole[..0x40]).is_empty());
    assert!(read_list(&[]).is_empty());
}

#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
fn every_car_has_eight_damage_clips_of_21_keys_but_a_few() {
    let Some(dir) = std::env::var_os("CHAMELEON_GAME_DIR") else { return };
    let (mut cars, mut with_clips, mut pieces, mut regular, mut clip_count) = (0, 0, 0, 0, 0);
    for entry in std::fs::read_dir(Path::new(&dir).join("VEHICLES")).unwrap().flatten() {
        let file = entry.file_name().to_string_lossy().to_uppercase();
        if !(file.starts_with("VEH_") && file.ends_with("_HI.BNDL")) {
            continue;
        }
        let data = std::fs::read(entry.path()).unwrap();
        let bundle = Bundle::parse(&data).unwrap();
        cars += 1;
        let clips = damage_clips(&bundle);
        if clips.is_empty() {
            continue;
        }
        with_clips += 1;
        assert!(clips.len() == 8 || file.contains("122706") && clips.len() == 7, "{file}: {} clips", clips.len());
        for (n, clip) in clips.iter().enumerate() {
            assert_eq!(usize::from(clip.zone), n + 1, "{file}");
            regular += usize::from((clip.keys, clip.seconds) == (21, 6.0));
            clip_count += 1;
            assert!(clip.tracks.iter().all(|t| t.rotation.len() == clip.keys && t.bone < 64), "{file}");
            let whole = |s: &Vec<f32>| s[0] == 1.0 || s.iter().all(|&v| v == 0.0);
            assert!(clip.tracks.iter().all(|t| t.scalar.as_ref().is_none_or(whole)), "{file}: whole at no damage");
            assert!(clip.tracks.iter().all(|t| t.translation[0] == [0.0; 3]), "{file}: key 0 is the reference");
            pieces += clip.tracks.iter().filter(|t| t.break_key().is_some()).count();
        }
        let parts = hinged_parts(&bundle).unwrap();
        assert!(parts.iter().all(|p| p.targets.len() == p.thresholds.len()), "{file}");
    }
    assert!(cars >= 90 && with_clips >= cars - 2, "{with_clips} of {cars} cars have clips");
    assert!(regular + 3 >= clip_count, "{regular} of {clip_count} clips have 21 keys over 6 s: all but three");
    assert!(pieces > 500, "{pieces} breaking pieces in all");
}

#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
fn the_911_breaks_its_lamps_at_a_quarter_and_drops_its_bumpers_at_full_damage() {
    let Some(dir) = std::env::var_os("CHAMELEON_GAME_DIR") else { return };
    let data = std::fs::read(Path::new(&dir).join("VEHICLES/VEH_1085007_HI.BNDL")).unwrap();
    let clips = damage_clips(&Bundle::parse(&data).unwrap());
    let key = |zone: usize, bone: usize| clips[zone - 1].track(bone).and_then(|t| t.break_key());
    assert_eq!((key(1, 18), key(2, 34), key(3, 28), key(5, 4)), (Some(5), Some(8), Some(8), Some(12)));
    assert_eq!((key(1, 1), key(2, 7), key(2, 36)), (Some(20), Some(20), Some(20)));
    assert_eq!(key(1, 13), None, "the bonnet only tilts");
    let bonnet = clips[0].track(13).unwrap().at(20.0).rotation;
    assert!((bonnet[0] + 0.08).abs() < 0.01, "about 9 degrees up at the front: {bonnet:?}");
}
