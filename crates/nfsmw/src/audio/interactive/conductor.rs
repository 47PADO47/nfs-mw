//! Carries the director's plan out: starts the pursuit voice, steers it, cross-fades between sets, and tells the
//! radio when it may play. The sound device is behind [`Stage`] and [`Voice`], so the logic runs in tests.

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use super::director::{Director, Plan};
use super::state::MusicState;

/// Seconds a pursuit voice takes to come in when it replaces the licensed songs (which stop at once) **[guess]**.
pub const FADE_IN: f32 = 1.0;
/// Seconds two pursuit sets overlap when the set changes **[guess]**.
pub const CROSSFADE: f32 = 2.5;
/// Seconds the pursuit music takes to leave when the pursuit ends **[guess]**.
pub const FADE_OUT: f32 = 3.0;

/// A playing pursuit track.
pub trait Voice {
    /// Fade to silence over `secs`; the voice may be dropped once that time has passed.
    fn fade_out(&mut self, secs: f32);
    /// The track ended by itself (the graph reached an end or the decoder failed).
    fn finished(&self) -> bool;
}

/// Where voices come from: the music mixer track and the music file.
pub trait Stage {
    /// Start the music of pursuit set `set`, steered by `control` (the graph's control value, which the voice
    /// reads whenever it picks its next bar), fading in over `fade_in` seconds.
    fn start(&mut self, set: u8, control: Arc<AtomicU8>, fade_in: f32) -> Result<Box<dyn Voice>, String>;
}

struct Live {
    set: u8,
    control: Arc<AtomicU8>,
    voice: Box<dyn Voice>,
}

/// A voice that is leaving; it is dropped when `left` runs out.
struct Fading {
    /// Held so that dropping the entry removes the voice.
    _voice: Box<dyn Voice>,
    left: f32,
}

#[derive(Default)]
pub struct Conductor {
    director: Director,
    live: Option<Live>,
    fading: Vec<Fading>,
    /// The set that could not start or ended, and why. Not retried until the pursuit is over.
    failed: Option<(u8, String)>,
}

impl Conductor {
    /// One frame. Returns whether the licensed songs may play.
    pub fn update(&mut self, stage: &mut dyn Stage, state: &MusicState, driving: bool, dt: f32) -> bool {
        let plan = self.director.update(state, driving, dt);
        self.fading.retain_mut(|f| {
            f.left -= dt;
            f.left > 0.0
        });
        self.check_live();
        self.apply(stage, &plan);
        // When the pursuit music cannot play the songs stay: silence is worse than the wrong music.
        plan.radio || (plan.pursuit.is_some() && self.live.is_none())
    }

    /// A voice whose track ended is dropped and its set is not restarted until the pursuit is over.
    fn check_live(&mut self) {
        if !self.live.as_ref().is_some_and(|l| l.voice.finished()) {
            return;
        }
        let Some(live) = self.live.take() else { return };
        log::warn!("music: the pursuit track of set {} ended", live.set);
        self.failed = Some((live.set, "the track ended".into()));
    }

    fn apply(&mut self, stage: &mut dyn Stage, plan: &Plan) {
        let Some(set) = plan.pursuit else {
            self.failed = None;
            self.release(FADE_OUT);
            return;
        };
        if let Some(live) = self.live.as_ref().filter(|l| l.set == set) {
            live.control.store(plan.control, Ordering::Relaxed);
            return;
        }
        if self.failed.as_ref().is_some_and(|(failed, _)| *failed == set) {
            return;
        }
        let replaced = self.live.is_some();
        self.release(CROSSFADE);
        let control = Arc::new(AtomicU8::new(plan.control));
        let fade = if replaced { CROSSFADE } else { FADE_IN };
        match stage.start(set, control.clone(), fade) {
            Ok(voice) => {
                log::info!("music: pursuit set {set} at control {}", plan.control);
                self.live = Some(Live { set, control, voice });
            }
            Err(e) => {
                log::warn!("music: pursuit set {set} cannot play: {e}");
                self.failed = Some((set, e));
            }
        }
    }

    /// Let the live voice go: it fades out over `secs` and is dropped afterwards.
    fn release(&mut self, secs: f32) {
        let Some(mut live) = self.live.take() else { return };
        live.voice.fade_out(secs);
        self.fading.push(Fading { _voice: live.voice, left: secs });
    }

    /// A line for the `music` console command.
    pub fn status(&self, state: &MusicState, driving: bool) -> String {
        let playing = match &self.live {
            Some(live) => format!("pursuit set {} at control {}", live.set, live.control.load(Ordering::Relaxed)),
            None => "no pursuit music".to_owned(),
        };
        let leaving = if self.fading.is_empty() { String::new() } else { format!(", {} leaving", self.fading.len()) };
        let radio = match self.director.resume_in() {
            Some(left) => format!("the songs return in {left:.0} s"),
            None => "the songs are free to play".to_owned(),
        };
        let wanted = match state.pursuit {
            Some(p) => format!("asked: set {} at intensity {:.2}", p.set, p.intensity),
            None => "asked: no pursuit".to_owned(),
        };
        let game = if driving { "" } else { " (not in a game: the music waits)" };
        let failed = self.failed.as_ref().map(|(set, e)| format!("\n  set {set} failed: {e}")).unwrap_or_default();
        let race = if state.racing { "; racing" } else { "" };
        format!("interactive music: {playing}{leaving}; {radio}; {wanted}{race}{game}{failed}")
    }

    #[cfg(test)]
    pub fn live_set(&self) -> Option<u8> {
        self.live.as_ref().map(|l| l.set)
    }

    #[cfg(test)]
    pub fn leaving(&self) -> usize {
        self.fading.len()
    }
}
