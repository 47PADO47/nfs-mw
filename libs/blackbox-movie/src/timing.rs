//! Playback timing. Pure arithmetic: the caller owns the clock and passes elapsed seconds in, so the
//! same code runs against a wall clock, an audio clock or a test.

use std::ops::Range;

use crate::header::MovieHeader;

/// What to do with the video at one point in time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Update {
    /// The frame to put on screen now, or `None` if the frame already on screen is still current.
    pub present: Option<u32>,
    /// Frames whose time has passed without being shown. A VP6 decoder must still decode them, in
    /// order, because later frames predict from them; they are just not presented.
    pub skipped: Range<u32>,
    /// The movie is over: its last frame was presented or skipped and its duration has passed.
    pub finished: bool,
}

/// Maps elapsed time to video frame indices.
#[derive(Debug, Clone, PartialEq)]
pub struct Timeline {
    rate: u32,
    scale: u32,
    frame_count: u32,
    next: u32,
}

impl Timeline {
    /// A timeline of `frame_count` frames at `rate / scale` frames per second.
    /// A zero rate or scale gives an empty timeline.
    pub fn new(rate: u32, scale: u32, frame_count: u32) -> Self {
        let frame_count = if rate == 0 || scale == 0 { 0 } else { frame_count };
        Self { rate, scale, frame_count, next: 0 }
    }

    /// A timeline for a parsed header.
    pub fn from_header(header: &MovieHeader) -> Self {
        Self::new(header.rate, header.scale, header.frame_count)
    }

    /// Index of the first frame that has not been presented or skipped yet.
    pub fn next_frame(&self) -> u32 {
        self.next
    }

    /// Seconds from the start at which the given frame is due.
    pub fn frame_time(&self, index: u32) -> f64 {
        if self.rate == 0 {
            return 0.0;
        }
        f64::from(index) * f64::from(self.scale) / f64::from(self.rate)
    }

    /// Seconds from the start at which the next unshown frame is due, for sleeping until then.
    /// `None` when every frame has been handled.
    pub fn next_deadline(&self) -> Option<f64> {
        if self.next >= self.frame_count {
            return None;
        }
        Some(self.frame_time(self.next))
    }

    /// Index of the frame that is on screen at `elapsed` seconds, clamped to the last frame.
    /// `None` before the start (negative time) or for an empty movie.
    pub fn frame_at(&self, elapsed: f64) -> Option<u32> {
        if self.frame_count == 0 || elapsed.is_nan() || elapsed < 0.0 {
            return None;
        }
        let last = self.frame_count - 1;
        let index = (elapsed * f64::from(self.rate) / f64::from(self.scale)).floor();
        if index >= f64::from(last) {
            return Some(last);
        }
        Some(index as u32)
    }

    /// Whether `elapsed` is past the end of the last frame.
    pub fn is_finished_at(&self, elapsed: f64) -> bool {
        elapsed >= self.frame_time(self.frame_count)
    }

    /// Advances to `elapsed` seconds and says which frame to present and which to skip. Time only
    /// moves forward: a smaller `elapsed` than before presents nothing.
    pub fn update(&mut self, elapsed: f64) -> Update {
        let finished = self.is_finished_at(elapsed) && self.frame_count > 0;
        let Some(target) = self.frame_at(elapsed) else {
            return Update { present: None, skipped: self.next..self.next, finished: false };
        };
        if target < self.next {
            return Update { present: None, skipped: self.next..self.next, finished };
        }
        let skipped = self.next..target;
        self.next = target + 1;
        Update { present: Some(target), skipped, finished }
    }
}
