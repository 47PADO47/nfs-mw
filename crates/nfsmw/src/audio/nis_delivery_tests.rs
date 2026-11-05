//! Countdown request switching, prefetch ownership and audible output through the play wrapper.
use super::*;
use crate::viewer::Scene;

struct Request(NisAudioRequest);

impl Scene for Request {
    fn title(&self) -> String {
        String::new()
    }
    fn init(&mut self, _: &mut dyn blackbox_gfx::RenderBackend) -> anyhow::Result<()> {
        Ok(())
    }
    fn update(&mut self, _: &mut dyn blackbox_gfx::RenderBackend, _: &crate::input::ActionState, _: f32) {}
    fn frame(&mut self, _: f32) -> (blackbox_gfx::FrameParams, &[blackbox_gfx::Instance]) {
        unreachable!()
    }
    fn nis_audio(&self) -> Option<&NisAudioRequest> {
        Some(&self.0)
    }
}

#[test]
#[ignore = "needs installed NIS stream; verifies production request switch and countdown output"]
fn installed_race_presentation_countdown_request_reaches_audible_output() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    for (scene, take) in [("IntroNis02", 11), ("IntroNis04", 15), ("IntroNis06", 19)] {
        let tables = tables(&dir).unwrap();
        let selected = nfsmw_data::sound::nis_countdown_take(&tables.index, &tables.sentences, scene).unwrap();
        assert_eq!(
            tables.index.banks()[selected.bank].header.event,
            0x4023,
            "race starts must use the native start bank, not arrest audio"
        );
        assert_eq!(selected.take, take, "{scene}: wrong countdown recording");
        let full: Slot = Arc::new(Mutex::new(Some(load(&dir, scene, false))));
        let countdown: Slot = Arc::new(Mutex::new(Some(load(&dir, scene, true))));
        *lock(&PREFETCHED) =
            Some(Prefetched { scene: scene.into(), full: Some(full), countdown: Some(countdown.clone()) });
        let mut audio = Audio::replay(dir.clone(), 48000);
        let full_request = NisAudioRequest { scene: Some(scene.into()), countdown: false, serial: 0 };
        audio.update_nis(Some(&full_request));
        audio.update_nis(Some(&full_request));
        let request = Request(NisAudioRequest { countdown: true, ..full_request });
        let wrapped = crate::frontend::Pausable::new(Box::new(request), Default::default());
        audio.update_nis(wrapped.nis_audio());
        assert!(
            Arc::ptr_eq(&audio.nis.pending.as_ref().unwrap().1, &countdown),
            "countdown must use its prefetched section"
        );
        audio.update_nis(wrapped.nis_audio());
        let mut peak = 0.0_f32;
        for _ in 0..180 {
            audio.update_nis(wrapped.nis_audio());
            for sample in audio.render_replay(800) {
                peak = peak.max(sample.abs());
            }
        }
        assert!(peak > 0.001, "{scene}: countdown window is silent: {peak}");
        let restart_take: Slot = Arc::new(Mutex::new(Some(load(&dir, scene, true))));
        *lock(&PREFETCHED) =
            Some(Prefetched { scene: scene.into(), full: None, countdown: Some(restart_take.clone()) });
        let restart = NisAudioRequest { scene: Some(scene.into()), countdown: true, serial: 1 };
        audio.update_nis(Some(&restart));
        assert!(Arc::ptr_eq(
            &audio.nis.pending.as_ref().expect("restart must replay the same countdown").1,
            &restart_take
        ));
        audio.update_nis(Some(&restart));
        assert!(audio.nis.voice.is_some(), "restarted countdown must reach the output");
    }
}
