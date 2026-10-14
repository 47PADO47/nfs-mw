use super::*;
use blackbox_carsound::SweetenerKind;

fn levels() -> PowerLevels {
    PowerLevels {
        nitrous_volume: 24000,
        purge_volume: 24000,
        nitrous_azimuth: 1200,
        nitrous_pitch: 4096,
        nitrous_low_pass: 22000,
        shift_azimuth: 500,
        shift_volume: [28000; 7],
        turbo_azimuth: 1200,
        turbo_volume: [28000; 3],
        whine_volume: 24000,
    }
}

#[test]
fn nitrous_stop_and_purge_keep_their_distinct_original_class_contracts() {
    assert_eq!(params::nitrous(levels(), false), [0, 24000, 1200, 0, 4096, 22000, 0, 32767, 0]);
    assert_eq!(params::nitrous(levels(), true)[3], 1);
    assert_eq!(params::purge(levels()), [24000, 1200, 0, 4096, 25000, 0, 32767, 0]);
    let extreme = PowerLevels {
        nitrous_volume: 50000,
        purge_volume: -10,
        nitrous_azimuth: 70000,
        nitrous_pitch: 20000,
        nitrous_low_pass: -1,
        ..levels()
    };
    assert_eq!(params::nitrous(extreme, true), [0, 32767, 65535, 1, 16384, 0, 0, 32767, 0]);
    assert_eq!(params::purge(extreme)[0], 0);
}

#[test]
fn power_gain_preserves_sequential_integer_products_and_fractional_rpm_truncation() {
    assert_eq!(params::sweet(20000, 4000.0, 28000), 13559);
    assert_eq!(params::sweet(20000, 7000.0, 28000), 17088);
    assert_eq!(params::sweet(-1, 7000.0, 28000), 0);
    assert_eq!(params::clunk(20000, 8000.0, 28000), 17088);
    assert_eq!(params::clunk(20000, 1500.0, 28000), 2666);
    assert_eq!(params::clunk(20000, 4500.0, 28000), 12358);
    assert_eq!(params::clunk(20000, 4500.9, 28000), params::clunk(20000, 4500.0, 28000));
    assert_eq!(params::clunk(20000, 8000.0, 0), 0);
}

#[test]
fn reverse_uses_physical_maximum_and_turbo_params_keep_native_bounds() {
    let frame = PowerFrame { physics_rpm: 6000.0, max_physics_rpm: 8000.0, ..Default::default() };
    assert_eq!(params::whine(frame, levels()), [0, 6000, 18000, 500, 0, 0, 25000, 0, 32767]);
    assert_eq!(params::whine(PowerFrame { max_physics_rpm: 0.0, ..frame }, levels())[2], 0);
    assert_eq!(params::turbo(10, 50000, 2000, 70000, -2, 12000.0), [5, 32767, 1024, 65535, 0, 10000]);
    assert_eq!(params::turbo(-1, -2, -3, -4, -5, -6.0), [0; 6]);
    assert_eq!(params::turbo(1, 20000, 512, 1200, 700, 7500.0), [1, 20000, 512, 1200, 700, 7500]);
}

#[test]
#[ignore = "requires installed audio banks; set NFSMW_GAME_DIR"]
fn installed_reverse_spool_and_blowoff_render_and_drop_across_render_rates() {
    for rate in [44100usize, 48000] {
        for fps in [5usize, 60, 144, 360] {
            for (name, reverse) in [("BMWM3GTR", true), ("LANCEREVO8", false)] {
                let mut audio = installed_audio(rate as u32);
                let sound = audio.load_car_engine(name).unwrap().sound;
                assert!(reverse || sound.turbo.is_some());
                let mut playback = PowerPlayback::default();
                let mut peak = 0.0f32;
                let mut tail = 0.0f32;
                for tick in 0..fps * 6 {
                    let input = CarInput {
                        gear: if reverse { blackbox_carsound::GEAR_REVERSE } else { blackbox_carsound::GEAR_FIRST },
                        throttle: u8::from(tick < fps * 3) as f32,
                        ..Default::default()
                    };
                    let frame = PowerFrame {
                        physics_rpm: 6000.0,
                        max_physics_rpm: 8000.0,
                        engine_torque: input.throttle * 100.0,
                        physics_torque: input.throttle * 100.0,
                        ..Default::default()
                    };
                    if !reverse && tick == fps * 3 {
                        assert!(playback.play(&mut audio, &sound, frame, Some(levels()), SoundRef::TurboBlowoff(0)));
                    }
                    playback.update(
                        &mut audio,
                        &sound,
                        &input,
                        frame,
                        (tick < fps * 5).then_some(levels()),
                        1.0 / fps as f32,
                    );
                    if tick < fps * 5 {
                        assert!(playback.owns(if reverse { SoundRef::ReverseWhine } else { SoundRef::TurboSpool }));
                    }
                    let first = tick * rate / fps;
                    let last = (tick + 1) * rate / fps;
                    for sample in audio.render_replay(last - first) {
                        assert!(sample.is_finite());
                        if tick < fps * 5 {
                            peak = peak.max(sample.abs());
                        }
                        if tick * 2 >= fps * 11 {
                            tail = tail.max(sample.abs());
                        }
                    }
                }
                assert!(peak > 0.0001, "{name} {rate}/{fps}: {peak}");
                assert!(tail < 0.0001, "{name} {rate}/{fps} tail: {tail}");
                assert!(audio.missing.is_empty());
            }
        }
    }
}

fn installed_audio(rate: u32) -> Audio {
    let dir = game_install::GameDir::open(std::path::PathBuf::from(
        std::env::var_os("NFSMW_GAME_DIR").expect("requires installed audio banks"),
    ))
    .unwrap();
    Audio::replay(dir, rate)
}

#[test]
#[ignore = "requires installed audio banks; set NFSMW_GAME_DIR"]
fn installed_power_routes_original_up_down_ids_and_every_sweetener_class() {
    let mut audio = installed_audio(48000);
    let sound = audio.load_car_engine("BMWM3GTR").unwrap().sound;
    let mut playback = PowerPlayback::default();
    let frame = PowerFrame { physics_rpm: 7500.0, rpm_at_shift: 8000.0, camera_pov_zero: true, ..Default::default() };
    for (event, expected) in
        [(SoundRef::GearClunk { up: true }, 1), (SoundRef::GearClunk { up: false }, 0), (SoundRef::BrakeMash, 2)]
    {
        let (_, _, words) = params::shot(&sound, frame, levels(), event).unwrap();
        assert_eq!(words[0], expected);
        assert_eq!(&words[2..], &[4096, 500, 0, 1]);
        assert!(playback.play(&mut audio, &sound, frame, Some(levels()), event));
    }
    for kind in [SweetenerKind::Disengage, SweetenerKind::Engage, SweetenerKind::Accelerate, SweetenerKind::EngineOff] {
        assert!(playback.play(&mut audio, &sound, frame, Some(levels()), SoundRef::Sweetener(kind)));
    }
    let input = CarInput { brake: 1.0, ..Default::default() };
    let mut peak = 0.0f32;
    for _ in 0..120 {
        playback.update(&mut audio, &sound, &input, frame, Some(levels()), 1.0 / 60.0);
        peak = audio.render_replay(800).iter().fold(peak, |peak, sample| peak.max(sample.abs()));
    }
    assert!(peak > 0.0001, "authored shift/sweetener voices must render");
    assert!(audio.missing.is_empty());
    playback.update(&mut audio, &sound, &input, frame, None, 0.0);
    assert!(playback.shots.iter().all(Option::is_none));
}

#[test]
#[ignore = "requires installed audio banks; set NFSMW_GAME_DIR"]
fn installed_nitrous_purge_keep_pcm_and_retire_voices_across_render_rates() {
    for rate in [44100usize, 48000] {
        let mut energies = Vec::new();
        for fps in [5usize, 60, 144, 360] {
            let mut audio = installed_audio(rate as u32);
            let sound = audio.load_car_engine("BMWM3GTR").unwrap().sound;
            let mut playback = PowerPlayback::default();
            let frame = PowerFrame::default();
            let (mut energy, mut samples, mut starts, mut max_active) = (0.0f64, 0usize, 0u32, 0usize);
            let (mut purge_peak, mut tail_peak) = (0.0f32, 0.0f32);
            for tick in 0..fps * 8 {
                let input = CarInput { nos_active: (fps..3 * fps).contains(&tick), ..Default::default() };
                if tick == 3 * fps {
                    assert!(playback.play(&mut audio, &sound, frame, Some(levels()), SoundRef::Purge));
                }
                playback.update(&mut audio, &sound, &input, frame, Some(levels()), 1.0 / fps as f32);
                if let Some(running) = &playback.nitrous {
                    let counts = running.module.voice_counts();
                    starts = starts.max(counts.0);
                    max_active = max_active.max(counts.1);
                }
                let first = tick * rate / fps;
                let last = (tick + 1) * rate / fps;
                for (index, sample) in audio.render_replay(last - first).iter().enumerate() {
                    assert!(sample.is_finite());
                    let clock = first + index / 2;
                    if (2 * rate..3 * rate).contains(&clock) {
                        energy += f64::from(*sample).powi(2);
                        samples += 1;
                    }
                    if (3 * rate..5 * rate).contains(&clock) {
                        purge_peak = purge_peak.max(sample.abs());
                    }
                    if clock >= 7 * rate {
                        tail_peak = tail_peak.max(sample.abs());
                    }
                }
            }
            let rms = (energy / samples as f64).sqrt();
            assert!(rms > 0.0001, "nitrous {rate}/{fps}: {rms}");
            assert!(purge_peak > 0.0001, "purge {rate}/{fps}: {purge_peak}");
            assert!(starts > 0 && max_active <= 6, "bounded authored nitrous voices");
            assert!(tail_peak < 0.0001, "power tail {rate}/{fps}: {tail_peak}");
            assert!(playback.nitrous.is_none() && playback.purge.is_none(), "graphs must finish");
            assert!(audio.missing.is_empty());
            energies.push((fps, rms));
        }
        let baseline = energies[1].1;
        for (fps, rms) in energies {
            assert!((0.8..=1.2).contains(&(rms / baseline)), "nitrous {rate}/{fps}: {rms}, 60FPS {baseline}");
        }
    }
}
