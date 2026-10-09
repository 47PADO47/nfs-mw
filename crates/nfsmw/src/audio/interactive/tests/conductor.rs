//! The conductor against a stage that records what it is asked: starts, steering, cross-fades, failures.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use super::super::conductor::{CROSSFADE, Conductor, FADE_IN, FADE_OUT, Stage, Voice};
use super::super::director::RESUME_DELAY;
use super::super::state::MusicState;

const DT: f32 = 0.1;

/// What happened, in order: `start 2 1.0`, `fade 2.5`, `drop`.
type Log = Rc<RefCell<Vec<String>>>;

struct FakeVoice {
    id: usize,
    log: Log,
    ended: Rc<Cell<bool>>,
}

impl Voice for FakeVoice {
    fn fade_out(&mut self, secs: f32) {
        self.log.borrow_mut().push(format!("fade {} {secs}", self.id));
    }

    fn finished(&self) -> bool {
        self.ended.get()
    }
}

impl Drop for FakeVoice {
    fn drop(&mut self) {
        self.log.borrow_mut().push(format!("drop {}", self.id));
    }
}

#[derive(Default)]
struct FakeStage {
    log: Log,
    started: usize,
    fail: Option<String>,
    /// The control value of the latest voice.
    control: Option<Arc<AtomicU8>>,
    /// Set to end the latest voice by itself.
    ended: Rc<Cell<bool>>,
}

impl Stage for FakeStage {
    fn start(&mut self, set: u8, control: Arc<AtomicU8>, fade_in: f32) -> Result<Box<dyn Voice>, String> {
        self.log.borrow_mut().push(format!("start {set} {fade_in}"));
        if let Some(e) = &self.fail {
            return Err(e.clone());
        }
        self.started += 1;
        self.control = Some(control);
        self.ended = Rc::new(Cell::new(false));
        Ok(Box::new(FakeVoice { id: self.started, log: self.log.clone(), ended: self.ended.clone() }))
    }
}

impl FakeStage {
    fn log(&self) -> Vec<String> {
        self.log.borrow().clone()
    }

    fn control(&self) -> u8 {
        self.control.as_ref().unwrap().load(Ordering::Relaxed)
    }
}

/// Run for `secs`; returns whether the songs were allowed on the last frame.
fn run(conductor: &mut Conductor, stage: &mut FakeStage, state: &MusicState, driving: bool, secs: f32) -> bool {
    let mut radio = conductor.update(stage, state, driving, 0.0);
    for _ in 0..(secs / DT).round() as usize {
        radio = conductor.update(stage, state, driving, DT);
    }
    radio
}

#[test]
fn nothing_starts_without_a_pursuit() {
    let (mut conductor, mut stage) = (Conductor::default(), FakeStage::default());
    assert!(run(&mut conductor, &mut stage, &MusicState::default(), true, 3.0));
    assert!(stage.log().is_empty());
    assert_eq!(conductor.live_set(), None);
}

#[test]
fn a_pursuit_starts_a_voice_and_takes_the_songs_off() {
    let (mut conductor, mut stage) = (Conductor::default(), FakeStage::default());
    let radio = run(&mut conductor, &mut stage, &MusicState::chased(2, 0.5), true, 1.0);
    assert!(!radio);
    assert_eq!(stage.log(), [format!("start 2 {FADE_IN}")]);
    assert_eq!(conductor.live_set(), Some(2));
    assert_eq!(stage.control(), 64);
}

#[test]
fn the_intensity_steers_the_voice_every_frame() {
    let (mut conductor, mut stage) = (Conductor::default(), FakeStage::default());
    run(&mut conductor, &mut stage, &MusicState::chased(1, 0.0), true, 1.0);
    assert_eq!(stage.control(), 0);
    run(&mut conductor, &mut stage, &MusicState::chased(1, 1.0), true, 0.5);
    let rising = stage.control();
    assert!(rising > 0 && rising < 127, "{rising}");
    run(&mut conductor, &mut stage, &MusicState::chased(1, 1.0), true, 5.0);
    assert_eq!(stage.control(), 127);
    assert_eq!(stage.started, 1, "steering does not restart the track");
}

#[test]
fn a_new_set_cross_fades_with_the_old_one() {
    let (mut conductor, mut stage) = (Conductor::default(), FakeStage::default());
    run(&mut conductor, &mut stage, &MusicState::chased(1, 0.5), true, 1.0);
    run(&mut conductor, &mut stage, &MusicState::chased(3, 0.5), true, 3.0);
    assert_eq!(conductor.live_set(), Some(3));
    assert_eq!(conductor.leaving(), 1, "the old set is still fading out");
    assert_eq!(
        stage.log(),
        [format!("start 1 {FADE_IN}"), format!("fade 1 {CROSSFADE}"), format!("start 3 {CROSSFADE}")]
    );
    // It is dropped when the fade is over, not before.
    run(&mut conductor, &mut stage, &MusicState::chased(3, 0.5), true, CROSSFADE);
    assert_eq!(conductor.leaving(), 0);
    assert_eq!(stage.log().last().unwrap(), "drop 1");
}

#[test]
fn the_end_of_the_pursuit_fades_the_music_out_and_the_songs_wait() {
    let (mut conductor, mut stage) = (Conductor::default(), FakeStage::default());
    run(&mut conductor, &mut stage, &MusicState::chased(4, 1.0), true, 1.0);
    let radio = run(&mut conductor, &mut stage, &MusicState::default(), true, 0.5);
    assert!(!radio);
    assert_eq!(conductor.live_set(), None);
    assert_eq!(conductor.leaving(), 1);
    assert_eq!(stage.log()[1], format!("fade 1 {FADE_OUT}"));

    run(&mut conductor, &mut stage, &MusicState::default(), true, FADE_OUT);
    assert_eq!(conductor.leaving(), 0);
    assert_eq!(stage.log().last().unwrap(), "drop 1");

    assert!(!run(&mut conductor, &mut stage, &MusicState::default(), true, RESUME_DELAY - FADE_OUT - 5.0));
    assert!(run(&mut conductor, &mut stage, &MusicState::default(), true, 6.0));
}

#[test]
fn leaving_the_game_fades_the_music_out_and_frees_the_songs() {
    let (mut conductor, mut stage) = (Conductor::default(), FakeStage::default());
    run(&mut conductor, &mut stage, &MusicState::chased(2, 0.5), true, 1.0);
    let radio = run(&mut conductor, &mut stage, &MusicState::chased(2, 0.5), false, 0.2);
    assert!(radio);
    assert_eq!(conductor.live_set(), None);
    assert_eq!(stage.log()[1], format!("fade 1 {FADE_OUT}"));
}

#[test]
fn a_set_that_cannot_start_leaves_the_songs_playing_and_is_not_retried() {
    let mut conductor = Conductor::default();
    let mut stage = FakeStage { fail: Some("no music file".into()), ..FakeStage::default() };
    let radio = run(&mut conductor, &mut stage, &MusicState::chased(2, 0.5), true, 5.0);
    assert!(radio, "silence is worse than the wrong music");
    assert_eq!(stage.log(), [format!("start 2 {FADE_IN}")], "one attempt, not one per frame");
    assert!(conductor.status(&MusicState::chased(2, 0.5), true).contains("no music file"));

    // The next pursuit tries again.
    run(&mut conductor, &mut stage, &MusicState::default(), true, 0.2);
    stage.fail = None;
    let radio = run(&mut conductor, &mut stage, &MusicState::chased(2, 0.5), true, 0.5);
    assert!(!radio);
    assert_eq!(conductor.live_set(), Some(2));
}

#[test]
fn a_track_that_ends_by_itself_is_dropped_and_not_restarted() {
    let (mut conductor, mut stage) = (Conductor::default(), FakeStage::default());
    run(&mut conductor, &mut stage, &MusicState::chased(1, 0.5), true, 1.0);
    stage.ended.set(true);
    let radio = run(&mut conductor, &mut stage, &MusicState::chased(1, 0.5), true, 2.0);
    assert!(radio);
    assert_eq!(conductor.live_set(), None);
    assert_eq!(stage.started, 1);
    assert_eq!(stage.log().last().unwrap(), "drop 1");
}

#[test]
fn the_status_line_tells_what_plays_and_what_is_asked() {
    let (mut conductor, mut stage) = (Conductor::default(), FakeStage::default());
    let idle = conductor.status(&MusicState::default(), false);
    assert!(idle.contains("no pursuit music") && idle.contains("not in a game"), "{idle}");
    let state = MusicState { racing: true, ..MusicState::chased(3, 0.5) };
    run(&mut conductor, &mut stage, &state, true, 1.0);
    let busy = conductor.status(&state, true);
    assert!(busy.contains("pursuit set 3 at control 64") && busy.contains("racing"), "{busy}");
    run(&mut conductor, &mut stage, &MusicState::default(), true, 1.0);
    assert!(conductor.status(&MusicState::default(), true).contains("the songs return in"));
}
