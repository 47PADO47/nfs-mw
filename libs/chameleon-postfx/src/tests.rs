use super::*;

fn put(block: &mut [u8], offset: usize, values: &[f32]) {
    for (index, value) in values.iter().enumerate() {
        block[offset + index * 4..offset + index * 4 + 4].copy_from_slice(&value.to_le_bytes());
    }
}

fn keyframe_block() -> Vec<u8> {
    let mut block = vec![0u8; KEYFRAME_LEN];
    put(&mut block, 0x20, &[0.526, 0.51]);
    put(&mut block, 0x30, &[0.459, 0.522]);
    put(&mut block, 0x40, &[1.0; 4]);
    put(&mut block, 0x50, &[0.037, 0.114, 0.089, 1.0]);
    put(&mut block, 0x64, &[0.0, 0.0, 1.0, 0.0, 0.258]);
    put(&mut block, 0x90, &[0.597, 0.831, 1.0, 1.0]);
    put(&mut block, 0xA8, &[0.2]);
    put(&mut block, 0xBC, &[1.35]);
    put(&mut block, 0xE4, &[1.0, 0.1, 1.0]);
    block
}

#[test]
fn a_keyframe_is_read_by_offset() {
    let key = Keyframe::parse(&keyframe_block()).unwrap();
    assert_eq!(key.vignette.b, [0.459, 0.522]);
    assert_eq!(key.vignette.outer, [0.037, 0.114, 0.089, 1.0]);
    assert_eq!(key.vignette.tail[4], 0.258);
    assert_eq!((key.bloom.colour[2], key.bloom.a, key.bloom.b), (1.0, 0.2, 1.35));
    assert_eq!(key.general, [1.0, 0.1, 1.0]);
}

#[test]
fn a_short_block_or_a_number_that_is_not_finite_is_refused() {
    assert!(Keyframe::parse(&[0u8; 100]).is_none());
    let mut block = keyframe_block();
    put(&mut block, 0x50, &[f32::NAN]);
    assert!(Keyframe::parse(&block).is_none());
}

#[test]
fn a_state_reads_its_expression_and_blend_time() {
    let mut block = vec![0u8; 480];
    put(&mut block, 0xE8, &[2.0, 1.0]);
    let text = b"Reporter.NitrousState.UsingNitrous\0\0Reporter.NitrousState.UsingNitrous\0";
    block[0xF8..0xF8 + text.len()].copy_from_slice(text);
    let state = State::parse(&block).unwrap();
    assert_eq!((state.blend_seconds, state.weight), (2.0, 1.0));
    assert!(state.is_bound_to("NitrousState.UsingNitrous"));
    assert!(!state.is_bound_to("InOverdrive"));
}

#[test]
fn the_default_state_has_no_expression() {
    let mut block = vec![0u8; 416];
    put(&mut block, 0xE8, &[0.0, 1.0]);
    block[0x140] = b')';
    let state = State::parse(&block).unwrap();
    assert_eq!(state.expression, None);
    assert!(!state.is_bound_to("anything"));
}

fn cube_with(mut entry: impl FnMut(usize) -> [u8; 3]) -> Vec<u8> {
    let mut block = vec![0u8; CUBE_LEN];
    block[0] = 16;
    block[4] = 16;
    for step in 0..CUBE_SIZE {
        let at = 16 + step * (1 + CUBE_SIZE + CUBE_SIZE * CUBE_SIZE) * 3;
        block[at..at + 3].copy_from_slice(&entry(step));
    }
    block
}

fn identity(step: usize) -> [u8; 3] {
    let v = (step as f32 / 15.0 * 255.0).round() as u8;
    [v, v, v]
}

#[test]
fn the_identity_cube_is_neutral() {
    let cube = ColourCube::parse(&cube_with(identity)).unwrap();
    assert!(cube.is_identity());
    let grade = cube.grade();
    assert!((grade.contrast - 1.0).abs() < 0.01, "{grade:?}");
    assert!(grade.tint.iter().all(|t| (t - 1.0).abs() < 0.01), "{grade:?}");
}

#[test]
fn a_wrong_size_is_not_a_cube() {
    assert!(ColourCube::parse(&[0u8; 100]).is_none());
}

#[test]
fn a_warm_cube_gives_a_tint_with_red_up_and_blue_down() {
    // Stored blue, green, red: the red channel is brighter than the identity, the blue one darker.
    let cube = ColourCube::parse(&cube_with(|step| {
        let v = step as f32 / 15.0;
        let byte = |x: f32| (x.clamp(0.0, 1.0) * 255.0) as u8;
        [byte(v * 0.9), byte(v * 1.1), byte(v * 1.2)]
    }))
    .unwrap();
    assert!(!cube.is_identity());
    let grade = cube.grade();
    assert!((grade.contrast - 1.0).abs() < 0.05, "{grade:?}");
    assert!(grade.tint[0] > grade.tint[1] && grade.tint[1] > grade.tint[2], "{grade:?}");
    assert!((grade.tint[0] - 1.2).abs() < 0.03 && (grade.tint[2] - 0.9).abs() < 0.03, "{grade:?}");
}

#[test]
fn a_cube_with_pushed_apart_greys_has_a_contrast_above_one() {
    let cube = ColourCube::parse(&cube_with(|step| {
        let v = (step as f32 / 15.0 - 0.5) * 1.5 + 0.5;
        let byte = (v.clamp(0.0, 1.0) * 255.0) as u8;
        [byte; 3]
    }))
    .unwrap();
    let grade = cube.grade();
    assert!((grade.contrast - 1.5).abs() < 0.05, "{grade:?}");
    assert!(grade.tint.iter().all(|t| (t - 1.0).abs() < 0.05), "{grade:?}");
}
