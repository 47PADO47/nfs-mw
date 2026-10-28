use chameleon_vehicle::{Bone, BoneTrack, DamageClip, Skeleton};

use super::*;

const KEYS: usize = 21;

fn clip(zone: u16, tracks: Vec<BoneTrack>) -> DamageClip {
    DamageClip { zone, keys: KEYS, seconds: 6.0, tracks }
}

/// A bone that turns about the x axis, `degrees` at the last key, in proportion to the key.
fn turning(bone: usize, degrees: f32) -> BoneTrack {
    let rotation = (0..KEYS)
        .map(|key| {
            let half = (degrees * key as f32 / (KEYS - 1) as f32).to_radians() / 2.0;
            [half.sin(), 0.0, 0.0, half.cos()]
        })
        .collect();
    BoneTrack { bone, translation: vec![[0.0; 3]; KEYS], rotation, scalar: None }
}

/// A bone that is whole until key `gone` and broken from there on.
fn breaking(bone: usize, gone: usize) -> BoneTrack {
    let scalar = (0..KEYS).map(|key| if key < gone { 1.0 } else { 0.0 }).collect();
    BoneTrack {
        bone,
        translation: vec![[0.0; 3]; KEYS],
        rotation: vec![[0.0, 0.0, 0.0, 1.0]; KEYS],
        scalar: Some(scalar),
    }
}

fn skeleton(parents: &[Option<usize>]) -> Skeleton {
    Skeleton { bones: parents.iter().map(|&parent| Bone { hash: 0, parent, position: [0.0, 1.0, 2.0] }).collect() }
}

#[test]
fn a_part_turns_by_its_clip_at_the_three_panel_levels() {
    let clips = [clip(1, vec![turning(3, 10.0)])];
    let swing = swing_of(&clips, 3, 1).unwrap();
    assert!((swing.axis[0] - 1.0).abs() < 1e-4, "about the x axis: {:?}", swing.axis);
    for (got, want) in swing.angles.iter().zip([10.0 / 3.0, 20.0 / 3.0, 10.0]) {
        assert!((got - want).abs() < 0.05, "{got} is not {want}");
    }
    let back = swing_of(&[clip(1, vec![turning(3, -10.0)])], 3, 1).unwrap();
    assert!(back.angles[2] < -9.9 || back.axis[0] < -0.99, "a turn the other way keeps its sign: {back:?}");
}

#[test]
fn a_part_that_hardly_turns_has_no_swing() {
    assert!(swing_of(&[clip(1, vec![turning(3, 0.4)])], 3, 1).is_none());
    assert!(swing_of(&[clip(1, vec![turning(3, 5.0)])], 4, 1).is_none(), "another bone");
}

#[test]
fn a_piece_breaks_off_where_its_scalar_drops() {
    let clips = [clip(1, vec![breaking(5, 5), turning(6, 4.0)]), clip(2, vec![breaking(5, 2)])];
    assert_eq!(release_of(&clips, 5, 1), Some(Release::At(0.25)));
    assert_eq!(release_of(&clips, 5, 2), Some(Release::At(0.1)), "the zone's own clip wins");
    assert_eq!(release_of(&clips, 6, 1), Some(Release::Never), "it is moved but never goes");
    assert_eq!(release_of(&clips, 7, 1), None, "no clip moves it");
}

#[test]
fn a_piece_that_leaves_by_a_jump_breaks_at_the_jump() {
    let mut track = turning(1, 0.0);
    track.translation[KEYS - 1] = [0.0, -0.97, 0.0];
    assert_eq!(release_of(&[clip(1, vec![track])], 1, 1), Some(Release::At(1.0)));
}

#[test]
fn a_piece_turns_with_the_part_it_sits_on() {
    // Bone 1 is a part of the car's damage object; bone 2 hangs from it and breaks off at 0.4.
    let skeleton = skeleton(&[None, Some(0), Some(1)]);
    let clips = [clip(1, vec![turning(1, 9.0), breaking(2, 8)])];
    let part = part_damage(&clips, &skeleton, &[1], 1, 1).unwrap();
    let piece = part_damage(&clips, &skeleton, &[1], 2, 1).unwrap();
    assert_eq!(piece.swing, part.swing, "the piece moves with its carrier");
    assert_eq!((part.release, piece.release), (Release::Never, Release::At(0.4)));
    let loose = part_damage(&clips, &skeleton, &[], 2, 1).unwrap();
    assert!(loose.swing.is_none(), "nothing above it turns");
}

#[test]
fn bones_that_break_become_parts_in_bone_order_within_the_room() {
    let skeleton = skeleton(&[None, Some(0), Some(0), Some(0), Some(0)]);
    let clips = [
        clip(1, vec![breaking(4, 5), breaking(2, 8), turning(3, 5.0), breaking(1, 2)]),
        clip(5, vec![breaking(2, 12)]),
    ];
    let found = pieces(&clips, &skeleton, &[1], 10);
    let names: Vec<_> = found.iter().map(|part| (part.name.as_str(), part.zone)).collect();
    assert_eq!(
        names,
        [("Piece2", 1), ("Piece4", 1)],
        "bone 1 is a part already; a bone that only turns is not a piece"
    );
    assert_eq!(pieces(&clips, &skeleton, &[1], 1).len(), 1);
    assert!(pieces(&clips, &skeleton, &[1], 0).is_empty());
    assert_eq!(found[0].centre, [0.0, 1.0, 2.0]);
}
