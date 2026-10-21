//! The scene clock: when a scene starts and ends, in seconds, and the playback state of one run of it.
//! Timing: `docs/specs/nis-player.md`, section 0 item 5 and section 4.

use crate::Scene;

/// The frame range of a scene and the frame rate that converts it to seconds.
///
/// The frame rate is passed in by the caller: the files count frames at 30 per second, but the type does not assume it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneClock {
    start_frame: i32,
    vanish_frame: i32,
    fps: f32,
}

impl SceneClock {
    /// A clock for frames `start_frame` to `vanish_frame` at `fps` frames per second. `None` unless `fps` is a
    /// positive finite number.
    pub fn new(start_frame: i32, vanish_frame: i32, fps: f32) -> Option<Self> {
        if !(fps.is_finite() && fps > 0.0) {
            return None;
        }
        Some(Self { start_frame, vanish_frame, fps })
    }

    /// The clock of a scene's header frame range. `None` for the same reason as [`SceneClock::new`].
    pub fn from_scene(scene: &Scene, fps: f32) -> Option<Self> {
        Self::new(scene.start_frame, scene.vanish_frame, fps)
    }

    /// The first frame that plays.
    pub fn start_frame(&self) -> i32 {
        self.start_frame
    }

    /// The last frame.
    pub fn vanish_frame(&self) -> i32 {
        self.vanish_frame
    }

    /// Frames per second.
    pub fn fps(&self) -> f32 {
        self.fps
    }

    /// The scene time at which the scene starts, in seconds.
    pub fn start_secs(&self) -> f32 {
        self.start_frame as f32 / self.fps
    }

    /// The scene time at which the scene ends, in seconds.
    pub fn end_secs(&self) -> f32 {
        self.vanish_frame as f32 / self.fps
    }

    /// The length of the scene in seconds, zero when the range is empty or reversed.
    pub fn duration(&self) -> f32 {
        (self.end_secs() - self.start_secs()).max(0.0)
    }
}

/// Where a run of a scene has got to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlaybackState {
    /// The scene is running, at this scene time in seconds (counted from the clock's zero, not from its start).
    Playing {
        /// The scene time in seconds.
        scene_time: f32,
    },
    /// The scene has reached its end, by the clock or by a skip.
    Finished,
}

/// One run of a scene: a clock, whether the caller allows skipping it, and the time elapsed since it began.
///
/// The caller drives the time: [`Playback::advance`] adds the time of each frame. The action button and the rule for
/// which scenes allow skipping are the caller's; this type records the rule and performs the jump.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Playback {
    clock: SceneClock,
    can_skip: bool,
    elapsed: f32,
}

impl Playback {
    /// A run that has not started. `can_skip` is the caller's rule for this scene.
    pub fn new(clock: SceneClock, can_skip: bool) -> Self {
        Self { clock, can_skip, elapsed: 0.0 }
    }

    /// The clock of this run.
    pub fn clock(&self) -> SceneClock {
        self.clock
    }

    /// Whether the caller allows this run to be skipped.
    pub fn can_skip(&self) -> bool {
        self.can_skip
    }

    /// Adds `dt` seconds to the run and returns its state. A zero, negative or non-finite step changes nothing. The
    /// elapsed time stops at the scene's duration.
    pub fn advance(&mut self, dt: f32) -> PlaybackState {
        if dt > 0.0 && dt.is_finite() {
            self.elapsed = (self.elapsed + dt).min(self.clock.duration());
        }
        self.state()
    }

    /// Jumps the run to its last frame. Returns `false` and does nothing when skipping is not allowed or the run has
    /// already finished.
    pub fn skip(&mut self) -> bool {
        if !self.can_skip || self.is_finished() {
            return false;
        }
        self.elapsed = self.clock.duration();
        true
    }

    /// Seconds elapsed since the run began, never more than the scene's duration.
    pub fn elapsed(&self) -> f32 {
        self.elapsed
    }

    /// The scene time in seconds: the clock's start plus the elapsed time.
    pub fn scene_time(&self) -> f32 {
        self.clock.start_secs() + self.elapsed
    }

    /// Whether the run has reached its end.
    pub fn is_finished(&self) -> bool {
        self.elapsed >= self.clock.duration()
    }

    /// The current state, without advancing.
    pub fn state(&self) -> PlaybackState {
        if self.is_finished() {
            return PlaybackState::Finished;
        }
        PlaybackState::Playing { scene_time: self.scene_time() }
    }
}
