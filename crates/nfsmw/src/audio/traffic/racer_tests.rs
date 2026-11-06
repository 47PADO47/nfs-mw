//! Race opponents use their own engine voices and never the traffic horn, engine or whoosh producers.
use super::racer::{CONNECT_DISTANCE, EngineData, MAX_VOICES, MixOut, car_words, connected};
use super::*;

fn racer(id: u32, position: Vec3) -> RacerEmitter {
    RacerEmitter {
        id,
        car: "carreragt".into(),
        position,
        forward: Vec3::X,
        velocity: Vec3::X * 30.0,
        rpm_pct: 0.6,
        throttle: 1.0,
        wheels_on_ground: 4,
    }
}

#[test]
fn at_most_four_nearest_racers_within_the_connect_distance_are_heard() {
    let racers: Vec<RacerEmitter> = [5.0, 300.0, 40.0, 10.0, 134.0, 20.0, 136.0]
        .iter()
        .enumerate()
        .map(|(i, &x)| racer(i as u32, Vec3::X * x))
        .collect();
    assert_eq!(connected(&racers, Vec3::ZERO), [0, 3, 5, 2]);
    assert_eq!(MAX_VOICES, 4);
    let far = [racer(9, Vec3::X * CONNECT_DISTANCE)];
    assert!(connected(&far, Vec3::ZERO).is_empty());
    let mut broken = racer(1, Vec3::X);
    broken.position.x = f32::NAN;
    assert!(connected(&[broken], Vec3::ZERO).is_empty());
}

#[test]
fn racer_engine_words_map_sound_rpm_onto_the_engine_range() {
    let data = EngineData { car_id: 66, min_rpm: 800.0, max_rpm: 8000.0, master_volume: 32767 };
    let mix = MixOut { volume: 0.5, pitch: 1.0, azimuth: 1234, low_pass: 30000 };
    let words = car_words(&data, 0.5, 100.0, mix, 500);
    assert_eq!(words[0], 66);
    assert_eq!(words[1], 4400, "half the sound scale lands half way through MinRPM..MaxRPM");
    assert_eq!(words[3], 1024);
    assert_eq!(words[4], words[5]);
    assert!((words[4] + 16384).abs() <= 2, "half volume attenuates by half: {}", words[4]);
    assert_eq!(words[15], 1234);
    assert_eq!(words[16], 500, "a unit mixer pitch keeps the voice's own offset");
    assert_eq!(words[21], 25000, "the distance low-pass is published, never left at 0 Hz");
    let silent = car_words(&data, 0.0, 0.0, MixOut { volume: 0.0, pitch: 2.0, azimuth: 0, low_pass: 725 }, -1500);
    assert_eq!(silent[1], 800);
    assert_eq!(silent[4], -0x7FFF);
    assert_eq!(silent[16], 13000);
}

#[test]
#[ignore = "requires NFSMW_GAME_DIR; installed AI-racer map state, engine banks and playback"]
fn installed_racers_play_engines_without_traffic_horns_or_whooshes() {
    let path = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR required");
    let dir = game_install::GameDir::open(std::path::PathBuf::from(&path)).unwrap();
    let mut audio = Audio::replay(dir, 48000);
    let trace = std::env::temp_dir().join(format!("nfsmw-racer-engines-{}.jsonl", std::process::id()));
    audio.trace.start(&trace).expect("racer audio trace");
    let cars = ["carreragt", "cts", "bmwm3gtre46", "gti", "supra", "911turbo"];
    let mut energy = 0.0f64;
    for tick in 0..960u32 {
        let t = tick as f32 / 120.0;
        let racers: Vec<RacerEmitter> = cars
            .iter()
            .enumerate()
            .map(|(i, car)| RacerEmitter {
                car: (*car).into(),
                rpm_pct: 0.3 + 0.6 * (t / 8.0),
                ..racer(100 + i as u32, Vec3::new(8.0 + i as f32 * 12.0 - t * 4.0, 3.0, 0.0))
            })
            .collect();
        let mut sounds = Vec::new();
        if tick == 0 {
            sounds.push(TrafficSound::NearMiss { emitter: 100, relative_speed: 30.0 });
        }
        sounds.push(TrafficSound::Frame(TrafficFrame {
            dt: 1.0 / 120.0,
            listener_position: Vec3::ZERO,
            listener_forward: Vec3::X,
            listener_velocity: Vec3::X * 25.0,
            emitters: Vec::new(),
            racers,
        }));
        audio.update_traffic(sounds, AudioView::default());
        assert!(audio.traffic.racer_voice_count() <= MAX_VOICES);
        let samples = audio.render_replay(400);
        assert!(samples.iter().all(|s| s.is_finite()));
        energy += samples.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>();
    }
    assert_eq!(audio.traffic.emitter_count(), 0, "racers never become traffic emitters");
    assert_eq!(audio.traffic.racer_voice_count(), MAX_VOICES);
    audio.trace.stop();
    let evidence = std::fs::read_to_string(&trace).expect("flushed racer trace");
    assert!(energy > 0.001, "racer engines must produce PCM, energy {energy}");
    std::fs::remove_file(&trace).expect("remove this test's trace");
    for forbidden in ["\"event\":\"traffic_horn\"", "\"event\":\"traffic_whoosh\"", "\"event\":\"traffic_source_tick\""]
    {
        assert!(!evidence.contains(forbidden), "racers must not reach {forbidden}");
    }
    let connects = evidence.lines().filter(|l| l.contains("\"event\":\"racer_engine_connect\"")).count();
    assert!(connects >= MAX_VOICES, "{connects} racer engine connections");
    assert!(
        evidence.lines().filter(|l| l.contains("racer_engine_connect")).all(|l| l.contains("\"success\":true")),
        "every racer engine module loads"
    );
    let empty = TrafficFrame {
        dt: 1.0 / 120.0,
        listener_position: Vec3::ZERO,
        listener_forward: Vec3::X,
        listener_velocity: Vec3::ZERO,
        emitters: Vec::new(),
        racers: Vec::new(),
    };
    audio.update_traffic(vec![TrafficSound::Frame(empty)], AudioView::default());
    assert_eq!(audio.traffic.racer_voice_count(), 0, "racers leaving release their voices");
    audio.render_replay(48000);
    assert!(audio.render_replay(480).iter().all(|s| s.abs() < 1e-6), "released racer engines fall silent");
}
