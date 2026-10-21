use super::params::{Changes, RADIUS, SLOTS, assign, class_words};
use super::*;

fn source(id: u32, state: State, at: Vec3) -> SirenSource {
    SirenSource { id, state, position: at, velocity: Vec3::ZERO }
}

fn frame(sources: Vec<SirenSource>) -> SirenFrame {
    SirenFrame { dt: 1.0 / 60.0, sources }
}

#[test]
fn class_words_follow_the_setter_clamps() {
    let words = class_words(20000, 4096 + 100, 0xFFFF, 50.0, 1);
    assert_eq!(words, [20000, 100, 65535, 640, 1, 25000, 0, 32767, 0]);
    // The distance word is metres times 12.8 and stops at 1024 (80 m).
    assert_eq!(class_words(0, 4096, 0, 80.0, 0)[3], 1024);
    assert_eq!(class_words(0, 4096, 0, 500.0, 0)[3], 1024);
    assert_eq!(class_words(0, 4096, 0, f32::NAN, 0)[3], 0);
    // Volume, pitch offset and type are clamped.
    let loud = class_words(99999, 40000, 0, 0.0, 9);
    assert_eq!((loud[0], loud[1], loud[4]), (32767, 0x4000, 7));
    assert_eq!(class_words(-5, 0, 0, 0.0, -1)[..2], [0, -4096]);
    assert_eq!(class_words(0, -90000, 0, 0.0, -1)[1], -0x4000);
}

#[test]
fn four_voices_attach_within_the_radius_and_the_fifth_car_waits() {
    let mut slots = [None; SLOTS];
    let cars: Vec<(u32, f32)> = (1..=5).map(|id| (id, 20.0 * id as f32)).collect();
    let changes = assign(&mut slots, &cars);
    assert_eq!(changes, Changes { attached: vec![0, 1, 2, 3], detached: vec![] });
    assert_eq!(slots, [Some(1), Some(2), Some(3), Some(4)]);
}

#[test]
fn a_car_beyond_the_radius_gets_no_voice() {
    let mut slots = [None; SLOTS];
    let changes = assign(&mut slots, &[(1, RADIUS + 1.0), (2, RADIUS - 1.0), (3, RADIUS)]);
    assert_eq!(changes.attached, vec![0]);
    assert_eq!(slots, [Some(2), None, None, None]);
}

#[test]
fn a_voice_stays_with_its_car_and_a_freed_slot_goes_to_the_waiting_car() {
    let mut slots = [None; SLOTS];
    let mut cars: Vec<(u32, f32)> = (1..=5).map(|id| (id, 10.0)).collect();
    assign(&mut slots, &cars);
    // Car 2 drives off: its slot goes to car 5, the others keep theirs.
    cars[1].1 = RADIUS + 20.0;
    let changes = assign(&mut slots, &cars);
    assert_eq!(changes, Changes { attached: vec![1], detached: vec![1] });
    assert_eq!(slots, [Some(1), Some(5), Some(3), Some(4)]);
    // Nothing changes while the cars hold still.
    assert_eq!(assign(&mut slots, &cars), Changes::default());
}

#[test]
fn a_car_that_is_gone_loses_its_voice() {
    let mut slots = [Some(7), None, None, None];
    let changes = assign(&mut slots, &[]);
    assert_eq!(changes.detached, vec![0]);
    assert_eq!(slots, [None; SLOTS]);
}

#[test]
fn only_a_finite_frame_with_time_passing_plays() {
    assert!(frame(vec![]).is_usable());
    assert!(!SirenFrame { dt: 0.0, ..frame(vec![]) }.is_usable());
    assert!(!SirenFrame { dt: f32::NAN, ..frame(vec![]) }.is_usable());
    assert!(!SirenFrame { dt: 5.0, ..frame(vec![]) }.is_usable(), "a long stall restarts rather than lags");
}

#[test]
#[ignore = "requires NFSMW_GAME_DIR; the map's state 4 must carry the siren's slots and controller"]
fn installed_map_routes_the_siren_to_the_slots_the_mixer_reads() {
    use blackbox_mixmap::{MixMap, OutputKind};
    let path = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR required");
    let dir = game_install::GameDir::open(std::path::PathBuf::from(&path)).unwrap();
    let map = MixMap::parse(&dir.read("SOUND/MIXMAPS/MAPOUTPUT.mxb").unwrap()).unwrap();
    let state = map.state(4).expect("state 4 (cop car)");
    let masters: Vec<_> = state.masters.iter().filter(|m| m.object.object() == 17).collect();
    let slots = |kind: OutputKind| {
        masters
            .iter()
            .filter(|m| m.kind == kind)
            .map(|m| m.words.iter().map(|w| (w.slot, w.azimuth)).collect::<Vec<_>>())
            .collect::<Vec<_>>()
    };
    assert_eq!(slots(OutputKind::Volume), vec![vec![(0, true), (1, false)], vec![(4, false)]]);
    assert_eq!(slots(OutputKind::Pitch), vec![vec![(3, false)]]);
    assert!(state.spatial.iter().all(|s| s.id.object() == 7), "the 3D controls read the position controller");
}

#[test]
#[ignore = "requires NFSMW_GAME_DIR; loads IG_GLOBAL/SIREN_MB.abk, runs the siren and renders it"]
fn installed_sirens_decode_play_follow_the_state_and_stop() {
    let path = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR required");
    let dir = game_install::GameDir::open(std::path::PathBuf::from(&path)).unwrap();
    let mut audio = Audio::replay(dir, 48000);
    let view = AudioView::default();
    // The bank decodes: every sound the module can play.
    let sounds = audio.bank_len(BANK).expect("SIREN_MB.abk");
    assert!(sounds >= 5, "{sounds} sounds");
    for index in 1..=sounds {
        audio.bank_sound(BANK, index).unwrap_or_else(|e| panic!("sound {index}: {e}"));
    }

    // A car 20 m ahead wails for a while: the voice starts at once with the wail (type 0), then follows.
    let wailing = vec![source(1, State::Wail, Vec3::new(20.0, 0.0, 0.0))];
    let mut energy = 0.0f64;
    for _ in 0..180 {
        audio.update_sirens(Some(frame(wailing.clone())), view);
        let samples = audio.render_replay(800);
        assert!(samples.iter().all(|s| s.is_finite()));
        energy += samples.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>();
    }
    assert!(energy > 0.001, "the siren must produce PCM");
    assert_eq!(audio.siren_status(), vec![(1, Some(0))]);

    // The car's light bar goes dark: the voice stops and the sound fades out.
    let dark = vec![source(1, State::Off, Vec3::new(20.0, 0.0, 0.0))];
    for _ in 0..30 {
        audio.update_sirens(Some(frame(dark.clone())), view);
        audio.render_replay(800);
    }
    assert_eq!(audio.siren_status(), vec![(1, None)]);
    audio.render_replay(48000);
    assert!(audio.render_replay(480).iter().all(|s| s.abs() < 1e-4), "an off siren is silent");

    // Five cars: only four sound.
    let many: Vec<_> = (1..=5).map(|id| source(id, State::Scream, Vec3::new(10.0 + id as f32, 5.0, 0.0))).collect();
    for _ in 0..30 {
        audio.update_sirens(Some(frame(many.clone())), view);
        audio.render_replay(800);
    }
    let status = audio.siren_status();
    assert_eq!(status.len(), 4, "{status:?}");
    assert!(status.iter().all(|&(id, kind)| id <= 4 && kind == Some(2)), "{status:?}");

    // A near siren is louder than a far one (the map's rolloff places it).
    let loudness = |at: f32| {
        let mut audio = Audio::replay(
            game_install::GameDir::open(std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR").unwrap())).unwrap(),
            48000,
        );
        let sources = vec![source(1, State::Wail, Vec3::new(at, 0.0, 0.0))];
        let mut energy = 0.0f64;
        for step in 0..120 {
            audio.update_sirens(Some(frame(sources.clone())), view);
            let samples = audio.render_replay(800);
            if step >= 60 {
                energy += samples.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>();
            }
        }
        energy
    };
    let sweep: Vec<f64> = [5.0, 15.0, 30.0, 60.0, 80.0, 100.0, 140.0].map(loudness).to_vec();
    println!("wail energy at 5, 15, 30, 60, 80, 100, 140 m: {sweep:.4?}");
    assert!(sweep.windows(2).all(|pair| pair[0] > pair[1]), "the siren fades with distance: {sweep:?}");
    assert!(sweep[6] < 1e-6, "and is gone far out");

    // The sirens stop with the sound.
    audio.stop_sirens();
    assert!(audio.siren_status().is_empty());
}
