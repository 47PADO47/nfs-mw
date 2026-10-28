use super::*;
use crate::Task;

fn put(block: &mut [u8], at: usize, value: f32) {
    block[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn emitter(op: u8, block: Vec<u8>) -> Emitter {
    Emitter { flags: 0, tasks: vec![Task { op, block }], stages: Default::default() }
}

#[test]
fn the_collision_mesh_spawner_reads_its_numbers() {
    let mut b = vec![0u8; 100];
    for (at, v) in [(0x00, 4.0), (0x04, 8.0), (0x0C, 10.0), (0x14, 10.0), (0x20, 10.0), (0x24, 100.0)] {
        put(&mut b, at, v);
    }
    for (at, v) in [(0x48, 0.5), (0x4C, 0.5), (0x50, 0.35)] {
        put(&mut b, at, v);
    }
    b[0x54..0x58].copy_from_slice(&0u32.to_le_bytes());
    b[0x58..0x5C].copy_from_slice(&200u32.to_le_bytes());
    b[0x5C..0x62].copy_from_slice(&[0, 0, 3, 3, 0, 2]);
    let mesh = emitter(EMITTER_COLLISION_MESH, b).collision_mesh().unwrap();
    assert_eq!(mesh.speed, [4.0, 8.0]);
    assert_eq!(mesh.angle, [[0.0, 10.0], [0.0, 10.0]]);
    assert_eq!((mesh.burst, mesh.rate), (200, 0));
    assert_eq!(mesh.params, [0, 0, 3, 3, 0]);
    assert_eq!(mesh.quality, 2);
    assert_eq!((mesh.radius, mesh.height, mesh.facing), (0.5, 0.5, 0.35));
}

#[test]
fn the_bounce_block_reads_restitution_friction_and_gravity() {
    let mut b = vec![0u8; 0x44];
    for (at, v) in [(0x1C, 0.8), (0x20, 0.9), (0x24, 0.01), (0x28, 0.05), (0x2C, -9.8), (0x30, 0.4)] {
        put(&mut b, at, v);
    }
    let bounce = emitter(UPDATE_POS_VEL_WITH_BOUNCE_PLANE, b).bounce().unwrap();
    assert_eq!(bounce.restitution, [0.8, 0.9]);
    assert_eq!(bounce.friction, [0.01, 0.05]);
    assert_eq!((bounce.gravity, bounce.drag), (-9.8, 0.4));
}

#[test]
fn the_streak_block_reads_its_length_numbers() {
    let mut b = vec![0u8; 0x50];
    for (at, v) in [(0x30, 0.0), (0x34, 100.0), (0x44, 1.0), (0x48, 0.125)] {
        put(&mut b, at, v);
    }
    b[0x4C] = 2;
    let streak = emitter(RENDER_SPRITE_VELOCITY, b).velocity_streak().unwrap();
    assert_eq!(streak.length, [0.0, 100.0]);
    assert_eq!((streak.length_per_speed, streak.opacity, streak.scale_param), (0.125, 1.0, 2));
}

#[test]
fn a_binding_is_a_response_curve_of_a_scaled_source() {
    let mut b = vec![0u8; 0x70];
    // Binding 2: the closing speed over 22.35, 0.153 at rest rising to 1 at 0.988 of the scale.
    b[0x18 + 20..0x18 + 30].copy_from_slice(&[39, 255, 255, 255, 255, 0, 252, 253, 254, 255]);
    put(&mut b, 0x40 + 8, 22.35);
    b[0x62] = 0x11;
    let bindings = emitter(UPDATE_BINDINGS, b).bindings().unwrap();
    let binding = bindings.bindings[2];
    assert_eq!(binding.source, Source::InputBOver);
    assert!((binding.response(0.0) - 0.153).abs() < 1e-3);
    assert!((binding.response(0.5) - (0.153 + 0.847 * 0.5 / 0.988)).abs() < 0.01);
    assert_eq!(binding.response(2.0), 1.0);
}

#[test]
fn the_input_names_follow_the_type_bytes() {
    let mut main = vec![0u8; 0x28];
    main[0x20..0x22].copy_from_slice(&2u16.to_le_bytes());
    main.extend_from_slice(&[3, 4]);
    main.extend_from_slice(b"Transform\0Velocity\0");
    assert_eq!(input_names(&main), ["Transform", "Velocity"]);
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn the_installed_collision_effects_decode_to_the_values_the_docs_give() {
    let root = std::path::PathBuf::from(std::env::var_os("MW2012_GAME_DIR").expect("MW2012_GAME_DIR"));
    let data = std::fs::read(root.join("GLOBALEFFECTS.BNDL")).unwrap();
    let bundle = chameleon_bundle::Bundle::parse(&data).unwrap();
    let ns = 1u64 << 56;
    let effect = |id: u64| Effect::parse(&bundle.resource_by_id(ns | id).unwrap().blocks[0]).unwrap();
    // The impact effect of the default surface: three spark streaks that leave the mesh in a burst.
    let hit = effect(0x14021C);
    let jet = &hit.emitters[1];
    let mesh = jet.collision_mesh().unwrap();
    assert_eq!((mesh.burst, mesh.rate, mesh.speed), (200, 0, [4.0, 8.0]));
    let bounce = jet.bounce().unwrap();
    assert!(
        (bounce.restitution[0] - 0.8).abs() < 1e-6
            && (bounce.restitution[1] - 0.9).abs() < 1e-6
            && bounce.gravity == -9.8
    );
    assert_eq!(jet.velocity_streak().unwrap().length_per_speed, 0.125);
    let strength = jet.bindings().unwrap().bindings[2];
    assert_eq!(strength.source, Source::InputBOver);
    assert!((strength.scale - 22.35).abs() < 1e-3 && (strength.curve[0].1 - 0.153).abs() < 1e-3);
    // The scrape effect: a rate for as long as the effect lives, gated by the sliding speed.
    let scrape = effect(0x140279);
    let embers = scrape.emitters[2].collision_mesh().unwrap();
    assert_eq!((embers.rate, embers.burst), (2000, 0));
    assert!(embers.window[1] > 1.0e30);
    let speed = scrape.emitters[2].bindings().unwrap().bindings[0];
    assert_eq!(speed.source, Source::Speed);
    assert!((speed.curve[0].0 - 0.3647).abs() < 1e-3 && (speed.curve[1].0 - 0.6863).abs() < 1e-3);
    let names = input_names(&bundle.resource_by_id(ns | 0x14021C).unwrap().blocks[0]);
    assert_eq!(names.len(), 10);
    assert_eq!(names[8], "ImpactClosingVelocity");
}
