//! Exercise stock/restored isolation with real linkage profiles and synthetic contacts.

use super::super::drive::ContactKind;
use super::*;
use crate::settings::SparkStyle;
use blackbox_attrib::{Database, vlt_hash};

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn vehicle_effects_pc_style_isolated_and_clears_sources_on_switch_disable_and_reset() {
    let path = std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR");
    let dir = game_install::GameDir::open(path).unwrap();
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
    let data = VisualEffectsData::read(&db, "bmwm3gtr");
    let pose = CarPose {
        position: Vec3::ZERO,
        rotation: glam::Quat::IDENTITY,
        wheels: [nfsmw_data::car::WheelPose::default(); 4],
    };
    let contact = VisualContact {
        point: Vec3::ZERO,
        normal: Vec3::X,
        velocity: -Vec3::X * 30.0,
        impulse_delta_v: 30.0,
        surface: Some(vlt_hash("default")),
        kind: ContactKind::Barrier,
        info: None,
        prop: None,
    };
    let mut effects = VehicleEffects::new(data);
    effects.set_enabled(true, false);
    effects.step(&[contact], pose, Vec3::ZERO, 1.0 / 60.0);
    assert!(effects.pc.len() > 0);
    assert_eq!(effects.sparks.len(), 0);
    let mut streaks = Vec::new();
    effects.geometry(Vec3::ZERO, Vec3::Y, true, &mut streaks);
    effects.glows(Vec3::ZERO, Vec3::Y, &mut streaks);
    assert!(streaks.is_empty(), "stock sprites do not borrow procedural streaks or glows");

    effects.set_style(SparkStyle::RestoredExperimental);
    assert_eq!(effects.pc.len(), 0);
    effects.step(&[contact], pose, Vec3::ZERO, 1.0 / 60.0);
    assert!(effects.sparks.len() > 0);
    effects.set_style(SparkStyle::OriginalPc);
    assert_eq!(effects.sparks.len(), 0);
    effects.step(&[], pose, Vec3::ZERO, 1.0 / 60.0);
    assert_eq!(effects.pc.len(), 0, "old one-shot clocks cannot leak across styles");
    for reset in [false, true] {
        effects.set_enabled(true, false);
        effects.step(&[contact], pose, Vec3::ZERO, 1.0 / 60.0);
        assert!(effects.pc.len() > 0);
        if reset {
            effects.clear();
        }
        if !reset {
            effects.set_enabled(false, false);
        }
        effects.step(&[], pose, Vec3::ZERO, 1.0 / 60.0);
        assert_eq!(effects.pc.len(), 0);
    }
}
