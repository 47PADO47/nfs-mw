//! With the install (`NFSMW_GAME_DIR`): a story movie the world plays and the player skips at 2 s leaves no sound
//! behind while the world stays in the window. Headless: the real mixer renders into memory.
use super::StoryMovie;
use crate::audio::Audio;
use crate::viewer::Scene;
use game_install::GameDir;
use std::time::{Duration, Instant};

/// The world's two movie answers to the audio, as `WorldScene` gives them (`scene.rs`).
struct World(StoryMovie);

impl Scene for World {
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
    fn take_clip(&mut self) -> Option<ea_audio::Pcm> {
        self.0.take_clip()
    }
    fn clip_playing(&self) -> bool {
        self.0.is_playing()
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn a_story_movie_skipped_at_two_seconds_leaves_no_sound() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR")).unwrap();
    let mut audio = Audio::replay(dir.clone(), 48_000);
    let mut world = World(StoryMovie::default());
    world.0.play(&dir, "storyfmv_rac01").unwrap();
    let opening = Instant::now();
    let mut heard_for = 0.0;
    // In real time, so the movie's workers keep up as in the game.
    while heard_for < 2.0 {
        world.0.update(false, 1.0 / 60.0);
        audio.follow_clip(&mut world);
        audio.render_replay(800);
        if audio.active_streams().iter().any(|s| s == "movie") {
            heard_for += 1.0 / 60.0;
        }
        assert!(opening.elapsed() < Duration::from_secs(60), "the movie's sound never started");
        std::thread::sleep(Duration::from_millis(16));
    }
    world.0.update(true, 1.0 / 60.0);
    assert!(!world.0.is_playing(), "the skip ends the movie");
    let mut peak = 0.0_f32;
    for frame in 0..30 {
        audio.follow_clip(&mut world);
        let block = audio.render_replay(800);
        if frame >= 5 {
            peak = block.iter().fold(peak, |m, s| m.max(s.abs()));
        }
    }
    assert!(audio.active_streams().is_empty(), "left playing: {:?}", audio.active_streams());
    assert!(peak < 0.01, "the movie is still heard after the skip: {peak}");
}
