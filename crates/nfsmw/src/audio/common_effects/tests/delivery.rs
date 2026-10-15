use super::*;
use crate::viewer::Scene;
use std::sync::Mutex;

struct Events(Arc<Mutex<Vec<CommonEvent>>>);

impl Scene for Events {
    fn title(&self) -> String {
        String::new()
    }
    fn init(&mut self, _: &mut dyn blackbox_gfx::RenderBackend) -> anyhow::Result<()> {
        Ok(())
    }
    fn update(&mut self, _: &mut dyn blackbox_gfx::RenderBackend, _: &crate::input::ActionState, _: f32) {}
    fn frame(&mut self, _: f32) -> (blackbox_gfx::FrameParams, &[blackbox_gfx::Instance]) {
        unreachable!("audio delivery does not render a scene")
    }
    fn take_common_sounds(&mut self) -> Vec<CommonEvent> {
        std::mem::take(&mut *self.0.lock().unwrap())
    }
}

#[test]
#[ignore = "needs a game install; set NFSMW_GAME_DIR"]
fn installed_speedbreaker_transitions_survive_play_wrapper_and_produce_pcm() {
    let evidence = std::env::var_os("NFSMW_AUDIO_CAPTURE_DIR").map(std::path::PathBuf::from);
    if let Some(folder) = &evidence {
        std::fs::create_dir_all(folder).unwrap();
    }
    for rate in [44_100usize, 48_000] {
        for fps in [5usize, 60, 144, 360] {
            let mut audio = installed_audio(rate as u32);
            let queue = Arc::new(Mutex::new(Vec::new()));
            let mut scene =
                crate::frontend::Pausable::new(Box::new(Events(queue.clone())), crate::frontend::PauseFlag::default());
            let name = format!("speedbreaker-delivery-{}-{rate}-{fps}", std::process::id());
            let trace_path =
                evidence.as_ref().cloned().unwrap_or_else(std::env::temp_dir).join(format!("{name}.jsonl"));
            audio.trace.start(&trace_path).unwrap();
            for event in [CommonEvent::SpeedbreakerBegin, CommonEvent::SpeedbreakerEnd] {
                queue.lock().unwrap().push(event);
                let mut samples = Vec::with_capacity(rate * 8 * 2);
                for frame in 0..fps * 8 {
                    let events = scene.take_common_sounds();
                    if frame == 0 {
                        assert_eq!(events, [event], "normal play must forward {event:?}");
                    }
                    if frame != 0 {
                        assert!(events.is_empty(), "event queue must be consumed once");
                    }
                    audio.update_common(events, 1.0 / fps as f32);
                    let first = frame * rate / fps;
                    let last = (frame + 1) * rate / fps;
                    samples.extend_from_slice(audio.render_replay(last - first));
                    audio.trace.advance(1.0 / fps as f32);
                }
                let levels = crate::audio::capture::levels(&samples);
                assert!(levels.peak > 0.0001, "{event:?} {rate}Hz/{fps}FPS must be audible: {levels:?}");
                assert!(audio.common.active.iter().all(Option::is_none), "{event:?} graph must release");
                println!("{event:?} {rate}Hz/{fps}FPS {levels:?}");
                if let Some(folder) = &evidence {
                    crate::audio::capture::write_float(
                        &folder.join(format!("{name}-{event:?}.wav")),
                        &samples,
                        rate as u32,
                        2,
                    );
                }
            }
            audio.trace.stop();
            let trace = std::fs::read_to_string(&trace_path).unwrap();
            for event in ["SpeedbreakerBegin", "SpeedbreakerEnd"] {
                assert_eq!(trace.matches(&format!("\"detail\":\"{event}\"")).count(), 1);
            }
            for id in [1, 2] {
                assert!(
                    trace.lines().any(|line| {
                        line.contains("\"class\":\"FX_UVES\"")
                            && line.contains("\"event\":\"module_tick\"")
                            && line.contains(&format!("\"params\":[{id},"))
                    }),
                    "authored transition class ID{id} must be evaluated"
                );
            }
            assert!(trace.matches("\"event\":\"voice_start\"").count() >= 2);
            assert!(!trace.contains("\"event\":\"voice_failure\""));
            assert!(trace.contains("\"dropped\":0,\"write_failed\":false"));
            if evidence.is_none() {
                std::fs::remove_file(trace_path).unwrap();
            }
        }
    }
}
