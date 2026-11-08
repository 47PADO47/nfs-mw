use super::*;

fn arc(cube: ColourCube) -> Arc<ColourCube> {
    Arc::new(cube)
}

/// A cube that inverts every channel.
fn invert(size: u32) -> ColourCube {
    let identity = ColourCube::identity(size).unwrap();
    ColourCube::new(size, identity.rgb().iter().map(|b| 255 - b).collect()).unwrap()
}

#[test]
fn a_cube_needs_a_size_in_range_and_the_right_amount_of_data() {
    assert!(ColourCube::new(1, vec![0; 3]).is_none());
    assert!(ColourCube::new(65, vec![0; 65 * 65 * 65 * 3]).is_none());
    assert!(ColourCube::new(2, vec![0; 23]).is_none());
    assert!(ColourCube::new(2, vec![0; 24]).is_some());
    assert!(ColourCube::identity(1).is_none());
}

#[test]
fn the_identity_cube_has_every_entry_at_its_own_position() {
    for size in [2, 16, 17, 32] {
        let cube = ColourCube::identity(size).unwrap();
        assert!(cube.is_identity(0.0));
        assert_eq!(cube.entry(0, 0, 0), [0.0; 3]);
        assert_eq!(cube.entry(size - 1, size - 1, size - 1), [1.0; 3]);
        assert_eq!(cube.entry(size - 1, 0, 0), [1.0, 0.0, 0.0], "red varies along x");
        assert_eq!(cube.entry(0, size - 1, 0), [0.0, 1.0, 0.0]);
        assert_eq!(cube.entry(0, 0, size - 1), [0.0, 0.0, 1.0]);
    }
}

#[test]
fn the_identity_cube_leaves_colours_unchanged_to_a_255th() {
    for size in [16, 32] {
        let grade = ColourGrade::single(arc(ColourCube::identity(size).unwrap()), 1.0);
        for r in 0..=20 {
            for g in 0..=20 {
                for b in 0..=20 {
                    let rgb = [r, g, b].map(|i| i as f32 / 20.0);
                    let out = grade.apply(rgb);
                    for channel in 0..3 {
                        assert!((out[channel] - rgb[channel]).abs() <= 1.0 / 255.0, "size {size} {rgb:?} -> {out:?}");
                    }
                }
            }
        }
    }
}

#[test]
fn a_lookup_between_entries_is_trilinear() {
    // Entries with x = 1 carry red 200: the output follows the input's red linearly.
    let mut rgb = vec![0u8; 24];
    for entry in (0..8).filter(|e| e & 1 == 1) {
        rgb[entry * 3] = 200;
    }
    let cube = ColourCube::new(2, rgb).unwrap();
    for t in [0.0, 0.25, 0.5, 1.0] {
        let out = cube.sample([t, 0.3, 0.9]);
        assert!((out[0] - t * 200.0 / 255.0).abs() < 1e-6, "{t} {out:?}");
        assert_eq!((out[1], out[2]), (0.0, 0.0));
    }
}

#[test]
fn a_lookup_clamps_and_survives_non_finite_input() {
    let cube = invert(16);
    assert_eq!(cube.sample([-1.0, 2.0, f32::NAN]), [1.0, 0.0, 1.0]);
}

#[test]
fn the_strength_goes_from_the_original_to_the_graded_colour() {
    let cube = arc(invert(16));
    let rgb = [0.2, 0.5, 0.9];
    let full = ColourGrade::single(cube.clone(), 1.0).apply(rgb);
    let half = ColourGrade::single(cube.clone(), 0.5).apply(rgb);
    let none = ColourGrade::single(cube, 0.0).apply(rgb);
    assert_eq!(none, rgb);
    for channel in 0..3 {
        assert!((full[channel] - (1.0 - rgb[channel])).abs() < 0.01, "{full:?}");
        assert!((half[channel] - 0.5 * (rgb[channel] + full[channel])).abs() < 1e-6);
    }
    assert!(ColourGrade::single(arc(invert(16)), 0.0).is_neutral());
    assert!(!ColourGrade::single(arc(invert(16)), 0.1).is_neutral());
}

#[test]
fn two_cubes_blend_by_the_mix() {
    let identity = arc(ColourCube::identity(16).unwrap());
    let inverted = arc(invert(16));
    let grade = |mix| ColourGrade { from: identity.clone(), to: Some(inverted.clone()), mix, strength: 1.0 };
    let rgb = [0.1, 0.4, 0.8];
    assert_eq!(grade(0.0).apply(rgb), ColourGrade::single(identity.clone(), 1.0).apply(rgb));
    let half = grade(0.5).apply(rgb);
    let (a, b) = (grade(0.0).apply(rgb), grade(1.0).apply(rgb));
    for channel in 0..3 {
        assert!((half[channel] - 0.5 * (a[channel] + b[channel])).abs() < 1e-6);
    }
    // The cubes need not have the same size.
    let small = arc(ColourCube::identity(8).unwrap());
    let mixed = ColourGrade { from: small, to: Some(inverted), mix: 1.0, strength: 1.0 };
    assert!((mixed.apply(rgb)[0] - 0.9).abs() < 0.01);
}

#[test]
fn a_value_above_one_keeps_its_excess() {
    let grade = ColourGrade::single(arc(invert(16)), 1.0);
    let out = grade.apply([1.5, 1.0, 0.0]);
    assert!((out[0] - 0.5).abs() < 1e-6, "the graded 1.0 is 0, plus the excess 0.5: {out:?}");
    assert!(out[1].abs() < 1e-6);
    assert!((out[2] - 1.0).abs() < 1e-6);
}

#[test]
fn equal_cubes_have_equal_ids_and_different_cubes_differ() {
    let a = ColourCube::identity(16).unwrap();
    let b = ColourCube::identity(16).unwrap();
    assert_eq!(a.id(), b.id());
    assert_ne!(a.id(), invert(16).id());
    assert_ne!(a.id(), ColourCube::identity(8).unwrap().id());
}

#[test]
fn a_grade_is_sanitised() {
    let cube = arc(ColourCube::identity(4).unwrap());
    let grade = ColourGrade { from: cube.clone(), to: Some(cube.clone()), mix: f32::NAN, strength: 3.0 };
    let clean = grade.sanitized();
    assert_eq!((clean.mix, clean.strength), (0.0, 1.0));
    assert!(clean.to.is_none(), "no blend, no second cube to look up");
    let grade = ColourGrade { from: cube, to: None, mix: 0.7, strength: -1.0 };
    assert_eq!(grade.sanitized().mix, 0.0);
    assert!(grade.is_neutral());
}

#[test]
fn the_rgba_copy_has_a_full_alpha() {
    let cube = ColourCube::identity(2).unwrap();
    let rgba = cube.rgba();
    assert_eq!(rgba.len(), 8 * 4);
    assert!(rgba.chunks_exact(4).all(|px| px[3] == 255));
    assert_eq!(&rgba[4..8], &[255, 0, 0, 255], "the second entry is red");
}

#[test]
fn a_blend_of_cubes_is_their_weighted_mean_at_every_entry() {
    let identity = ColourCube::identity(16).unwrap();
    let inverted = invert(16);
    let half = ColourCube::blend(&[(&identity, 1.0), (&inverted, 1.0)]).unwrap();
    assert_eq!(half.size(), 16);
    for entry in half.rgb().chunks_exact(3) {
        assert!(entry.iter().all(|b| (127..=128).contains(b)), "{entry:?}");
    }
    let mostly = ColourCube::blend(&[(&identity, 3.0), (&inverted, 1.0)]).unwrap();
    assert!((mostly.entry(15, 0, 0)[0] - 0.75).abs() < 0.01, "{:?}", mostly.entry(15, 0, 0));
    // One part is that cube, a part of weight 0 does not count, and the first part sets the size.
    assert!(ColourCube::blend(&[(&identity, 1.0), (&inverted, 0.0)]).unwrap().is_identity(0.0));
    let small = ColourCube::identity(8).unwrap();
    assert_eq!(ColourCube::blend(&[(&small, 1.0), (&identity, 1.0)]).unwrap().size(), 8);
    assert!(ColourCube::blend(&[]).is_none());
    assert!(ColourCube::blend(&[(&identity, 0.0)]).is_none());
    assert!(ColourCube::blend(&[(&identity, f32::NAN)]).is_none());
}
