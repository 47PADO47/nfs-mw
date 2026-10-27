//! Exercise the scene-to-controller-to-backend path, including exclusive ownership.
use super::*;

#[test]
#[ignore = "requires installed car banks; set NFSMW_GAME_DIR"]
fn installed_ordered_controller_trace_has_no_duplicate_power_fallbacks() {
    replay(false);
}

#[test]
#[ignore = "requires installed car banks; set NFSMW_GAME_DIR"]
fn installed_speedbreaker_reaches_authored_nitrous_voices_without_duplicate_pitch() {
    replay(true);
}

fn replay(speedbreaker: bool) {
    let install = std::env::var_os("NFSMW_GAME_DIR").expect("installed trace replay requires NFSMW_GAME_DIR");
    let evidence = std::env::var_os("NFSMW_AUDIO_EVIDENCE_DIR").map(std::path::PathBuf::from);
    for fps in [5usize, 60, 144, 360] {
        let path = evidence.as_ref().map_or_else(
            || std::env::temp_dir().join(format!("nfsmw-routing-{}-{fps}-{speedbreaker}.jsonl", std::process::id())),
            |dir| dir.join(format!("routing-{fps}-{speedbreaker}.jsonl")),
        );
        let dir = game_install::GameDir::open(std::path::PathBuf::from(&install)).unwrap();
        let mut audio = Audio::replay(dir, 48000);
        audio.start_trace(&path).unwrap();
        let mut physics_tick = 0;
        let mut owed = 0.0_f64;
        let mut peak = 0.0_f32;
        let mut latest = (CarInput::default(), CarAudioTelemetry::default());
        for frame in 0..fps * 7 {
            owed += 120.0 / fps as f64;
            let mut ticks = Vec::new();
            while owed + 1e-9 >= 1.0 {
                owed -= 1.0;
                let time = physics_tick as f32 / 120.0;
                let input = CarInput {
                    rpm_pct: 0.8,
                    throttle: if time < 4.0 { 1.0 } else { 0.0 },
                    gear: match time {
                        t if t < 3.0 => 2,
                        t if t < 4.0 => 3,
                        t if t < 5.0 => 2,
                        _ => 0,
                    },
                    speed: 35.0,
                    nos_active: (1.0..3.0).contains(&time),
                    nos_empty: (3.0..4.0).contains(&time),
                    ..Default::default()
                };
                ticks.push(CarSoundTick {
                    sim_dt: blackbox_vehicle::FIXED_STEP,
                    input,
                    telemetry: CarAudioTelemetry { forward_speed: 35.0, max_physics_rpm: 8500.0, ..Default::default() },
                    contacts: Vec::new(),
                    surface: 0,
                });
                physics_tick += 1;
            }
            if let Some(tick) = ticks.last() {
                latest = (tick.input, tick.telemetry);
            }
            let state = CarSoundState {
                car: "LANCEREVO8".into(),
                upgrades: Default::default(),
                input: latest.0,
                telemetry: latest.1,
                events: Vec::new(),
                scrape: None,
                surface: 0,
                ticks,
                view: crate::audio::AudioView {
                    speedbreaker: speedbreaker && (fps..fps * 4).contains(&frame),
                    ..Default::default()
                },
            };
            audio.drive_car(Some(&state), 1.0 / fps as f32);
            audio.trace.advance(1.0 / fps as f32);
            let count = (frame + 1) * 48000 / fps - frame * 48000 / fps;
            for sample in audio.render_replay(count) {
                assert!(sample.is_finite());
                peak = peak.max(sample.abs());
            }
        }
        audio.drive_car(None, 0.0);
        audio.trace.stop();
        let trace = std::fs::read_to_string(&path).unwrap();
        if speedbreaker {
            assert!(trace.contains("\"class\":\"SPEEDBREAKER_AUDIO\""));
            assert!(
                trace
                    .lines()
                    .filter(
                        |line| line.contains("\"class\":\"FX_NITROUS\"") && line.contains("\"event\":\"module_tick\"")
                    )
                    .filter_map(|line| line
                        .split("\"params\":[")
                        .nth(1)?
                        .split(']')
                        .next()?
                        .split(',')
                        .nth(4)?
                        .trim()
                        .parse::<i32>()
                        .ok())
                    .any(|pitch| pitch > 0 && pitch < 4096),
                "authored pitch must reach the module at {fps} FPS"
            );
        }
        for class in ["FX_NITROUS", "FX_PURGE", "FX_SHIFTING_01", "CAR_SWTN", "FX_TURBO_01", "CAR_WHINE"] {
            assert!(
                trace.lines().any(|line| line.contains("\"event\":\"voice_start\"")
                    && line.contains(&format!("\"class\":\"{class}\""))),
                "{class} must actually start at {fps} FPS"
            );
            assert!(
                !trace.lines().any(|line| line.contains("\"event\":\"fallback_command\"")
                    && line.contains(&format!("\"class\":\"{class}\""))),
                "duplicate fallback for {class} at {fps} FPS"
            );
        }
        assert!(trace.contains("\"event\":\"session_end\""));
        assert!(trace.contains("\"dropped\":0"));
        assert!(!trace.contains("\"event\":\"voice_failure\""));
        assert!(peak > 0.001);
        assert!(audio.missing.is_empty());
        if evidence.is_none() {
            std::fs::remove_file(path).unwrap();
        }
    }
}
