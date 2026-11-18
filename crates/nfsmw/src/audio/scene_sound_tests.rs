//! A movie's sound stops with its movie (run out, skipped, stopped), and a scene change takes every scene-owned stream
//! along. Headless: the real mixer renders into memory, nothing reaches a device.
use super::super::nis::NisAudioRequest;
use super::super::{Audio, Group};
use crate::viewer::Scene;
use ea_audio::Pcm;
use game_install::GameDir;

/// Ten seconds of a 440 Hz tone at 22050 Hz, mono: a stand-in for a movie's sound.
fn tone() -> Pcm {
    let rate = 22_050;
    let samples = (0..rate * 10)
        .map(|i| ((i as f32 * 440.0 * std::f32::consts::TAU / rate as f32).sin() * 12_000.0) as i16)
        .collect();
    Pcm { sample_rate: rate, channels: 1, samples, loop_range: None }
}

/// A scene that plays a movie: it hands its sound over once and says whether the movie still plays.
#[derive(Default)]
pub(crate) struct MovieStandIn {
    pub clip: Option<Pcm>,
    pub playing: bool,
    pub nis: Option<NisAudioRequest>,
}

impl Scene for MovieStandIn {
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
    fn take_clip(&mut self) -> Option<Pcm> {
        self.clip.take()
    }
    fn clip_playing(&self) -> bool {
        self.playing
    }
    fn nis_audio(&self) -> Option<&NisAudioRequest> {
        self.nis.as_ref()
    }
}

/// An audio over an empty folder with the headless mixer, and a trace file for it.
fn rig(name: &str) -> (Audio, std::path::PathBuf, tempdir::Dir) {
    let dir = tempdir::Dir::new(name);
    let audio = Audio::replay(GameDir::open(dir.path()).unwrap(), 48_000);
    let trace = dir.path().join("trace.jsonl");
    audio.trace.start(&trace).unwrap();
    (audio, trace, dir)
}

/// Renders `seconds` of output at 60 frames a second, following `scene` each frame as the plugin does. Returns the
/// loudest sample of the last frame.
fn run(audio: &mut Audio, scene: &mut MovieStandIn, seconds: f32) -> f32 {
    let mut last = 0.0;
    for _ in 0..(seconds * 60.0).round() as usize {
        audio.follow_clip(scene);
        audio.update_nis(scene.nis_audio());
        audio.trace.advance(1.0 / 60.0);
        last = audio.render_replay(800).iter().fold(0.0_f32, |m, s| m.max(s.abs()));
    }
    last
}

fn trace_lines(audio: &mut Audio, path: &std::path::Path) -> Vec<String> {
    audio.trace.stop();
    std::fs::read_to_string(path).unwrap().lines().map(str::to_owned).collect()
}

#[test]
fn a_skipped_movie_takes_its_sound_along_while_the_scene_stays() {
    let (mut audio, trace, _dir) = rig("skip");
    let mut scene = MovieStandIn { clip: Some(tone()), playing: true, nis: None };
    assert!(run(&mut audio, &mut scene, 2.0) > 0.05, "the movie's sound plays");
    assert_eq!(audio.active_streams(), ["movie"]);
    // Skipped at 2 s: the scene stays in the window (a world story movie), the movie is over.
    scene.playing = false;
    run(&mut audio, &mut scene, 0.1);
    assert!(audio.active_streams().is_empty(), "left playing: {:?}", audio.active_streams());
    assert!(run(&mut audio, &mut scene, 0.5) < 0.01, "nothing is heard after the skip");
    let lines = trace_lines(&mut audio, &trace);
    let start = lines.iter().find(|l| l.contains("\"stream_start\"") && l.contains("\"movie\"")).expect("start");
    let stop = lines.iter().find(|l| l.contains("\"stream_stop\"") && l.contains("movie over")).expect("stop");
    let instance = |line: &str| line.split("\"instance\":").nth(1).unwrap().split(',').next().unwrap().to_owned();
    assert_eq!(instance(start), instance(stop), "the stop names the stream it stops");
}

#[test]
fn a_movie_that_runs_out_is_recorded_as_ended() {
    let (mut audio, trace, _dir) = rig("end");
    let mut short = tone();
    short.samples.truncate(22_050);
    let mut scene = MovieStandIn { clip: Some(short), playing: true, nis: None };
    run(&mut audio, &mut scene, 1.5);
    scene.playing = false;
    run(&mut audio, &mut scene, 0.1);
    let lines = trace_lines(&mut audio, &trace);
    assert!(lines.iter().any(|l| l.contains("\"stream_end\"") && l.contains("movie over")), "{lines:#?}");
}

#[test]
fn a_scene_change_stops_the_movie_and_the_nis_take() {
    let (mut audio, trace, _dir) = rig("change");
    let mut scene = MovieStandIn { clip: Some(tone()), playing: true, nis: None };
    run(&mut audio, &mut scene, 0.5);
    let nis = audio.play(Group::Sfx, super::super::pcm::sound(&tone())).unwrap();
    audio.nis.set_voice(nis);
    run(&mut audio, &mut scene, 0.1);
    assert_eq!(audio.active_streams(), ["movie", "nis ?"]);
    assert!(audio.follow_scene_change(1), "the host's count moved on");
    assert!(!audio.follow_scene_change(1), "the same change is followed once");
    run(&mut audio, &mut MovieStandIn::default(), 0.1);
    assert!(audio.active_streams().is_empty(), "left playing: {:?}", audio.active_streams());
    let lines = trace_lines(&mut audio, &trace);
    assert!(
        lines.iter().any(|l| l.contains("\"stream_stop\"") && l.contains("scene changed") && l.contains("\"movie\""))
    );
    assert!(
        lines.iter().any(|l| l.contains("\"stream_stop\"") && l.contains("scene changed") && l.contains("\"nis\""))
    );
}

#[test]
fn a_nis_take_stops_when_the_scene_no_longer_asks_for_it() {
    let (mut audio, trace, _dir) = rig("nis");
    let request = NisAudioRequest { scene: Some("IntroNis01".into()), countdown: false, serial: 1 };
    let mut scene = MovieStandIn { nis: Some(request), ..MovieStandIn::default() };
    run(&mut audio, &mut scene, 0.1);
    // The empty folder has no NIS stream; stand in for the take that would have started.
    let voice = audio.play(Group::Sfx, super::super::pcm::sound(&tone())).unwrap();
    audio.nis.set_voice(voice);
    run(&mut audio, &mut scene, 1.9);
    assert!(audio.nis_playing());
    // Skipped at 2 s: the player hands back no request.
    scene.nis = None;
    assert!(run(&mut audio, &mut scene, 0.5) < 0.01, "nothing is heard after the skip");
    assert!(audio.active_streams().is_empty(), "left playing: {:?}", audio.active_streams());
    let lines = trace_lines(&mut audio, &trace);
    assert!(lines.iter().any(|l| l.contains("\"stream_stop\"") && l.contains("request changed")), "{lines:#?}");
}

#[test]
fn a_nis_skip_is_taken_once() {
    super::super::nis::note_skip();
    super::super::nis::note_skip();
    let (mut audio, _trace, _dir) = rig("skip-once");
    audio.follow_nis_skip();
    assert!(!super::super::nis::skip_pending(), "the skip was taken");
}

/// A folder under the temp directory, removed when dropped.
mod tempdir {
    pub struct Dir(std::path::PathBuf);
    impl Dir {
        pub fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!("nfsmw-scene-sound-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        pub fn path(&self) -> &std::path::Path {
            &self.0
        }
    }
    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}
