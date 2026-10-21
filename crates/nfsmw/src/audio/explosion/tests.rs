use super::*;

#[test]
fn source_position_publishes_distances_azimuths_and_validity() {
    let point = Vec3::new(12.0, 24.0, 6.0);
    let view = AudioView { car_position: Vec3::new(12.0, 20.0, 1.0), ..Default::default() };
    let distances = view.distances(point);
    assert_eq!(distances[0], 400);
    assert_ne!(distances[1], view.distances(view.car_position)[1]);
    assert_ne!(distances[2], 0);
}

#[test]
#[ignore = "requires NFSMW_GAME_DIR; original gas-station recording and authored spatial mixer"]
fn installed_petrol_explosion_selects_original_stream_positions_and_stops() {
    let dir = game_install::GameDir::open(std::path::PathBuf::from(
        std::env::var_os("NFSMW_GAME_DIR").expect("installed streams required"),
    ))
    .unwrap();
    for rate in [44_100, 48_000] {
        let mut audio = Audio::replay(dir.clone(), rate);
        audio.prepare_explosion();
        let recording = audio.explosion.recording.as_ref().expect("actual gas-station moment must resolve");
        assert_eq!((recording.definition.stream, recording.definition.param), (32, 16));
        assert_eq!(recording.definition.volume_slot, 3);
        assert!(recording.definition.positioned);
        assert_eq!((recording.bank, recording.take), (3, 29));
        assert!(recording.data.frames.len() > rate as usize);
        assert!(recording.data.frames.iter().any(|frame| frame.left.abs() + frame.right.abs() > 0.01));
        assert!(recording.data.settings.loop_region.is_none());
        let definition = recording.definition;
        let mixer = audio.explosion.mixer.as_mut().unwrap();
        let view = AudioView::default();
        let point = Vec3::new(0.0, 5.0, 0.0);
        publish(mixer, point, view);
        mixer.process(1.0 / 60.0);
        let near = levels(mixer, definition).unwrap();
        assert!(near.0 > 0.0 && near.0 <= 1.0, "authored nearby explosion gain: {near:?}");
        assert!(near.1.abs() > 0.1, "positioned azimuth follows blast point: {near:?}");
        assert_eq!(mixer.input(InputKey::controller(0, 0, 2, 0)), 500);
        publish(mixer, Vec3::new(0.0, 110.0, 0.0), view);
        mixer.process(1.0 / 60.0);
        let far = levels(mixer, definition).unwrap();
        assert!(far.0 <= near.0, "authored distance rolloff: near={near:?}, far={far:?}");
        audio.play_explosion(point, view);
        assert!(audio.explosion.voice.is_some());
        audio.play_explosion(Vec3::X, view);
        assert_eq!(audio.explosion.point, point, "chained blasts cannot restart the current recording");
        let mut energy = 0.0;
        for _ in 0..30 {
            audio.update_explosion(view, 1.0 / 60.0);
            energy += audio.render_replay((rate / 60) as usize).iter().map(|x| f64::from(*x).powi(2)).sum::<f64>();
        }
        assert!(energy > 0.1, "installed blast recording must reach actual replay PCM at {rate}");
        assert_eq!(audio.explosion.point, point);
        audio.stop_explosion();
        audio.render_replay(2048);
        assert!(audio.render_replay(1024).iter().all(|sample| sample.abs() < 1e-6), "scene teardown stops audio");
        audio.play_explosion(Vec3::new(0.0, 121.0, 0.0), view);
        assert!(audio.explosion.voice.is_none(), "distant blasts do not restart");
        audio.play_explosion(Vec3::X, view);
        assert!(audio.explosion.voice.is_some(), "new scene event can play after a stop");
        audio.update_explosion(AudioView { car_position: Vec3::splat(500.0), ..view }, 1.0 / 60.0);
        assert!(audio.explosion.voice.is_none(), "moving beyond native range purges the voice");
    }
}
