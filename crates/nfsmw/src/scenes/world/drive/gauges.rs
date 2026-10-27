//! Completed display samples, interpolated with the same render phase as the car pose.

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct GaugeSample {
    pub rpm: f32,
    pub speed: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct GaugeHistory {
    previous: GaugeSample,
    current: GaugeSample,
}

impl GaugeHistory {
    pub fn snap(&mut self, sample: GaugeSample) {
        self.previous = sample;
        self.current = sample;
    }

    pub fn push(&mut self, sample: GaugeSample) {
        self.previous = self.current;
        self.current = sample;
    }

    pub fn sample(&self, alpha: f32) -> GaugeSample {
        let alpha = match alpha.is_finite() {
            true => alpha.clamp(0.0, 1.0),
            false => 1.0,
        };
        GaugeSample {
            rpm: self.previous.rpm + (self.current.rpm - self.previous.rpm) * alpha,
            speed: self.previous.speed + (self.current.speed - self.previous.speed) * alpha,
        }
    }
}

pub(super) fn in_launch_range(rpm: f32, band: Option<(f32, f32)>) -> bool {
    let Some((lower, width)) = band else { return false };
    lower > 0.0 && width > 0.0 && rpm >= lower && rpm <= lower + width
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_frames_between_ticks_interpolate_without_advancing_the_gauge() {
        let mut history = GaugeHistory::default();
        history.snap(GaugeSample { rpm: 800.0, speed: 0.0 });
        history.push(GaugeSample { rpm: 1200.0, speed: 2.0 });
        assert_eq!(history.sample(0.0), GaugeSample { rpm: 800.0, speed: 0.0 });
        assert_eq!(history.sample(0.25), GaugeSample { rpm: 900.0, speed: 0.5 });
        assert_eq!(history.sample(0.75), GaugeSample { rpm: 1100.0, speed: 1.5 });
        history.push(GaugeSample { rpm: 1600.0, speed: 4.0 });
        assert_eq!(history.sample(0.5), GaugeSample { rpm: 1400.0, speed: 3.0 });
        history.snap(GaugeSample { rpm: 800.0, speed: 0.0 });
        assert_eq!(history.sample(0.5), GaugeSample { rpm: 800.0, speed: 0.0 });
    }

    #[test]
    fn visual_launch_range_includes_endpoints_but_requires_a_band() {
        let band = Some((3500.0, 1550.0));
        for rpm in [3500.0, 4200.0, 5050.0] {
            assert!(in_launch_range(rpm, band));
        }
        for rpm in [3499.0, 5051.0, f32::NAN] {
            assert!(!in_launch_range(rpm, band));
        }
        assert!(!in_launch_range(4200.0, None));
        assert!(!in_launch_range(4200.0, Some((0.0, 5050.0))));
    }
}
