use super::*;
use crate::ops;

/// An effect of one emitter with one `EmitterWheel` task, the block written with the layout of the file.
fn effect_with(block: &[u8]) -> Vec<u8> {
    let mut b = vec![0u8; 0x300];
    b[0..4].copy_from_slice(&crate::MAGIC.to_le_bytes());
    b[4..6].copy_from_slice(&1u16.to_le_bytes());
    b[0x12..0x14].copy_from_slice(&0x14u16.to_le_bytes());
    b[0x14..0x16].copy_from_slice(&0x40u16.to_le_bytes());
    let d = 0x40;
    b[d + 4..d + 6].copy_from_slice(&0x20u16.to_le_bytes());
    b[d + 6..d + 8].copy_from_slice(&0x30u16.to_le_bytes());
    b[d + 0x0C..d + 0x0E].copy_from_slice(&0x0101u16.to_le_bytes());
    b[d + 0x20..d + 0x22].copy_from_slice(&0x40u16.to_le_bytes());
    b[d + 0x22] = ops::EMITTER_WHEEL;
    b[d + 0x40..d + 0x40 + block.len()].copy_from_slice(block);
    // The block runs to the end of the resource, as the last one of an emitter does.
    b.truncate(d + 0x40 + block.len());
    b
}

fn block(rate_per_speed: f32, alt: f32) -> Vec<u8> {
    let mut block = vec![0u8; WHEEL_BLOCK];
    let mut put = |at: usize, v: f32| block[at..at + 4].copy_from_slice(&v.to_le_bytes());
    put(0x00, 0.0);
    put(0x04, f32::MAX);
    put(0x08, 10.0);
    put(0x0C, 150.0);
    put(0x10, 0.125);
    put(0x14, 0.5);
    put(0x28, 0.0);
    put(0x2C, 360.0);
    put(0x30, 0.9);
    put(0x34, 1.2);
    put(0x38, -1.0);
    put(0x3C, 1.0);
    put(0x40, 0.01);
    put(0x44, 0.04);
    put(0x6C, alt);
    put(0x70, rate_per_speed);
    put(0x74, 18.0);
    block[0x78] = 1;
    block
}

#[test]
fn the_wheel_spawner_is_read_from_its_block() {
    let effect = crate::Effect::parse(&effect_with(&block(2.0, 0.0001))).unwrap();
    let wheel = effect.emitters[0].wheel().unwrap();
    assert_eq!((wheel.rate, wheel.rate_per_speed), (18.0, 2.0));
    assert_eq!(wheel.fade_distance, [10.0, 150.0]);
    assert_eq!((wheel.speed, wheel.angle), ([0.125, 0.5], [0.0, 360.0]));
    assert_eq!((wheel.radius_scale, wheel.width_scale, wheel.height), ([0.9, 1.2], [-1.0, 1.0], [0.01, 0.04]));
    assert_eq!(wheel.params, [1, 0, 0, 0]);
    assert_eq!(wheel.gate[0], 0.0);
}

#[test]
fn a_rate_per_speed_of_zero_falls_back_to_the_other_field() {
    let effect = crate::Effect::parse(&effect_with(&block(0.0, 0.5))).unwrap();
    assert_eq!(effect.emitters[0].wheel().unwrap().rate_per_speed, 0.5);
}

#[test]
fn an_emitter_without_the_operation_or_with_a_short_block_has_no_wheel() {
    let short = crate::Effect::parse(&effect_with(&block(2.0, 0.0)[..0x40])).unwrap();
    assert!(short.emitters[0].wheel().is_none(), "the block ends before the rate");
}

#[test]
fn the_bounce_plane_drag_is_read_from_its_block() {
    let mut plane = vec![0u8; BOUNCE_BLOCK];
    plane[0x34..0x38].copy_from_slice(&0.25f32.to_le_bytes());
    let mut bytes = effect_with(&plane);
    bytes[0x40 + 0x22] = ops::UPDATE_POS_VEL_WITH_BOUNCE_PLANE;
    let effect = crate::Effect::parse(&bytes).unwrap();
    assert_eq!(effect.emitters[0].bounce_drag(), Some(0.25));
    assert!(effect.emitters[0].wheel().is_none(), "another operation: no wheel spawner");
    let without = crate::Effect::parse(&effect_with(&block(2.0, 0.0))).unwrap();
    assert_eq!(without.emitters[0].bounce_drag(), None);
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn the_tyre_smoke_of_the_install_spawns_around_the_wheel() {
    let root = std::path::PathBuf::from(std::env::var_os("MW2012_GAME_DIR").expect("MW2012_GAME_DIR"));
    let data = std::fs::read(root.join("GLOBALEFFECTS.BNDL")).unwrap();
    let bundle = chameleon_bundle::Bundle::parse(&data).unwrap();
    let resource = bundle.resource_by_id((1u64 << 56) | 0x21B507).unwrap();
    let effect = crate::Effect::parse(&resource.blocks[0]).unwrap();
    assert_eq!(effect.emitters.len(), 7);
    let first = effect.emitters[0].wheel().unwrap();
    assert_eq!((first.rate, first.rate_per_speed, first.speed), (18.0, 2.0, [0.125, 0.5]));
    assert!(effect.emitters.iter().all(|e| e.wheel().is_some()));
    let drag = |n: usize| effect.emitters[n].bounce_drag().unwrap();
    assert!((drag(0) - 0.025).abs() < 1e-6, "the first cloud drifts on");
    assert!((drag(2) - 0.25).abs() < 1e-6, "the big one stays where it is made");
}
