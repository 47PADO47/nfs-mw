use super::*;
use crate::audio::{AudioView, capture};
use game_install::GameDir;
use glam::Vec3;
use nfsmw_data::car::physics::SurfaceTable;

fn install() -> GameDir {
    GameDir::open(std::path::PathBuf::from(
        std::env::var_os("NFSMW_GAME_DIR").expect("installed audio tests require NFSMW_GAME_DIR"),
    ))
    .unwrap()
}

fn contact() -> CarContact {
    CarContact {
        source: ContactSource::World,
        identity: 1,
        surface: SurfaceTable::hash_of("default"),
        point: Vec3::new(0.0, 0.0, 2.0),
        normal: Vec3::NEG_Z,
        velocity: Vec3::new(10.0, 0.0, 3.0),
        delta_v: 0.1,
        front: true,
        bottom: false,
    }
}

fn state(car: &str, input: CarInput, ticks: Vec<CarSoundTick>) -> CarSoundState {
    CarSoundState {
        car: car.into(),
        upgrades: SoundUpgrades::default(),
        input,
        events: vec![],
        scrape: None,
        surface: SurfaceTable::hash_of("default"),
        ticks,
        view: AudioView::default(),
    }
}

#[test]
fn physical_contact_coalescing_keeps_distinct_partners_and_lateral_scrapes() {
    let a = contact();
    let stronger = CarContact { delta_v: 5.0, ..a };
    let other = CarContact { identity: 2, ..a };
    let out = contacts::coalesced(&[a, stronger, other]);
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].delta_v, 5.0);
    assert_eq!(a.sliding_speed(), 10.0);
    assert_eq!(CarContact { velocity: Vec3::X * 10.0, ..a }.closing_speed(), 0.0);
}

#[test]
#[ignore = "requires installed audio banks; set NFSMW_GAME_DIR"]
fn installed_authored_scrape_is_audible_reuses_its_voice_and_releases() {
    let mut audio = Audio::replay(install(), 48_000);
    let input = CarInput { speed: 10.0, throttle: 0.5, rpm_pct: 0.4, ..Default::default() };
    let mut recorded = Vec::new();
    for i in 0..180 {
        let contacts = if i < 60 { vec![contact()] } else { vec![] };
        let tick = CarSoundTick { input, contacts, surface: 0 };
        audio.drive_car(Some(&state("BMWM3GTR", input, vec![tick])), 1.0 / 60.0);
        recorded.extend_from_slice(audio.render_replay(800));
        let count = audio.car.as_ref().unwrap().contacts.active_count();
        if i == 30 {
            assert_eq!(count, 1, "the selected authored scrape must start");
        }
        if i == 100 {
            assert_eq!(count, 0, "scraping must release after contact stops");
        }
    }
    let levels = capture::levels(&recorded);
    eprintln!("authored scrape mixed output: {levels:?}");
    assert!(levels.rms > 0.005);
    assert!(audio.missing.is_empty());
    if let Some(dir) = std::env::var_os("NFSMW_AUDIO_CAPTURE_DIR") {
        capture::write_float(&std::path::PathBuf::from(dir).join("scrape-mix.wav"), &recorded, 48_000, 2);
    }
}

fn replay(rate: u32, fps: u32, car: &str) -> (capture::Levels, u64) {
    let mut audio = Audio::replay(install(), rate);
    audio.set_volumes(crate::audio::Volumes { sfx: 0.0, music: 0.0, ..Default::default() });
    let mut recorded = Vec::new();
    let mut physics_tick = 0;
    let mut latest = CarInput::default();
    for frame in 0..fps * 6 {
        let until = ((u64::from(frame + 1) * 60) / u64::from(fps)) as u32;
        let mut ticks = Vec::new();
        while physics_tick < until {
            let time = physics_tick as f32 / 60.0;
            latest = CarInput {
                rpm_pct: if time < 2.0 {
                    time / 2.0 * 0.8
                } else if time < 4.0 {
                    1.0
                } else {
                    0.4
                },
                throttle: if time < 4.0 { 1.0 } else { 0.0 },
                speed: 70.0,
                gear: 5,
                ..Default::default()
            };
            ticks.push(CarSoundTick { input: latest, contacts: vec![], surface: 0 });
            physics_tick += 1;
        }
        audio.drive_car(Some(&state(car, latest, ticks)), 1.0 / fps as f32);
        let from = (u64::from(frame) * u64::from(rate) / u64::from(fps)) as usize;
        let to = (u64::from(frame + 1) * u64::from(rate) / u64::from(fps)) as usize;
        recorded.extend_from_slice(audio.render_replay(to - from));
    }
    let revision = audio.car.as_ref().unwrap().engine.output().tick_revision;
    let levels = capture::levels(&recorded);
    if let Some(dir) = std::env::var_os("NFSMW_AUDIO_CAPTURE_DIR") {
        capture::write_float(
            &std::path::PathBuf::from(dir).join(format!("{car}-{rate}-{fps}.wav")),
            &recorded,
            rate,
            2,
        );
    }
    (levels, revision)
}

#[test]
#[ignore = "requires installed audio banks; set NFSMW_GAME_DIR"]
fn installed_engine_playback_remains_audible_and_controls_keep_time_across_frame_rates() {
    for rate in [44_100, 48_000] {
        let mut reference = None;
        for fps in [5, 30, 60, 120, 144, 240, 360] {
            let (levels, revision) = replay(rate, fps, "BMWM3GTR");
            eprintln!("{rate} Hz {fps} FPS: {levels:?}, controller ticks {revision}");
            assert_eq!(revision, 360, "controller time must not be lost");
            assert!(levels.rms > 0.005, "complete engine must remain audible");
            assert_eq!(levels.over_full_scale, 0, "engine playback must retain headroom");
            let baseline = *reference.get_or_insert(levels.rms);
            assert!((levels.rms / baseline - 1.0).abs() < 0.05, "render FPS must not change overall engine loudness");
        }
    }
    for car in ["COBALTSS", "TT", "CORVETTE", "LANCEREVO8", "911TURBO"] {
        let (levels, revision) = replay(48_000, 144, car);
        eprintln!("{car} engine output: {levels:?}");
        assert_eq!(revision, 360);
        assert!(levels.rms > 0.005);
        assert_eq!(levels.over_full_scale, 0);
    }
}

#[test]
#[ignore = "requires installed audio banks; set NFSMW_GAME_DIR"]
fn installed_scrape_module_alone_is_audible_and_sim_timeout_releases_in_audio_time() {
    let mut audio = Audio::replay(install(), 48_000);
    let sound = audio.load_car_engine("BMWM3GTR").unwrap().sound;
    let mixer = CarMixer::load(&audio.dir, true).unwrap();
    let mut playback = contacts::ContactPlayback::default();
    let input = CarInput { speed: 10.0, ..Default::default() };
    let view = AudioView::default();
    let mut recorded = Vec::new();
    for _ in 0..60 {
        playback.observe(&mut audio, &sound, Some(&mixer), &[contact()], 1.0 / 60.0);
        playback.update(&mut audio, 1.0 / 60.0, &input, &view);
        recorded.extend_from_slice(audio.render_replay(800));
    }
    assert_eq!(playback.active_count(), 1);
    let levels = capture::levels(&recorded);
    eprintln!("scrape-only output: {levels:?}");
    assert!(levels.rms > 0.001, "scrape itself must be audible");
    assert_eq!(levels.over_full_scale, 0);
    // Audio continues during a stalled/paused simulation; SIM timeout must not expire.
    for _ in 0..30 {
        playback.observe(&mut audio, &sound, Some(&mixer), &[], 0.0);
        playback.update(&mut audio, 1.0 / 60.0, &input, &view);
        audio.render_replay(800);
    }
    assert_eq!(playback.active_count(), 1);
    for _ in 0..60 {
        playback.observe(&mut audio, &sound, Some(&mixer), &[], 1.0 / 60.0);
        playback.update(&mut audio, 1.0 / 60.0, &input, &view);
        audio.render_replay(800);
    }
    assert_eq!(playback.active_count(), 0);
    assert!(audio.missing.is_empty());
    if let Some(dir) = std::env::var_os("NFSMW_AUDIO_CAPTURE_DIR") {
        capture::write_float(&std::path::PathBuf::from(dir).join("scrape-only.wav"), &recorded, 48_000, 2);
    }
}

#[test]
#[ignore = "requires installed audio banks; set NFSMW_GAME_DIR"]
fn installed_authored_wind_and_road_modules_render_all_ordinary_terrain_ids() {
    for terrain in 0..8 {
        let mut audio = Audio::replay(install(), 48_000);
        let mut playback = ambient::AmbientPlayback::default();
        let input = CarInput { speed: 30.0, ..Default::default() };
        let mut recorded = Vec::new();
        for _ in 0..120 {
            let mut commands = (0..2)
                .map(|side| SoundCommand::SetLoop {
                    id: LoopId::Road(side),
                    sound: SoundRef::RoadNoise(terrain),
                    volume: 0.7,
                    pitch: 0.9,
                })
                .collect();
            playback.update(&mut audio, &input, &mut commands, Default::default(), 1.0 / 60.0);
            assert!(commands.is_empty(), "authored graphs replace ordinary raw loops");
            recorded.extend_from_slice(audio.render_replay(800));
        }
        assert!(playback.available(), "wind and both road modules remain usable for {terrain}");
        let levels = capture::levels(&recorded);
        assert!(levels.rms > 0.001);
        assert_eq!(levels.over_full_scale, 0);
        assert!(audio.missing.is_empty());
        eprintln!("terrain {terrain} wind/road output: {levels:?}");
    }
}
