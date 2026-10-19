//! Phase-aligned probe deadlines avoid rounding every capture up to another whole game frame.
use blackbox_gfx::ReflectionQuality;

pub(super) fn reuse(cached: bool, quality: ReflectionQuality, last: f32, now: f32, distance_squared: f32) -> bool {
    let hz: f64 = match (cached, quality) {
        (_, ReflectionQuality::Off) => return false,
        (true, ReflectionQuality::High) => 60.0,
        (true, ReflectionQuality::Medium) => 30.0,
        (true, ReflectionQuality::Low) => 15.0,
        (false, ReflectionQuality::High) => return false,
        (false, ReflectionQuality::Medium) => 30.0,
        (false, ReflectionQuality::Low) => 15.0,
    };
    let movement_limit = match cached {
        true => 1.0,
        false => 4.0,
    };
    if now < last || distance_squared >= movement_limit {
        return false;
    }
    if !cached {
        return f64::from(now - last) < 1.0 / hz;
    }
    // Keep the deadline on the simulation clock, rather than restarting a full interval after
    // each late frame. At 130 FPS a restarted 60 Hz deadline otherwise falls to about 43 Hz.
    // A motion/streaming refresh close to a deadline may service that deadline too.
    let tick = |seconds: f32| {
        let seconds = f64::from(seconds);
        // A 60 FPS frame rounded just below its tick must not accidentally reuse the old probe.
        let tolerance = seconds.abs().max(1.0) * f64::from(f32::EPSILON);
        ((seconds + tolerance) * hz).floor()
    };
    let same_tick = tick(now) == tick(last);
    same_tick || f64::from(now - last) < 0.5 / hz
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_cache_tracks_sixty_hz_without_restarting_the_deadline() {
        for fps in [30, 45, 60, 61, 75, 100, 130, 144, 165, 240, 500] {
            let mut last = 0.0;
            let mut refreshes = Vec::new();
            for frame in 1..=fps * 10 {
                let now = frame as f32 / fps as f32;
                if !reuse(true, ReflectionQuality::High, last, now, 0.0) {
                    refreshes.push(now - last);
                    last = now;
                }
            }
            assert!(
                ((fps.min(60) * 10 - 1) as usize..=(fps.min(60) * 10 + 1) as usize).contains(&refreshes.len()),
                "{fps} FPS: {} captures",
                refreshes.len()
            );
            assert!(refreshes.iter().all(|dt| *dt <= 1.0 / 60.0 + 1.0 / fps as f32 + 0.00001));
            if fps > 60 {
                assert!(refreshes.len() < fps as usize * 10, "cache retains frame skips");
            }
        }
    }

    #[test]
    fn slow_frames_refresh_once_and_motion_and_clock_jumps_refresh_immediately() {
        assert!(!reuse(true, ReflectionQuality::High, 1.0, 1.04, 0.0));
        assert!(!reuse(true, ReflectionQuality::High, 1.0, 1.001, 1.0));
        assert!(!reuse(true, ReflectionQuality::High, 1.0, 0.0, 0.0));
        assert!(reuse(true, ReflectionQuality::High, 1.0, 1.0, 0.0), "paused capture can be reused");
        assert!(reuse(true, ReflectionQuality::High, 0.016, 0.017, 0.0), "no consecutive boundary burst");
        assert!(reuse(true, ReflectionQuality::Medium, 0.0, 0.02, 0.0));
        assert!(!reuse(true, ReflectionQuality::Medium, 0.0, 0.04, 0.0));
        assert!(reuse(true, ReflectionQuality::Low, 0.0, 0.04, 0.0));
    }

    #[test]
    fn disabling_cache_preserves_the_legacy_quality_and_movement_budgets() {
        assert!(!reuse(false, ReflectionQuality::High, 0.0, 0.0, 0.0));
        assert!(reuse(false, ReflectionQuality::Low, 0.0, 0.06, 3.9));
        assert!(!reuse(false, ReflectionQuality::Low, 0.0, 0.07, 0.0));
        assert!(reuse(false, ReflectionQuality::Medium, 0.0, 0.03, 0.0));
        assert!(!reuse(false, ReflectionQuality::Medium, 0.0, 0.01, 4.0));
    }
}
