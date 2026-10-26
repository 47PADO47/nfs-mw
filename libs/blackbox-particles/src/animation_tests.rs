use crate::{Animation, Emitter, EmitterSpec, Frame};

#[test]
fn zero_fps_random_atlases_choose_one_cell_and_hold_it() {
    let spec = EmitterSpec {
        animation: Animation { grid: 4, fps: 0, random_start: true },
        rate: 600.0,
        life: 10.0,
        ..Default::default()
    };
    let mut emitter = Emitter::new(spec, 7);
    emitter.step(0.02, &Frame::default());
    emitter.set_enabled(false);
    let before: Vec<_> = emitter.sprites().map(|s| s.uv).collect();
    assert!(before.len() > 1);
    assert!(before.windows(2).any(|pair| pair[0] != pair[1]));
    for uv in &before {
        assert!((uv[1][0] - uv[0][0] - 0.25).abs() < 2.0 / 65535.0);
        assert!((uv[1][1] - uv[0][1] - 0.25).abs() < 2.0 / 65535.0);
    }
    emitter.step(0.02, &Frame::default());
    assert_eq!(before, emitter.sprites().map(|s| s.uv).collect::<Vec<_>>());
}

#[test]
fn quantized_animation_preserves_pc_frame_boundaries_and_wrap() {
    let animation = Animation { grid: 2, fps: 4, random_start: false };
    let (mut phase, mut cell) = (0, 0);
    animation.advance(&mut phase, &mut cell, 0.25);
    assert_eq!((phase, cell), (16383, 0));
    animation.advance(&mut phase, &mut cell, 0.25);
    assert_eq!((phase, cell), (32766, 1));
    phase = 60000;
    animation.advance(&mut phase, &mut cell, 0.25);
    assert_eq!((phase, cell), (10848, 0));
}

#[test]
fn a_five_by_five_flipbook_has_twenty_five_cells_of_a_fifth() {
    let animation = Animation { grid: 5, fps: 25, random_start: false };
    let [min, max] = animation.uv(7);
    assert!((min[0] - 0.4).abs() < 1e-3 && (min[1] - 0.2).abs() < 1e-3, "cell 7 is column 2, row 1: {min:?}");
    assert!((max[0] - 0.6).abs() < 1e-3 && (max[1] - 0.4).abs() < 1e-3, "{max:?}");
    let mut phase = 0;
    let mut cell = 0;
    for _ in 0..100 {
        animation.advance(&mut phase, &mut cell, 0.01);
    }
    assert!(cell < 25, "the cell never leaves the sheet");
}
