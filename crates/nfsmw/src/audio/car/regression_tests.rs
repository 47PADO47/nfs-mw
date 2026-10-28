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
        sound_profile: 0,
        vehicle: None,
        audio_zone: None,
        detached: false,
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
        nitrous_fx: None,
        wind_fx: None,
        engine: Default::default(),
        telemetry: CarAudioTelemetry::default(),
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
    let npc_a = CarContact { vehicle: Some(crate::audio::ContactVehicle { identity: 7, profile: 9 }), ..a };
    let npc_b = CarContact { vehicle: Some(crate::audio::ContactVehicle { identity: 8, profile: 9 }), ..a };
    assert_eq!(contacts::coalesced(&[npc_a, npc_b]).len(), 2, "same-model cars touching one wall remain independent");
}

#[test]
#[ignore = "requires installed audio banks; set NFSMW_GAME_DIR"]
fn installed_authored_scrape_is_audible_reuses_its_voice_and_releases() {
    let mut audio = Audio::replay(install(), 48_000);
    let input = CarInput { speed: 10.0, throttle: 0.5, rpm_pct: 0.4, ..Default::default() };
    let mut recorded = Vec::new();
    for i in 0..180 {
        let contacts = if i < 60 { vec![contact()] } else { vec![] };
        let tick = CarSoundTick {
            sim_dt: blackbox_vehicle::FIXED_STEP,
            telemetry: CarAudioTelemetry::default(),
            input,
            contacts,
            surface: 0,
        };
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
            ticks.push(CarSoundTick {
                sim_dt: blackbox_vehicle::FIXED_STEP,
                telemetry: CarAudioTelemetry::default(),
                input: latest,
                contacts: vec![],
                surface: 0,
            });
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
        playback.observe(&mut audio, &sound, Some(&mixer), &[contact()], 1.0 / 60.0, &AudioView::default());
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
        playback.observe(&mut audio, &sound, Some(&mixer), &[], 0.0, &AudioView::default());
        playback.update(&mut audio, 1.0 / 60.0, &input, &view);
        audio.render_replay(800);
    }
    assert_eq!(playback.active_count(), 1);
    for _ in 0..60 {
        playback.observe(&mut audio, &sound, Some(&mixer), &[], 1.0 / 60.0, &AudioView::default());
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
            let commands: Vec<_> = (0..2)
                .map(|side| SoundCommand::SetLoop {
                    id: LoopId::Road(side),
                    sound: SoundRef::RoadNoise(terrain),
                    volume: 0.7,
                    pitch: 0.9,
                })
                .collect();
            playback.update(&mut audio, &input, &commands, Default::default(), 1.0 / 60.0);
            assert!(
                (0..2).all(|side| playback.owns_loop(LoopId::Road(side))),
                "authored graphs own ordinary road loops"
            );
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

#[test]
#[ignore = "requires installed audio banks; set NFSMW_GAME_DIR"]
fn installed_effect_pan_centres_mono_and_places_left_and_right_correctly() {
    use crate::audio::aems::{EffectModule, EffectSampler};
    for azimuth in [0, 16384, 49152] {
        let mut audio = Audio::replay(install(), 48_000);
        let mut effect = EffectModule::load(&mut audio, "IG_GLOBAL/FX_Misc_MB.abk", "FX_Scrape", 1).unwrap();
        let params = [32767, 4096, azimuth, 0, 100, 25000, 0, 32767, 0];
        let mut energy = [0.0_f64; 2];
        for tick in 0..60 {
            assert!(effect.update(&mut EffectSampler(&mut audio, crate::audio::Group::Sfx), 1.0 / 60.0, &params));
            let pcm = audio.render_replay(800);
            if tick < 20 {
                continue;
            }
            for frame in pcm.as_chunks::<2>().0 {
                energy[0] += f64::from(frame[0]).powi(2);
                energy[1] += f64::from(frame[1]).powi(2);
            }
        }
        match azimuth {
            0 => assert!(
                (energy[0] / energy[1] - 1.0).abs() < 0.01,
                "centred effect must have equal speaker energy: {energy:?}"
            ),
            16384 => assert!(energy[1] > energy[0] * 100.0, "right effect must pan right"),
            _ => assert!(energy[0] > energy[1] * 100.0, "left effect must pan left"),
        }
    }
}

#[test]
#[ignore = "requires installed audio banks; set NFSMW_GAME_DIR"]
fn installed_gentle_tyre_landing_uses_its_dedicated_sound_and_releases() {
    let gentle = tyre_landing(0.01);
    assert_eq!(gentle.peak, 0.0, "the sub-step startup gain must remain silent when unchanged");
    let ordinary = tyre_landing(0.25);
    assert!(ordinary.rms > 0.0001, "a stronger wheel return must still be audible");
}

fn tyre_landing(compression: f32) -> capture::Levels {
    let mut audio = Audio::replay(install(), 48_000);
    audio.set_volumes(crate::audio::Volumes { engine: 0.0, music: 0.0, ..Default::default() });
    let grounded = CarInput {
        speed: 0.0,
        wheels: [blackbox_carsound::WheelInput { compression, ..Default::default() }; 4],
        ..Default::default()
    };
    let air = CarInput {
        wheels: grounded.wheels.map(|w| blackbox_carsound::WheelInput { on_ground: false, ..w }),
        ..grounded
    };
    for input in std::iter::repeat_n(grounded, 20).chain(std::iter::repeat_n(air, 20)) {
        audio.drive_car(
            Some(&state(
                "BMWM3GTR",
                input,
                vec![CarSoundTick {
                    sim_dt: blackbox_vehicle::FIXED_STEP,
                    telemetry: CarAudioTelemetry::default(),
                    input,
                    contacts: vec![],
                    surface: 0,
                }],
            )),
            1.0 / 60.0,
        );
        // Isolate the landing from independently covered continuous sound families.
        let car = audio.car.as_mut().unwrap();
        car.sound.engine.has_transmission_loop = false;
        car.mixer.as_mut().unwrap().mute_continuous_effects();
        audio.render_replay(800);
    }
    audio.drive_car(
        Some(&state(
            "BMWM3GTR",
            grounded,
            vec![CarSoundTick {
                sim_dt: blackbox_vehicle::FIXED_STEP,
                telemetry: CarAudioTelemetry::default(),
                input: grounded,
                contacts: vec![],
                surface: 0,
            }],
        )),
        1.0 / 60.0,
    );
    assert_eq!(audio.impact_counter, 0, "a wheel landing must not become a physical crash event");
    assert_eq!(audio.car.as_ref().unwrap().impacts.active_count(), 1);
    let mut recorded = audio.render_replay(800).to_vec();
    for _ in 0..180 {
        audio.drive_car(
            Some(&state(
                "BMWM3GTR",
                grounded,
                vec![CarSoundTick {
                    sim_dt: blackbox_vehicle::FIXED_STEP,
                    telemetry: CarAudioTelemetry::default(),
                    input: grounded,
                    contacts: vec![],
                    surface: 0,
                }],
            )),
            1.0 / 60.0,
        );
        recorded.extend_from_slice(audio.render_replay(800));
    }
    assert_eq!(
        audio.car.as_ref().unwrap().impacts.active_count(),
        0,
        "landing modules must retire after their samples end"
    );
    let levels = capture::levels(&recorded);
    eprintln!("tyre landing at compression {compression}: {levels:?}");
    assert!(levels.peak < 0.35, "gentle wheel return must not be a full-volume crash");
    assert_eq!(levels.over_full_scale, 0);
    if let Some(dir) = std::env::var_os("NFSMW_AUDIO_CAPTURE_DIR") {
        capture::write_float(
            &std::path::PathBuf::from(dir).join(format!("tyre-landing-{compression}.wav")),
            &recorded,
            48_000,
            2,
        );
    }
    levels
}

#[test]
#[ignore = "requires installed audio banks; set NFSMW_GAME_DIR"]
fn installed_physical_impact_stitches_render_authored_pieces_and_retire() {
    let mut audio = Audio::replay(install(), 48_000);
    let input = CarInput::default();
    audio.drive_car(Some(&state("BMWM3GTR", input, vec![])), 1.0 / 60.0);
    let mut car = audio.car.take().unwrap();
    let view = AudioView::default();
    for (kind, surface) in
        [(EventKind::HitWorld, "default"), (EventKind::HitObject, "carbody"), (EventKind::BottomOut, "concrete")]
    {
        let event = CarEvent {
            sound_profile: 0,
            vehicle: None,
            audio_zone: None,
            kind,
            surface: SurfaceTable::hash_of(surface),
            magnitude: 0.5,
            front: true,
            smackable: false,
            position: Vec3::ZERO,
        };
        let before = audio.impact_counter;
        audio.play_impact(&mut car, &event, &input, &view);
        assert!(audio.impact_counter > before, "installed {kind:?} must select its authored stitch");
        let mut recorded = Vec::new();
        for _ in 0..180 {
            car.impacts.update(&mut audio, 1.0 / 60.0, 0.0, &input, &view, (1.0, 0));
            recorded.extend_from_slice(audio.render_replay(800));
        }
        assert_eq!(car.impacts.active_count(), 0, "{kind:?} pieces release");
        let levels = capture::levels(&recorded);
        eprintln!("{kind:?} stitch: {levels:?}");
        assert!(levels.rms > 0.0001);
        assert_eq!(levels.over_full_scale, 0);
        assert!(audio.missing.is_empty());
        if let Some(dir) = std::env::var_os("NFSMW_AUDIO_CAPTURE_DIR") {
            capture::write_float(
                &std::path::PathBuf::from(dir).join(format!("impact-{kind:?}.wav")),
                &recorded,
                48_000,
                2,
            );
        }
    }
}

#[test]
#[ignore = "requires installed audio banks; set NFSMW_GAME_DIR"]
fn installed_detaching_props_emit_their_own_profile_effects() {
    let mut audio = Audio::replay(install(), 48_000);
    let db = audio.database().unwrap();
    let other = SurfaceTable::hash_of("carbody");
    let default = SurfaceTable::hash_of("default");
    let candidates: Vec<_> = db
        .collections_of("smackable")
        .filter_map(|collection| {
            let rules = nfsmw_data::sound::CollisionSounds::prop(&db, collection.key());
            let rule = rules.rule(EventKind::Detached, other, default, false)?;
            if !rule.minimum.is_finite() || !rule.maximum.is_finite() || rule.maximum <= rule.minimum {
                return None;
            }
            Some((
                collection.key(),
                collection.name().unwrap_or("unnamed").to_owned(),
                (rule.minimum + rule.maximum) * 0.5,
            ))
        })
        .collect();
    assert!(!candidates.is_empty(), "installed props must retain OnDetached audio links");
    let input = CarInput::default();
    audio.drive_car(Some(&state("BMWM3GTR", input, vec![])), 1.0 / 60.0);
    let mut car = audio.car.take().unwrap();
    let mut feed = contacts::ContactPlayback::default();
    for (profile, name, speed) in candidates.into_iter().take(5) {
        let event = CarContact {
            sound_profile: profile,
            vehicle: None,
            audio_zone: None,
            detached: true,
            identity: u64::from(profile),
            surface: other,
            velocity: Vec3::X * speed.max(1.01),
            normal: Vec3::NEG_X,
            delta_v: 0.1,
            source: ContactSource::Object,
            front: false,
            ..contact()
        };
        let events =
            feed.observe(&mut audio, &car.sound, car.mixer.as_ref(), &[event], 1.0 / 60.0, &AudioView::default());
        assert_eq!(events.len(), 1, "{name} detachment must emit one owned hit");
        assert_eq!(events[0].sound_profile, profile);
        assert_eq!(events[0].kind, EventKind::Detached);
        let rules = nfsmw_data::sound::CollisionSounds::prop(&db, profile);
        let rule = rules.rule(EventKind::Detached, other, default, false).unwrap();
        let physical = rule.magnitude(event.velocity.length()).unwrap();
        assert!((events[0].magnitude - rule.audio_magnitude(physical)).abs() < 1e-5);
        let far = AudioView { camera_position: Vec3::splat(1000.0), ..Default::default() };
        assert!(feed.observe(&mut audio, &car.sound, car.mixer.as_ref(), &[event], 1.0 / 60.0, &far).is_empty());
        audio.play_impact(&mut car, &events[0], &input, &AudioView::default());
        assert!(car.impacts.active_count() > 0);
        let mut recorded = Vec::new();
        for _ in 0..180 {
            car.impacts.update(&mut audio, 1.0 / 60.0, 0.0, &input, &AudioView::default(), (1.0, 0));
            recorded.extend_from_slice(audio.render_replay(800));
        }
        let levels = capture::levels(&recorded);
        eprintln!("{name} prop-owned detachment: {levels:?}");
        assert!(levels.rms > 0.0001);
        assert_eq!(levels.over_full_scale, 0);
        assert_eq!(car.impacts.active_count(), 0);
    }
}
