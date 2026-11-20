//! AUDIO-030: the movie mute of the front-end cues follows a playing movie only. A scene that has no movie (the main
//! menu's backdrop, the garage) used to count as one, and every menu cue of a real session was dropped.
use super::super::Audio;
use super::{CueNote, note_suppressed, request_cue};
use crate::viewer::Scene;
use game_install::GameDir;

/// A scene with every optional answer left at the trait's default, as the menu and garage scenes leave the movie ones.
struct NoMovie;

impl Scene for NoMovie {
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
}

fn note(gate: Option<&str>) -> CueNote {
    CueNote {
        screen: "MainMenu.fng".into(),
        message: 0x4AC5_E165,
        source: "authored",
        candidate: Some(2),
        gate: gate.map(str::to_owned),
    }
}

/// Plays one accepted and one suppressed cue with the presentation state `movie` and returns the `fe_cue` lines.
fn fe_cues(name: &str, movie: bool) -> Vec<String> {
    let dir = std::env::temp_dir().join(format!("nfsmw-movie-gate-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut audio = Audio::replay(GameDir::open(&dir).unwrap(), 48_000);
    let trace = dir.join("trace.jsonl");
    audio.trace.start(&trace).unwrap();
    audio.set_presentation_mix(false, movie);
    request_cue(2, note(None));
    note_suppressed(note(Some("screen gate: IconMenu")));
    audio.update_menu(1.0 / 60.0);
    audio.trace.stop();
    let lines = std::fs::read_to_string(&trace).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    lines.lines().filter(|line| line.contains("\"fe_cue\"")).map(str::to_owned).collect()
}

#[test]
fn a_scene_without_a_movie_reports_no_playing_movie() {
    assert!(!NoMovie.clip_playing(), "the default must not mute the front-end cues");
}

#[test]
fn menu_cues_of_a_scene_without_a_movie_are_not_dropped() {
    let lines = fe_cues("menu", NoMovie.clip_playing());
    assert_eq!(lines.len(), 2, "{lines:?}");
    let played = &lines[0];
    assert!(played.contains("result=played"), "{played}");
    assert!(!played.contains("dropped: movie playing"), "{played}");
    assert!(played.contains("movie_active=false") && played.contains("effective=1.000"), "{played}");
    assert!(played.contains("screen=MainMenu.fng message=4AC5E165 source=authored candidate=2"), "{played}");
    let suppressed = &lines[1];
    assert!(suppressed.contains("result=suppressed gate=\\\"screen gate: IconMenu\\\""), "{suppressed}");
}

#[test]
fn a_playing_movie_still_mutes_the_front_end_cues() {
    let lines = fe_cues("movie", true);
    assert!(lines[0].contains("dropped: movie playing") && lines[0].contains("effective=0.000"), "{lines:?}");
}
