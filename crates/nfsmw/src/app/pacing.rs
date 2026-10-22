//! Frame-rate cap (`--max-fps`) and the once-a-second window title.

use std::fmt;
use std::str::FromStr;
use std::time::{Duration, Instant};

use bevy_app::AppExit;
use bevy_ecs::prelude::*;
use bevy_window::{Monitor, OnMonitor, PrimaryMonitor, PrimaryWindow, Window};

use super::host::Host;
use crate::settings::Settings;

/// How long before the deadline the limiter stops sleeping and spins instead. The OS wakes a sleeping thread up to a
/// millisecond or more late (Windows especially), so the last stretch is waited out by polling the clock.
const SPIN_MARGIN: Duration = Duration::from_millis(1);

/// A frame-rate cap: a number of frames per second, or `unlocked`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MaxFps(Option<u32>);

impl MaxFps {
    pub fn microseconds(self) -> u32 {
        self.0.map_or(0, |fps| (1_000_000 / fps.max(1)).max(1))
    }

    fn interval(self) -> Option<Duration> {
        self.0.map(|fps| Duration::from_secs_f64(1.0 / f64::from(fps)))
    }

    /// The cap in frames per second, or `None` when unlocked.
    fn fps(self) -> Option<u32> {
        self.0
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
        sleep_until(self.next);
    }
}

/// Waits until `deadline`: sleeps while there is more than [`SPIN_MARGIN`] left, then spins on the clock for the rest, so
/// the OS timer's late wake-up does not push the frame past its deadline.
fn sleep_until(deadline: Instant) {
    let coarse = deadline.checked_duration_since(Instant::now()).and_then(|left| left.checked_sub(SPIN_MARGIN));
    if let Some(coarse) = coarse.filter(|coarse| !coarse.is_zero()) {
        std::thread::sleep(coarse);
    }
    while Instant::now() < deadline {
        std::hint::spin_loop();
    }
}

/// Whether the game's own limiter should pace the frame. Without vsync it always does. Under vsync the display already
/// paces the frames (and the Reflex cap is off, see `latency.rs`), so a cap at or above the refresh rate would pace
/// twice; only a cap below the refresh rate is kept. An unknown refresh rate keeps the limiter.
fn limiter_wanted(vsync: bool, cap: MaxFps, refresh_hz: Option<f32>) -> bool {
    if !vsync {
        return true;
    }
    let Some(fps) = cap.fps() else { return false };
    let Some(hz) = refresh_hz else { return true };
    // Half a hertz of slack: a 60 fps cap on a 59.94 Hz display is the display's rate, not a lower one.
    (fps as f32) < hz - 0.5
}

/// The refresh rate in hertz of the monitor the primary window is on, or of the primary monitor.
fn refresh_hz(
    on_monitor: &Query<&OnMonitor, With<PrimaryWindow>>,
    monitors: &Query<(Entity, &Monitor, Has<PrimaryMonitor>)>,
) -> Option<f32> {
    let current = on_monitor.single().ok().map(|on| on.0);
    let (_, monitor, _) = monitors
        .iter()
        .find(|(entity, _, _)| Some(*entity) == current)
        .or_else(|| monitors.iter().find(|(_, _, primary)| *primary))?;
    monitor.refresh_rate_millihertz.map(|mhz| mhz as f32 / 1000.0)
}

/// Last system of the frame: refresh the title every second, then sleep to the frame-rate cap.
pub fn end_of_frame(
    mut host: NonSendMut<Host>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
    mut exit: MessageWriter<AppExit>,
    settings: Res<Settings>,
    on_monitor: Query<&OnMonitor, With<PrimaryWindow>>,
    monitors: Query<(Entity, &Monitor, Has<PrimaryMonitor>)>,
) {
    let _trace = blackbox_trace::scope("Frame: title / benchmark / limiter");
    let host = &mut *host;
    let Some(renderer) = host.renderer.as_ref() else { return };
    let ready = host.scene.benchmark_ready();
    if let Some(report) = host.bench.as_mut().and_then(|bench| bench.frame(ready)) {
        report.lines(renderer.info().renderer).iter().for_each(|line| println!("{line}"));
        println!("bench: backend {}", renderer.info().summary());
        let performance = renderer.frame_performance();
        println!("bench: OS activity {:?}; GPU base render {:?} ms", performance.activity, performance.gpu_render_ms);
        println!("bench: presentation {:?}", renderer.latency_status());
        exit.write(AppExit::Success);
    }
    let elapsed = host.title_timer.elapsed().as_secs_f32();
    if elapsed >= 1.0 {
        let status = host.scene.status().map(|s| format!(" - {s}")).unwrap_or_default();
        let fps = host.frames as f32 / elapsed;
        log::debug!("{fps:.1} fps");
        window.title = format!("{} - {} - {fps:.0} fps{status}", host.scene.title(), renderer.info().summary());
        (host.title_timer, host.frames) = (Instant::now(), 0);
    }
    if host.latency_pacing {
        return;
    }
    // Only look the refresh rate up when vsync makes it matter.
    let refresh = settings.vsync.then(|| refresh_hz(&on_monitor, &monitors)).flatten();
    if !limiter_wanted(settings.vsync, settings.max_fps, refresh) {
        return;
    }
    let _trace = blackbox_trace::scope("Wait: game FPS limiter");
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

    #[test]
    fn never_returns_before_the_deadline() {
        let deadline = Instant::now() + Duration::from_millis(5);
        sleep_until(deadline);
        assert!(Instant::now() >= deadline);
    }

    #[test]
    fn vsync_hands_pacing_to_the_display_unless_the_cap_is_lower() {
        let cap = |fps| MaxFps(Some(fps));
        // No vsync: the limiter always paces.
        assert!(limiter_wanted(false, cap(60), Some(60.0)));
        assert!(limiter_wanted(false, MaxFps(None), Some(60.0)));
        // Vsync with a cap at or above the refresh rate: the display paces, the limiter stays out.
        assert!(!limiter_wanted(true, cap(60), Some(60.0)));
        assert!(!limiter_wanted(true, cap(60), Some(59.94)));
        assert!(!limiter_wanted(true, cap(144), Some(60.0)));
        assert!(!limiter_wanted(true, MaxFps(None), Some(60.0)));
        // Vsync with a cap below the refresh rate: the limiter keeps it.
        assert!(limiter_wanted(true, cap(30), Some(60.0)));
        assert!(limiter_wanted(true, cap(59), Some(60.0)));
        // Unknown refresh rate: keep the limiter.
        assert!(limiter_wanted(true, cap(60), None));
    }
}
