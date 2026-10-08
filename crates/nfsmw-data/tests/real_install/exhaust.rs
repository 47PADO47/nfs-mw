//! The exhaust effects (docs/specs/exhaust-flames.md) read from the install.

use blackbox_attrib::Database;
use blackbox_hash::bstring_hash;
use glam::Vec3;
use nfsmw_data::car::exhaust::{ADDITIVE_BLEND, ExhaustFx, particle_textures, pipe_direction};
use nfsmw_data::car::{LoadOptions, load};

use crate::install;

fn effects(car: &str) -> Option<ExhaustFx> {
    let dir = install()?;
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).expect("attributes.bin");
    let model = load(&dir, car, &LoadOptions { lod: 'A', all_parts: false, preset: None }).unwrap();
    ExhaustFx::read(&db, &model)
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_m3_gtr_has_two_pipes_shooting_backwards_and_the_racer_blow_off() {
    let Some(fx) = effects("BMWM3GTR") else { return };
    assert_eq!(fx.pipes.len(), 2);
    for pipe in &fx.pipes {
        assert!((pipe_direction(pipe) - Vec3::NEG_X).length() < 1e-3);
        let at = pipe.w_axis.truncate();
        assert!(at.x < -1.8 && at.y.abs() < 0.5 && at.z > 0.0 && at.z < 0.5, "{at}");
    }
    assert_eq!((fx.shift_speed, fx.shift_angle), (8.0, 2.25));
    assert_eq!(fx.engine_upgrades, 0, "no upgrade levels: the blow-off is always on");
    assert_eq!(fx.nitrous.len(), 2);
    let fire = &fx.nitrous[0].spec;
    assert_eq!((fire.rate, fire.life, fire.speed), (500.0, 0.075, 6.0));
    assert_eq!(fire.size, [0.35, 0.45, 0.1, 0.0]);
    assert!((fire.keys[1] - 0.6).abs() < 1e-6);
    assert_eq!(fire.colors[0], [0, 40, 80, 60]);
    assert!(fire.live_motion);
    let glow = &fx.nitrous[1].spec;
    assert_eq!((glow.rate, glow.speed, glow.drag, glow.gravity), (50.0, 16.0, 0.75, -1.0));
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_fxx_evo_blow_off_is_shorter_and_an_upgradable_car_counts_its_levels() {
    let Some(fx) = effects("FXXEVO") else { return };
    assert_eq!((fx.shift_speed, fx.shift_angle), (8.0, 0.5));
    assert_eq!(fx.pipes.len(), 4);
    let Some(fx) = effects("CORVETTE") else { return };
    assert_eq!(fx.engine_upgrades, 2);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_nitrous_textures_are_in_the_particle_pack_with_their_blend() {
    let Some(dir) = install() else { return };
    let fire = bstring_hash("FX_FIRE02_ADDITIVE");
    let glow = bstring_hash("FX_SMK06_BLEND");
    let found = particle_textures(&dir, &[fire, glow]).unwrap();
    assert_eq!(found.len(), 2);
    let blend = |hash: u32| found.iter().find(|t| t.name_hash == hash).map(|t| t.alpha_blend);
    assert_eq!(blend(fire), Some(ADDITIVE_BLEND));
    assert_eq!(blend(glow), Some(1));
    let fx = effects("BMWM3GTR").unwrap();
    assert_eq!([fx.nitrous[0].texture, fx.nitrous[1].texture], [fire, glow]);
}
