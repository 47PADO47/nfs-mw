use super::*;
use crate::audio::{
    AudioView,
    mixer::{CarMixer, Frame},
};
use blackbox_carsound::{CarInput, EngineOutput};
use blackbox_mixmap::ObjectRef;

#[test]
fn absent_map_starts_with_neutral_pitch() {
    assert_eq!(Slot::default().output(), Some(EventOutput::default()));
}

#[test]
#[ignore = "requires NFSMW_GAME_DIR; installed Speedbreaker mixer routes"]
fn installed_speedbreaker_pitch_uses_real_time_and_preserves_engine_routing() {
    let install = std::env::var_os("NFSMW_GAME_DIR").expect("installed map required");
    let dir = game_install::GameDir::open(std::path::PathBuf::from(install)).unwrap();
    let map = Arc::new(MixMap::parse(&dir.read(MAP).unwrap()).unwrap());
    let event = &map.state(0).unwrap().events[2];
    assert!(event.swing < 0);
    let input = CarInput { rpm_pct: 0.8, throttle: 1.0, speed: 30.0, ..Default::default() };
    let engine = EngineOutput::default();
    let frame =
        Frame { input: &input, engine: &engine, effects: Default::default(), master_volume: 32767, sparks: false };
    for rate in [44100, 48000] {
        for fps in [5, 30, 60, 120, 144, 240, 360] {
            let mut audio = Audio::replay(dir.clone(), rate);
            let mut normal = CarMixer::new(map.clone(), true);
            let normal_levels = normal.update(1.0, &frame);
            for index in 0..fps * 3 {
                let active = index < fps * 2;
                audio.update_speedbreaker(active, 1.0 / fps as f32);
                let output = audio.speedbreaker.output().unwrap();
                let before = output;
                audio.update_speedbreaker(!active, 0.0);
                assert_eq!(audio.speedbreaker.output(), Some(before));
                if index == fps * 2 - 1 {
                    assert_eq!(output.level, event.swing, "full hold at {fps} FPS");
                    let view = AudioView { speedbreaker: true, speedbreaker_pitch: Some(output), ..Default::default() };
                    // Fresh sources must inherit the held pitch, despite a quarter-speed local clock.
                    let mut car = CarMixer::new(map.clone(), true);
                    car.set_view(view);
                    let levels = car.update(1.0 / 240.0, &frame);
                    assert!(levels.nitrous_pitch < normal_levels.nitrous_pitch);
                    assert_eq!(levels.engine_pitch, normal_levels.engine_pitch, "Ginsu is not globally multiplied");
                    let mut traffic = Mixer::new(map.clone(), &[1, 0, 1, 0, 0, 1]);
                    let object = ObjectRef { state: 5, instance: 0, object: 0 };
                    traffic.attach(object, true);
                    assert!(traffic.override_event(0, 0, 2, Some(output)));
                    traffic.process(0.0);
                    assert!(traffic.pitch(object, 4).unwrap() < 1.0);
                }
            }
            assert_eq!(audio.speedbreaker.output().unwrap().level, 0, "release at {fps} FPS");
            audio.update_speedbreaker(true, 1.0); // A stalled frame must retain the endpoint.
            assert_eq!(audio.speedbreaker.output().unwrap().level, event.swing);
            audio.update_speedbreaker(false, 1.0);
            audio.update_speedbreaker(false, 1.0);
            assert_eq!(audio.speedbreaker.output().unwrap().level, 0);
        }
    }
}
