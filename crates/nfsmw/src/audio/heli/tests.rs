use super::*;
use params::{RotorInputs, class_words, distance_word, speed_fraction, speed_word};

fn frame(dt: f32, position: Vec3, forward: Vec3, velocity: Vec3) -> HeliFrame {
    HeliFrame { dt, position, forward, velocity }
}

fn inputs() -> RotorInputs {
    RotorInputs {
        volume: 20000,
        azimuth: 0,
        pitch: 4096,
        in_front: true,
        distance: 50.0,
        forward_speed: 250.0,
        bearing: std::f32::consts::PI,
    }
}

#[test]
fn distance_word_is_metres_clamped_to_a_hundred_and_negative_behind() {
    assert_eq!(distance_word(42.7, true), 42);
    assert_eq!(distance_word(42.7, false), -42);
    assert_eq!(distance_word(150.0, true), 100);
    assert_eq!(distance_word(150.0, false), -100);
    assert_eq!(distance_word(f32::NAN, true), 0);
}

#[test]
fn speed_is_the_forward_speed_over_five_hundred_with_the_idle_below_a_quarter() {
    assert_eq!(speed_fraction(0.1), 0.5);
    assert_eq!(speed_fraction(0.0), 0.5);
    assert_eq!(speed_fraction(0.25), 0.25 / 500.0);
    assert_eq!(speed_fraction(125.0), 0.25);
    assert_eq!(speed_fraction(1000.0), 1.0);
    assert_eq!(speed_fraction(f32::NAN), 0.0);
    assert_eq!(speed_word(500.0), 1023);
    assert_eq!(speed_word(0.1), 511);
}

#[test]
fn class_words_follow_the_publisher_clamps_and_the_inputs() {
    let words = class_words(inputs());
    assert_eq!(words[0], 20000);
    assert_eq!(words[1], 0);
    assert_eq!(words[2], 1);
    assert_eq!(words[3], 4096);
    assert_eq!(words[4], 65535);
    assert_eq!(words[6], 50);
    assert_eq!(words[7], 25000);
    assert_eq!(words[8], 25000);
    assert_eq!(words[9], 32767);
    assert_eq!(words[11], 511);
}

#[test]
fn a_rotor_behind_the_listener_has_a_negative_distance_and_no_front_flag() {
    let words = class_words(RotorInputs { in_front: false, distance: 150.0, ..inputs() });
    assert_eq!(words[2], 0);
    assert_eq!(words[6], -100);
}

#[test]
fn azimuth_is_signed_and_the_levels_stay_in_range() {
    let words = class_words(RotorInputs { azimuth: 0xFFFF, volume: 40000, pitch: -5, ..inputs() });
    assert_eq!(words[1], -1);
    assert_eq!(words[0], 32767);
    assert_eq!(words[3], 0);
    let half = class_words(RotorInputs { azimuth: 0x8000, ..inputs() });
    assert_eq!(half[1], -8192);
}

#[test]
fn only_a_finite_frame_with_time_passing_plays() {
    let good = frame(1.0 / 60.0, Vec3::ONE, Vec3::X, Vec3::X * 40.0);
    assert!(good.is_usable());
    assert!(!HeliFrame { dt: 0.0, ..good }.is_usable());
    assert!(!HeliFrame { dt: f32::NAN, ..good }.is_usable());
    assert!(!HeliFrame { position: Vec3::splat(f32::INFINITY), ..good }.is_usable());
    assert!(!HeliFrame { velocity: Vec3::new(f32::NAN, 0.0, 0.0), ..good }.is_usable());
}

#[test]
#[ignore = "requires NFSMW_GAME_DIR; the map's state 11 must carry the slots and controller the rotor writes"]
fn installed_map_routes_the_rotor_to_the_slots_the_mixer_reads() {
    use blackbox_mixmap::MixMap;
    let path = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR required");
    let dir = game_install::GameDir::open(std::path::PathBuf::from(&path)).unwrap();
    let map = MixMap::parse(&dir.read("SOUND/MIXMAPS/MAPOUTPUT.mxb").unwrap()).unwrap();
    let state = map.state(11).expect("state 11 (helicopter)");
    let object = |m: &blackbox_mixmap::MasterChannel| (m.object.state(), m.object.object());
    let masters: Vec<_> = state.masters.iter().filter(|m| object(m) == (11, 0)).collect();
    let slot = |kind: blackbox_mixmap::OutputKind| {
        masters.iter().find(|m| m.kind == kind).map(|m| m.words.iter().map(|w| (w.slot, w.azimuth)).collect::<Vec<_>>())
    };
    assert_eq!(slot(blackbox_mixmap::OutputKind::Volume), Some(vec![(0, true), (1, false)]));
    assert_eq!(slot(blackbox_mixmap::OutputKind::Pitch), Some(vec![(2, false)]));
    assert!(state.spatial.iter().all(|s| s.id.object() == 1), "the rotor's 3D control reads controller 1");
}

#[test]
#[ignore = "requires NFSMW_GAME_DIR; loads IG_GLOBAL/FX_Helicopter_MB.abk and runs the rotor"]
fn installed_rotor_module_loads_plays_and_stops() {
    let path = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR required");
    let dir = game_install::GameDir::open(std::path::PathBuf::from(&path)).unwrap();
    let mut audio = Audio::replay(dir, 48000);
    let frame =
        HeliFrame { dt: 1.0 / 60.0, position: Vec3::new(20.0, 30.0, 0.0), forward: Vec3::X, velocity: Vec3::X * 60.0 };
    for _ in 0..60 {
        audio.update_heli(Some(frame), AudioView::default());
    }
    assert!(audio.heli.rotor.is_some(), "the rotor did not start: {}", audio.heli.failed);
    audio.update_heli(None, AudioView::default());
    assert!(audio.heli.rotor.is_none());
}
