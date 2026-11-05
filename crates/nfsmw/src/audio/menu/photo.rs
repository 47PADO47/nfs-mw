//! The photo entrance's authored camera cue, independent of menu acceptance sounds.
use std::cell::Cell;

use super::{Audio, EffectModule, Group, MenuPlayer, Running};

thread_local! {
    static PENDING: Cell<bool> = const { Cell::new(false) };
    static TRANSITION: Cell<bool> = const { Cell::new(false) };
}

pub fn request_photo_transition() {
    TRANSITION.with(|pending| pending.set(true));
}

pub(super) fn take_transition() -> bool {
    TRANSITION.with(|pending| pending.replace(false))
}

pub(super) fn start_transition(audio: &mut Audio) {
    let view = audio.speedbreaker.view;
    audio.trace.note("photo_transition_trigger", "game_camera");
    audio.update_world_effect_audio(
        vec![crate::audio::WorldEffectAudio {
            effect: 0,
            zone: nfsmw_data::sound::CollisionZone::default(),
            position: view.car_position,
            magnitude: 1.0,
            moment: Some(blackbox_attrib::vlt_hash("game_camera")),
        }],
        view,
        0.0,
    );
}

/// Queue misc sound 2 once for the photo entrance. The normal audio frame services it while paused.
pub fn request_photo_finish() {
    PENDING.with(|pending| pending.set(true));
}

pub(super) fn take() -> bool {
    PENDING.with(|pending| pending.replace(false))
}

impl MenuPlayer {
    pub(super) fn start_photo(&mut self, audio: &mut Audio) -> Result<(), String> {
        let bank = "IG_GLOBAL/FX_Camera.abk";
        let class = "FX_Camera";
        self.seed = self.seed.wrapping_add(1);
        let module = EffectModule::load(audio, bank, class, self.seed)?;
        let words = vec![0, 32767, 0, 0, 0, 0, 0, 0];
        audio.trace.record(crate::audio::trace::Record {
            event: "photo_finish_trigger",
            instance: module.trace_instance(),
            bank,
            class,
            params: &words,
            success: Some(true),
            logical_ms: audio.trace.now_ms(),
            ..Default::default()
        });
        self.active.push(Running { module, words, age: 0.0, released: false, group: Group::Sfx });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_entrance_request_is_consumed_once() {
        take();
        request_photo_finish();
        request_photo_finish();
        assert!(take());
        assert!(!take());
    }

    #[test]
    #[ignore = "needs installed FX_Camera bank; verifies positive output and completion"]
    fn installed_race_presentation_photo_camera_sound_plays_and_finishes() {
        let dir = game_install::GameDir::open(std::path::PathBuf::from(
            std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR"),
        ))
        .unwrap();
        for fps in [30, 60, 144, 360, 1000] {
            let mut audio = Audio::replay(dir.clone(), 48000);
            let mut player = MenuPlayer::default();
            take();
            request_photo_finish();
            player.service(&mut audio, 0.0);
            assert_eq!(player.active.len(), 1, "authored camera class must load");
            let mut peak = 0.0_f32;
            for frame in 0..fps * 5 {
                player.update(&mut audio, 1.0 / fps as f32);
                let samples = (frame + 1) * 48000 / fps - frame * 48000 / fps;
                for sample in audio.render_replay(samples) {
                    assert!(sample.is_finite());
                    peak = peak.max(sample.abs());
                }
            }
            assert!(peak > 0.0001, "{fps} FPS: no camera sound, peak={peak}");
            assert!(player.active.is_empty(), "{fps} FPS: camera sound did not complete");
        }
    }
}
