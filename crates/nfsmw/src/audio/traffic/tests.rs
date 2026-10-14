use super::*;
use emitter::{engine_params, horn_params};
use mixer::{Levels, Motion};
use selection::{Variants, horn_fade, horn_in_range};

fn emitter(position: Vec3) -> TrafficEmitter {
    TrafficEmitter {
        id: 77,
        profile: TrafficProfile { horn: Some(11), engine: Some(2), whoosh: Some(9) },
        position,
        forward: Vec3::X,
        velocity: Vec3::X * 20.0,
    }
}

#[test]
fn traffic_variants_cycle_inside_the_velocity_block_and_ignore_render_frames() {
    for fps in [5, 60, 144, 360] {
        let mut variants = Variants::default();
        let mut selected = Vec::new();
        let mut owed = 0.0f64;
        let mut tick = 0;
        for _ in 0..fps {
            owed += 120.0 / f64::from(fps);
            while owed + 1e-9 >= 1.0 {
                owed -= 1.0;
                if tick % 10 == 0 {
                    selected.push(variants.select(9, 50.0).unwrap());
                }
                tick += 1;
            }
        }
        assert_eq!(tick, 120);
        assert_eq!(selected, [51, 52, 53, 54, 55, 56, 51, 52, 53, 54, 55, 56]);
        assert_eq!(variants.select(9, 10.0), None);
        assert_eq!(variants.select(9, f32::NAN), None);
        assert_eq!(variants.select(99, 50.0), None);
    }
}

#[test]
fn moving_source_has_signed_relative_velocity_and_crossing_flags() {
    let view = AudioView { camera_position: Vec3::ZERO, ..Default::default() };
    let mut motion = Motion::default();
    let first = motion.inputs(&emitter(Vec3::X * 10.0), view, [Vec3::ZERO; 2]);
    assert_eq!(&first[13..], &[2000, 2000, 1]);
    let approach = motion.inputs(&emitter(Vec3::X * 8.0), view, [Vec3::ZERO; 2]);
    assert_eq!(&approach[13..15], &[-2000, -2000]);
    assert_eq!(approach[15] as u32, 0xc0000001);
    let passing = motion.inputs(&emitter(-Vec3::X * 9.0), view, [Vec3::ZERO; 2]);
    assert_eq!(&passing[13..15], &[2000, 2000]);
    assert_eq!(passing[15] as u32, 0xc0000001);
    // The established azimuth conversion reflects positive angles with ones complement.
    assert_eq!(passing[2], 32767);
}

#[test]
fn traffic_class_ids_are_not_raw_sample_indices_and_pitch_offset_is_q12() {
    let levels =
        Levels { engine: 9000, horn: 10000, whoosh: 2000, azimuth: [7000, 8000, 9000], pitch: [4200, 4096, 4500] };
    let words = engine_params(emitter(Vec3::ZERO), levels);
    assert_eq!(&words[..4], &[2, 9000, 104, 7000]);
    assert_eq!(&words[5..], &[25000, 0, 32767, 0]);
    let horn = horn_params(11, levels, 0.5);
    assert_eq!(horn, [11, 5000, 4500, 9000, 0, 25000, 0, 32767, 0]);
    assert_eq!(horn_params(999, levels, 1.0)[0], 50);
}

#[test]
fn autonomous_horn_uses_original_nondrag_range_direction_and_time_fade() {
    assert!(horn_in_range(Vec3::X * 15.0, 20.0, Vec3::ZERO, Vec3::X));
    assert!(horn_in_range(-Vec3::X * 9.0, 20.0, Vec3::ZERO, Vec3::X));
    assert!(!horn_in_range(-Vec3::X * 15.0, 20.0, Vec3::ZERO, Vec3::X));
    assert!(!horn_in_range(Vec3::Y * 10.0, 20.0, Vec3::ZERO, Vec3::X));
    assert!(!horn_in_range(Vec3::X * 15.0, 10.0, Vec3::ZERO, Vec3::X));
    assert!(!horn_in_range(Vec3::X * 21.0, 20.0, Vec3::ZERO, Vec3::X));
    assert_eq!(horn_fade(1.0, 1.2), 1.0);
    assert!((horn_fade(1.275, 1.2) - 0.5).abs() < 1e-5);
    assert_eq!(horn_fade(1.4, 1.2), 0.0);
}

#[test]
#[ignore = "requires NFSMW_GAME_DIR; authored traffic banks and playback"]
fn installed_traffic_graphs_select_voices_and_release_at_high_frame_rates() {
    let path = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR required");
    for fps in [5, 60, 144, 360] {
        let dir = game_install::GameDir::open(std::path::PathBuf::from(&path)).unwrap();
        let mut audio = Audio::replay(dir, 48000);
        let source = emitter(Vec3::X * 6.0);
        let frame = || TrafficFrame {
            dt: 1.0 / 120.0,
            listener_position: Vec3::ZERO,
            listener_forward: Vec3::X,
            listener_velocity: Vec3::X * 50.0,
            emitters: vec![source],
        };
        let mut owed = 0.0f64;
        let mut tick = 0;
        let mut energy = 0.0f64;
        for _ in 0..fps * 3 {
            owed += 120.0 / f64::from(fps);
            let mut sounds = Vec::new();
            while owed + 1e-9 >= 1.0 {
                owed -= 1.0;
                if tick == 0 {
                    sounds.push(TrafficSound::Horn { emitter: 77, duration: 1.2 });
                    sounds.push(TrafficSound::NearMiss { emitter: 77, relative_speed: 30.0 });
                }
                sounds.push(TrafficSound::Frame(frame()));
                tick += 1;
            }
            audio.update_traffic(sounds, AudioView::default());
            let samples = audio.render_replay((48000 / fps) as usize);
            assert!(samples.iter().all(|s| s.is_finite()));
            energy += samples.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>();
        }
        assert!(energy > 0.001, "authored traffic must produce PCM at {fps} FPS");
        assert_eq!(tick, 360);
        let mut empty = frame();
        empty.emitters.clear();
        audio.update_traffic(vec![TrafficSound::Frame(empty)], AudioView::default());
        assert_eq!(audio.traffic.emitter_count(), 0, "despawn cleans every source at {fps} FPS");
        audio.render_replay(48000);
        assert!(audio.render_replay(480).iter().all(|s| s.abs() < 1e-6), "despawned effects release");
    }
}
