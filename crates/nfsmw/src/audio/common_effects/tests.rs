use super::*;

#[test]
fn common_event_ids_inputs_and_class_words_are_distinct() {
    for (event, input, id) in
        [(CommonEvent::VisualPulse, 1, 0), (CommonEvent::SpeedbreakerBegin, 3, 2), (CommonEvent::SpeedbreakerEnd, 4, 1)]
    {
        assert_eq!(event.input(), input);
        assert_eq!(event.words(20000), [id, 20000, 0, 0, 0, 0]);
        assert_eq!(event.words(-1)[1], 0);
        assert_eq!(event.words(50000)[1], 32767);
    }
}

fn installed_audio(rate: u32) -> Audio {
    let dir = game_install::GameDir::open(std::path::PathBuf::from(
        std::env::var_os("NFSMW_GAME_DIR").expect("common audio tests require NFSMW_GAME_DIR"),
    ))
    .unwrap();
    Audio::replay(dir, rate)
}

#[test]
#[ignore = "needs a game install; set NFSMW_GAME_DIR"]
fn installed_common_events_play_release_and_match_across_render_rates() {
    for event in [CommonEvent::VisualPulse, CommonEvent::SpeedbreakerBegin, CommonEvent::SpeedbreakerEnd] {
        for rate in [44100usize, 48000] {
            let mut energies = Vec::new();
            for fps in [5usize, 60, 144, 360] {
                let mut audio = installed_audio(rate as u32);
                let mut common = CommonPlayback::default();
                assert!(common.trigger(&mut audio, event));
                let mut peak = 0.0f32;
                let mut energy = 0.0f64;
                let mut started = 0;
                for frame in 0..fps * 8 {
                    common.update(&mut audio, 1.0 / fps as f32);
                    for running in common.active.iter().flatten() {
                        started = started.max(running.module.voice_counts().0);
                    }
                    let first = frame * rate / fps;
                    let last = (frame + 1) * rate / fps;
                    for sample in audio.render_replay(last - first) {
                        assert!(sample.is_finite());
                        peak = peak.max(sample.abs());
                        energy += f64::from(*sample).powi(2);
                    }
                }
                assert!(started > 0 && peak > 0.0001, "{event:?} {rate}Hz {fps}FPS produced no authored sound");
                assert!(common.active.iter().all(Option::is_none), "{event:?} did not release");
                energies.push(energy);
            }
            let minimum = energies.iter().copied().fold(f64::INFINITY, f64::min);
            let maximum = energies.iter().copied().fold(0.0f64, f64::max);
            assert!(maximum / minimum < 1.05, "{event:?} {rate}Hz: {energies:?}");
        }
    }
}
