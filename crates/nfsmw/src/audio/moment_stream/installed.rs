//! The shared moment stream against the installed data: ranks, breaker markers, and what the original capture saw.
use super::super::{Audio, AudioView, WorldEffectAudio};
use super::{Owner, UNPAUSE};
use blackbox_attrib::vlt_hash;
use glam::Vec3;
use nfsmw_data::sound::CollisionZone;
use nfsmw_data::world::moment::QUEUED_MOMENTS;

const DT: f32 = 1.0 / 60.0;

fn installed() -> Audio {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
    Audio::replay(dir, 48_000)
}

/// Every queued moment exists, and the installed priorities order the moments the way the capture saw the original
/// arbitrate them.
#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_moment_ranks_match_the_captured_arbitration() {
    let mut audio = installed();
    let mut rank = |name: &str| audio.moment_rank(vlt_hash(name)).unwrap_or_else(|| panic!("{name} missing"));
    for (_, name) in QUEUED_MOMENTS {
        let found = rank(name);
        eprintln!("{name}: {found:?}");
    }
    let sting = rank("purs_break_start_fx");
    let car2car = rank("car2car");
    let logs = rank("logs_drop");
    let gate = rank("gate");
    let booth = rank("tollbooth");
    eprintln!("sting {sting:?} car2car {car2car:?} logs {logs:?} gate {gate:?} booth {booth:?}");
    assert!(sting.priority > car2car.priority, "the sting refused the car crashes after it");
    assert!(logs.priority >= sting.priority, "the log pile replaced the sting");
    assert!(gate.can_interrupt && gate.priority >= booth.priority, "the queued gate replaced a playing booth crash");
}

/// Coming near an installed gate marker queues the gate recording without playing it; the gate breaker's
/// `unpause` releases it at the marker; driving off afterwards does not queue it again while it plays.
#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_gate_marker_queues_its_moment_and_unpause_releases_it() {
    let mut audio = installed();
    let gate_key = vlt_hash("gate");
    audio.load_moment_markers();
    let markers = audio.moments.arbiter.markers().to_vec();
    assert!(markers.len() >= 10, "installed breaker markers: {}", markers.len());
    let gate = markers.iter().find(|(_, key)| *key == gate_key).expect("an installed gate marker").0;
    let near = gate + Vec3::new(15.0, -8.0, 0.0);
    let view = AudioView { car_position: near, camera_position: near + Vec3::Z * 2.0, ..AudioView::default() };
    let far = AudioView { car_position: gate + Vec3::new(60.0, 60.0, 0.0), ..view };
    audio.update_moment_stream(far, DT);
    assert_eq!(audio.moments.arbiter.current(), None, "nothing queued far from every marker");
    audio.update_moment_stream(view, DT);
    assert_eq!(audio.moments.arbiter.current(), Some(gate_key));
    assert!(audio.moments.arbiter.is_held());
    assert!(!audio.world_effects.sweetener.is_playing(), "a queued moment is silent");
    let unpause = WorldEffectAudio {
        effect: 0,
        zone: CollisionZone::Default,
        position: gate + Vec3::new(6.0, 4.0, 1.5),
        magnitude: 1.0,
        moment: Some(UNPAUSE),
    };
    audio.update_world_effect_audio(vec![unpause], view, DT);
    assert_eq!(audio.world_effects.sweetener.playing(), Some(gate_key));
    assert_eq!(audio.moments.arbiter.owner(), Some(Owner::World));
    assert_eq!(audio.moments.arbiter.point(), Some(gate), "the released moment plays at its marker");
    audio.update_moment_stream(view, DT);
    assert_eq!(audio.moments.arbiter.current(), Some(gate_key), "it plays on");
    assert!(!audio.moments.arbiter.is_held());
    let energy: f64 =
        (0..30).map(|_| audio.render_replay(800).iter().map(|s| f64::from(*s).powi(2)).sum::<f64>()).sum();
    assert!(energy > 0.0, "the released gate recording reaches replay PCM");
    audio.stop_world_effect_audio();
    audio.stop_moment_stream();
    assert_eq!(audio.moments.arbiter.current(), None);
}

/// One stream: the breaker sting refuses a car crash sweetener, a gas-station blast of higher or equal priority
/// replaces a car crash, and nothing else is left playing beside it.
#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_moments_share_one_stream() {
    let mut audio = installed();
    let car2car = vlt_hash("car2car");
    let db = audio.database().unwrap();
    let impacts = nfsmw_data::sound::all_impacts(&db);
    drop(db);
    let crash = impacts
        .iter()
        .find(|s| s.has("TWO_CAR") && s.sweeteners.iter().any(|w| w.moment == car2car && w.threshold <= 115))
        .expect("an installed car-to-car impact with a sweetener");
    let view = AudioView::default();
    let point = Vec3::X * 6.0;
    audio.prepare_breaker_moments();
    audio.update_speedbreaker(true, DT);
    assert!(audio.breaker_moment_playing(), "the breaker sting plays");
    assert_eq!(audio.moments.arbiter.owner(), Some(Owner::Breaker));
    audio.play_impact_sweetener(crash, 0.9, point, view);
    assert!(!audio.world_effects.sweetener.is_playing(), "the sting refuses the car crash");
    assert!(audio.breaker_moment_playing());
    audio.update_speedbreaker(false, DT);
    assert_eq!(audio.moments.arbiter.current(), None, "the breaker's end frees the stream");
    audio.play_impact_sweetener(crash, 0.9, point, view);
    assert_eq!(audio.world_effects.sweetener.playing(), Some(car2car));
    let blast = audio.moment_rank(vlt_hash("expl_gas_station")).unwrap();
    let crash_rank = audio.moment_rank(car2car).unwrap();
    audio.play_explosion(point, view);
    match blast.priority >= crash_rank.priority {
        true => {
            assert!(audio.explosion_playing(), "the blast takes the stream");
            assert!(!audio.world_effects.sweetener.is_playing(), "the crash it replaced stops");
        }
        false => assert!(!audio.explosion_playing(), "a lower blast waits"),
    }
    let playing =
        [audio.explosion_playing(), audio.world_effects.sweetener.is_playing(), audio.breaker_moment_playing()];
    assert_eq!(playing.iter().filter(|p| **p).count(), 1, "one moment at a time: {playing:?}");
}

/// The game moment shots' sounds (`docs/specs/game-moment-camera.md` section 2) are installed moments that play at the
/// car through the world effects. The pursuit shot's recording opens with about 1.4 s of silence.
#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_game_moment_shot_sounds_play_at_the_car() {
    for name in ["spot_cop_camera", "jump_cam", "crash_cam"] {
        let mut audio = installed();
        let key = vlt_hash(name);
        let rank = audio.moment_rank(key);
        eprintln!("{name}: {rank:?}");
        assert!(rank.is_some(), "{name} is an installed moment");
        let car = Vec3::new(10.0, 0.0, 0.0);
        let view = AudioView { car_position: car, camera_position: car + Vec3::Z * 2.0, ..AudioView::default() };
        let sound = WorldEffectAudio {
            effect: 0,
            zone: CollisionZone::Default,
            position: car,
            magnitude: 1.0,
            moment: Some(key),
        };
        audio.update_world_effect_audio(vec![sound], view, DT);
        assert_eq!(audio.world_effects.sweetener.playing(), Some(key), "{name} plays");
        let blocks: Vec<f64> =
            (0..180).map(|_| audio.render_replay(800).iter().map(|s| f64::from(*s).powi(2)).sum::<f64>()).collect();
        let first = blocks.iter().position(|e| *e > 0.0);
        eprintln!("{name}: first sound after {:?} blocks of 1/60 s", first);
        assert!(first.is_some(), "{name} reaches the output");
    }
}
