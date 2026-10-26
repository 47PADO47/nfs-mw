use super::*;
use crate::ops;

fn put_u16(bytes: &mut [u8], at: usize, value: u16) {
    bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_f32(bytes: &mut [u8], at: usize, value: f32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

/// An effect of one emitter with two tasks, `lifetime` and a radius curve, written with the layout of the format.
fn synthetic() -> Vec<u8> {
    let mut b = vec![0u8; 0x200];
    b[0..4].copy_from_slice(&MAGIC.to_le_bytes());
    put_u16(&mut b, 4, 1);
    put_u16(&mut b, 0x12, 0x14);
    put_u16(&mut b, 0x14, 0x40);
    let d = 0x40;
    put_u16(&mut b, d + 4, 0x20);
    put_u16(&mut b, d + 6, 0x90);
    put_u16(&mut b, d + 0x0C, 0x01);
    b[d + 0x0D] = 2;
    // The task table: two offsets, two operations, two second bytes.
    put_u16(&mut b, d + 0x20, 0x30);
    put_u16(&mut b, d + 0x22, 0x60);
    b[d + 0x24] = ops::INITIALISE_AGE_AND_LIFETIME;
    b[d + 0x25] = ops::UPDATE_PARAMETRIC_RADIUS;
    // Lifetime: 0.25 to 0.5 seconds, floor 0.001, channel 3.
    put_f32(&mut b, d + 0x30 + 0x0C, 0.25);
    put_f32(&mut b, d + 0x30 + 0x10, 0.5);
    put_f32(&mut b, d + 0x30 + 0x14, 0.001);
    b[d + 0x30 + 0x18] = 3;
    // Radius curve: peak 0.25, base 0.45, rise 1, fall 8.
    for (at, v) in [(0x14, 0.25), (0x18, 0.45), (0x1C, 1.0), (0x20, 8.0), (0x24, 0.001)] {
        put_f32(&mut b, d + 0x60 + at, v);
    }
    // The stage table: stage 0 lists one pair, the others are empty.
    put_u16(&mut b, d + 0x90, 0x98);
    b[d + 0x98..d + 0x9E].copy_from_slice(&[1, 0, 0, 0, 1, 2]);
    b
}

#[test]
fn the_structure_of_an_effect_is_read_task_by_task() {
    let effect = Effect::parse(&synthetic()).unwrap();
    assert_eq!(effect.emitters.len(), 1);
    let emitter = &effect.emitters[0];
    assert_eq!(emitter.flags, 0x0201, "the second byte of the flag word is the task count");
    assert_eq!(emitter.task_names(), ["InitialiseAgeAndLifetime", "UpdateParametricRadius"]);
    assert_eq!(emitter.render_op(), Some(ops::UPDATE_PARAMETRIC_RADIUS));
    assert_eq!(emitter.stages[0], Some(Stage { groups: (1, 0), pairs: vec![(1, 2)] }));
    assert!(emitter.stages[1].is_none());
}

#[test]
fn the_lifetime_and_the_radius_curve_are_read_from_their_blocks() {
    let effect = Effect::parse(&synthetic()).unwrap();
    let emitter = &effect.emitters[0];
    let life = emitter.lifetime().unwrap();
    assert_eq!((life.min, life.max, life.scale_floor, life.channel), (0.25, 0.5, 0.001, 3));
    let curve = emitter.radius_curve().unwrap();
    assert_eq!((curve.peak, curve.base, curve.rise, curve.fall), (0.25, 0.45, 1.0, 8.0));
    assert!((curve.at(0.0) - (0.45 + 8.0 * 0.25)).abs() < 1e-6, "at birth lo = 0 and hi = the peak");
    assert!((curve.at(1.0) - (0.25 + 0.45 + 8.0)).abs() < 1e-6, "at the end lo = the peak and hi = 1");
    assert!(
        emitter.cone().is_none() && emitter.gradient().is_none(),
        "an operation the emitter lacks has no reader result"
    );
}

#[test]
fn a_bad_resource_is_refused() {
    assert_eq!(Effect::parse(&[0; 8]), Err(Error::BadMagic(0)));
    assert_eq!(Effect::parse(&[1, 2]), Err(Error::Truncated("header")));
    let mut cut = synthetic();
    cut.truncate(0x50);
    assert!(matches!(Effect::parse(&cut), Err(Error::Truncated(_))));
}

fn gradient(a: [f32; 4], b: [f32; 4]) -> decode::Gradient {
    decode::Gradient {
        colours: [[0.5, 0.25, 1.0, 1.0], [0.1, 0.1, 0.0, 3.0], [0.0; 4], [0.0; 4], [0.0; 4]],
        a,
        b,
        colour_scale: 1.0,
        alpha_scale: 1.0,
        alpha_y: [0.0, 1.0, 0.5, 0.25, 0.0],
        alpha_x: [0.0, 0.1, 0.4, 0.7, 1.0],
        colour_override: false,
    }
}

#[test]
fn the_colour_gradient_ramps_its_first_stop_in_and_leaves_the_others_off() {
    // The glow's shape: weight 0 follows t, the others are switched off by a huge slope and offset.
    let g = gradient([1.0, 99_864.0, 99_864.0, 99_864.0], [0.0, -99_864.0, -99_865.0, -99_866.0]);
    assert_eq!(g.colour_at(0.0), [0.5, 0.25, 1.0, 1.0]);
    let half = g.colour_at(0.5);
    assert!((half[0] - 0.55).abs() < 1e-5 && (half[3] - 2.5).abs() < 1e-5, "{half:?}");
}

#[test]
fn the_alpha_curve_is_piecewise_linear_through_five_points_and_wraps_to_zero() {
    let g = gradient([0.0; 4], [0.0; 4]);
    assert_eq!(g.alpha_at(0.0), 0.0);
    assert!((g.alpha_at(0.05) - 0.5).abs() < 1e-6);
    assert!((g.alpha_at(0.1) - 1.0).abs() < 1e-6);
    assert!((g.alpha_at(0.25) - 0.75).abs() < 1e-6);
    assert!((g.alpha_at(0.85) - 0.125).abs() < 1e-6);
    assert_eq!(g.alpha_at(1.0), 0.0, "the curve restarts at 1, and a particle is dead by then");
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn the_real_effects_decode_to_the_values_the_docs_give() {
    let root = std::path::PathBuf::from(std::env::var_os("MW2012_GAME_DIR").expect("MW2012_GAME_DIR"));
    let data = std::fs::read(root.join("GLOBALEFFECTS.BNDL")).unwrap();
    let bundle = chameleon_bundle::Bundle::parse(&data).unwrap();
    let effect = |id: u64| {
        let resource = bundle.resource_by_id(id).unwrap();
        Effect::parse(&resource.blocks[0]).unwrap()
    };
    let namespace = 1u64 << 56;
    // The glow of the burn.
    let glow = effect(namespace | 0x22030B);
    let e = &glow.emitters[0];
    assert_eq!(e.render_op(), Some(ops::RENDER_SPRITE));
    let life = e.lifetime().unwrap();
    assert_eq!((life.min, life.max), (0.25, 0.5));
    let cone = e.cone().unwrap();
    assert_eq!((cone.base_rate, cone.rate_per_speed, cone.velocity), (5.0, 1.0, [0.0; 3]));
    let g = e.gradient().unwrap();
    assert!((g.colours[0][2] - 0.863).abs() < 0.01 && g.colour_override);
    assert!((g.alpha_at(0.286) - 0.85).abs() < 0.01);
    // The flames of the pop-off: a half second burst of 100 a second that lives a tenth to a fifth of a second and rises.
    let pop = effect(namespace | 0x24F2D);
    let flame = pop.emitters.iter().find(|e| e.flipbook() && e.cone().is_some_and(|c| c.window[1] == 0.5)).unwrap();
    let life = flame.lifetime().unwrap();
    assert!((life.min - 0.1).abs() < 1e-6 && (life.max - 0.2).abs() < 1e-6);
    assert_eq!(flame.cone().unwrap().base_rate, 100.0);
    assert_eq!(flame.motion().unwrap().lift, 10.0);
    for emitters in [&glow.emitters, &pop.emitters, &effect(namespace | 0x16205B).emitters] {
        for e in emitters.iter() {
            assert!(e.task_names().iter().all(|n| *n != "?"), "every operation id is in the table");
        }
    }
}
