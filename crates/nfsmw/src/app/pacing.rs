//! Frame-rate cap (`--max-fps`) and the once-a-second window title.

use std::fmt;
use std::str::FromStr;
use std::time::{Duration, Instant};

use bevy_ecs::prelude::*;
use bevy_window::{PrimaryWindow, Window};

use super::host::Host;

/// A frame-rate cap: a number of frames per second, or `unlocked`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MaxFps(Option<u32>);

impl MaxFps {
    fn interval(self) -> Option<Duration> {
        self.0.map(|fps| Duration::from_secs_f64(1.0 / f64::from(fps)))
    }
}

impl FromStr for MaxFps {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "unlocked" | "off" | "0" => Ok(Self(None)),
            n => n
                .parse::<u32>()
                .map(|fps| Self(Some(fps)))
                .map_err(|_| format!("expected a frame rate or `unlocked`, got {s:?}")),
        }
    }
}

impl fmt::Display for MaxFps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(fps) => write!(f, "{fps}"),
            None => f.write_str("unlocked"),
        }
    }
}

/// Sleeps at the end of each frame so frames start at most `max` times per second.
pub struct FrameLimiter {
    interval: Option<Duration>,
    next: Instant,
}

impl FrameLimiter {
    pub fn new(max: MaxFps) -> Self {
        Self { interval: max.interval(), next: Instant::now() }
    }

    /// Call once per frame, after presenting.
    pub fn wait(&mut self) {
        let Some(interval) = self.interval else { return };
        let now = Instant::now();
        // Schedule from the previous deadline so the average rate is exact; after a slow frame,
        // start over from now instead of rushing to catch up.
        self.next = if self.next + interval < now { now } else { self.next + interval };
        if let Some(left) = self.next.checked_duration_since(now) {
            std::thread::sleep(left);
        }
    }
}

/// Last system of the frame: refresh the title every second, then sleep to the frame-rate cap.
pub fn end_of_frame(mut host: NonSendMut<Host>, mut window: Single<&mut Window, With<PrimaryWindow>>) {
    let host = &mut *host;
    let Some(renderer) = host.renderer.as_ref() else { return };
    let elapsed = host.title_timer.elapsed().as_secs_f32();
    if elapsed >= 1.0 {
        let status = host.scene.status().map(|s| format!(" - {s}")).unwrap_or_default();
        let fps = host.frames as f32 / elapsed;
        log::debug!("{fps:.1} fps");
        window.title = format!("{} - {} - {fps:.0} fps{status}", host.scene.title(), renderer.adapter_summary());
        (host.title_timer, host.frames) = (Instant::now(), 0);
    }
    host.limiter.wait();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        assert_eq!("60".parse::<MaxFps>().unwrap(), MaxFps(Some(60)));
        assert_eq!("Unlocked".parse::<MaxFps>().unwrap(), MaxFps(None));
        assert_eq!("0".parse::<MaxFps>().unwrap(), MaxFps(None));
        assert!("fast".parse::<MaxFps>().is_err());
        assert_eq!(MaxFps(Some(144)).to_string(), "144");
    }

    #[test]
    fn caps_the_rate() {
        let mut limiter = FrameLimiter::new(MaxFps(Some(200)));
        let start = Instant::now();
        for _ in 0..10 {
            limiter.wait();
        }
        assert!(start.elapsed() >= Duration::from_millis(45));
    }
}
