use super::*;
use blackbox_mixmap::MixMap;
use std::sync::Arc;

#[test]
#[ignore = "requires NFSMW_GAME_DIR; original Speedbreaker moment recordings"]
fn installed_speedbreaker_moments_select_both_recordings_and_stop_at_all_fps() {
    let dir = game_install::GameDir::open(std::path::PathBuf::from(
        std::env::var_os("NFSMW_GAME_DIR").expect("installed streams required"),
    ))
    .unwrap();
    let map = MixMap::parse(&dir.read("SOUND/MIXMAPS/MAPOUTPUT.mxb").unwrap()).unwrap();
    let mut positioned = Mixer::new(Arc::new(map), &[1]);
    positioned.attach(OBJECT, true);
    positioned.process(0.0);
    assert_eq!(positioned.volume(OBJECT, 5), Some(0.0), "unset native position flag silences the master");
    feed_position(&mut positioned, crate::audio::AudioView::default());
    positioned.process(0.0);
    assert_eq!(positioned.input(InputKey::controller(0, 0, 2, 0)), 0, "moment follows player car");
    assert_eq!(positioned.input(InputKey::controller(0, 0, 2, 1)), 560, "gain keeps camera distance");
    assert_eq!(positioned.input(InputKey::controller(0, 0, 2, 15)), 1);
    assert!(positioned.volume(OBJECT, 5).unwrap() > 0.0, "native rolloff becomes audible with valid position");
    for rate in [44_100, 48_000] {
        for fps in [5, 60, 144, 360] {
            let mut audio = Audio::replay(dir.clone(), rate);
            let fx = audio.load_breaker_moment(false).unwrap();
            let music = audio.load_breaker_moment(true).unwrap();
            assert_eq!((fx.bank, fx.take, music.bank, music.take), (3, 44, 3, 45));
            assert!(fx.sound.frames.len() > rate as usize);
            assert!(music.sound.frames.len() > rate as usize);
            assert!(fx.sound.frames.iter().any(|f| f.left != 0.0 || f.right != 0.0));
            assert!(
                (fx.gain - music.gain).abs() < 1e-6 && fx.gain > 0.0 && fx.gain <= 1.0,
                "initial gains: fx={}, music={}",
                fx.gain,
                music.gain
            );
            audio.speedbreaker.moment.recordings = [Some(fx), Some(music)];
            let dt = 1.0 / fps as f32;
            let mut energy = 0.0;
            for _ in 0..fps {
                audio.update_speedbreaker(true, dt);
                energy += audio.render_replay((rate / fps) as usize).iter().map(|x| f64::from(*x).powi(2)).sum::<f64>();
            }
            assert!(energy > 0.1, "recording audible {rate}/{fps}");
            audio.update_speedbreaker(false, dt);
            audio.render_replay(2048);
            assert!(audio.render_replay(1024).iter().all(|x| x.abs() < 1e-6), "exit stops {rate}/{fps}");
            audio.update_speedbreaker(true, dt);
            assert!(audio.speedbreaker.moment.voice.is_some(), "retrigger after stopped-current cleanup {rate}/{fps}");
        }
    }
}
