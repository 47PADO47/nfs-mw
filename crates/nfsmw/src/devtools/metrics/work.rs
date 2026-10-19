//! Activity-based bottleneck evidence; CPU elapsed time and GPU queue spans are not utilization.
#[derive(Clone, Copy, Debug, Default)]
pub struct Work {
    pub cpu_ms: f32,
    pub gpu_ms: Option<f32>,
    pub acquire_ms: f32,
    pub present_ms: f32,
    pub sleep_ms: f32,
    pub activity: blackbox_gfx::WorkloadActivity,
}
#[derive(Clone, Copy, Debug)]
pub struct Assessment {
    pub label: &'static str,
    pub hint: &'static str,
}
impl Work {
    pub fn limit(self, frame_ms: f32, cap: Option<f32>, vsync: bool, focused: bool, generated: bool) -> Assessment {
        let result = |label, hint| Assessment { label, hint };
        if !focused {
            return result("Background / unfocused", "Focus the game before comparing CPU and GPU costs.");
        }
        if !frame_ms.is_finite() || frame_ms <= 0.0 {
            return result("Measuring…", "Wait for a stable base-frame cadence and activity sample.");
        }
        if cap.is_some_and(|fps| fps > 0.0 && (1000.0 / frame_ms - fps).abs() < fps * 0.07) {
            return result("At frame cap / pacing", "Uncap base FPS to measure optimization headroom.");
        }
        let valid = |v: Option<f32>| v.filter(|v| v.is_finite() && (0.0..=100.0).contains(v));
        let fresh = self.activity.sample > 0 && (0.0..=2500.0).contains(&self.activity.age_ms);
        let cpu = fresh.then(|| valid(self.activity.main_thread_busy_percent)).flatten();
        let gpu = fresh.then(|| valid(self.activity.gpu_busy_percent)).flatten();
        if gpu.is_some_and(|v| v >= 90.0) {
            if cpu.is_some_and(|v| v >= 85.0) {
                return result(
                    "CPU + GPU pressure",
                    "Both are busy: profile simulation/draw preparation and GPU passes separately.",
                );
            }
            if generated {
                return result(
                    "Likely GPU limited (rendering / FG)",
                    "GPU engine is saturated, including FG. Compare FG off, render scale and shadow/AO cost.",
                );
            }
            return result(
                "Likely GPU limited",
                "GPU engine is saturated. Profile shadows, AO, reflections and pixel/render cost.",
            );
        }
        if cpu.is_some_and(|v| v >= 85.0) && gpu.is_some_and(|v| v < 75.0) {
            return result(
                "Likely CPU main-thread limited",
                "GPU has measured headroom. Profile simulation, visibility, draw preparation and driver CPU cost.",
            );
        }
        let waits = self.acquire_ms + self.present_ms;
        if vsync && waits + self.sleep_ms > frame_ms * 0.2 && gpu.is_some_and(|v| v < 85.0) {
            return result("VSync / presentation pacing", "Disable pacing for an uncapped CPU/GPU comparison.");
        }
        if waits > frame_ms * 0.2 {
            return result(
                "FG / presentation wait; limit uncertain",
                "A blocked present is not CPU work. Check pacing and compare FG off; neither processor is proven saturated.",
            );
        }
        if self.sleep_ms > frame_ms * 0.2 {
            return result(
                "Pacing / idle; no proven bottleneck",
                "Reflex sleep alone does not establish CPU or GPU limitation.",
            );
        }
        if cpu.is_none() || gpu.is_none() {
            return result(
                "CPU/GPU activity unavailable",
                "Waiting for fresh OS counters. Render timestamps alone cannot identify the full FG bottleneck.",
            );
        }
        result(
            "Mixed / no clear saturated engine",
            "Profile CPU worker/driver waits and GPU passes; elapsed submission time is not CPU utilization.",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn measured(cpu: f32, gpu: f32) -> Work {
        Work {
            cpu_ms: 15.0,
            gpu_ms: Some(4.0),
            activity: blackbox_gfx::WorkloadActivity {
                main_thread_busy_percent: Some(cpu),
                gpu_busy_percent: Some(gpu),
                sample: 1,
                ..Default::default()
            },
            ..Default::default()
        }
    }
    #[test]
    fn long_cpu_wall_time_is_not_cpu_work_even_with_fg_and_a_short_render_timestamp() {
        let gpu = measured(25.0, 98.0);
        assert_eq!(gpu.limit(16.0, None, false, true, true).label, "Likely GPU limited (rendering / FG)");
        let cpu = measured(95.0, 40.0);
        assert_eq!(cpu.limit(16.0, None, false, true, true).label, "Likely CPU main-thread limited");
        assert_eq!(measured(95.0, 98.0).limit(16.0, None, false, true, true).label, "CPU + GPU pressure");
    }
    #[test]
    fn caps_and_waits_never_turn_into_cpu_saturation() {
        let mut work = measured(25.0, 40.0);
        work.present_ms = 10.0;
        assert_eq!(work.limit(16.0, None, false, true, true).label, "FG / presentation wait; limit uncertain");
        assert_eq!(work.limit(16.0, None, true, true, true).label, "VSync / presentation pacing");
        // Cap uses base cadence, regardless of a 2x-6x presented rate.
        assert_eq!(work.limit(1000.0 / 60.0, Some(60.0), false, true, true).label, "At frame cap / pacing");
        assert_eq!(work.limit(16.0, None, false, false, true).label, "Background / unfocused");
    }
    #[test]
    fn missing_stale_or_invalid_counters_are_unknown_not_idle() {
        assert_eq!(
            Work { cpu_ms: 16.0, gpu_ms: Some(2.0), ..Default::default() }.limit(16.0, None, false, true, false).label,
            "CPU/GPU activity unavailable"
        );
        let mut work = measured(99.0, 5.0);
        work.activity.age_ms = 3000.0;
        assert_eq!(work.limit(16.0, None, false, true, false).label, "CPU/GPU activity unavailable");
        work.activity.age_ms = 0.0;
        work.activity.gpu_busy_percent = None;
        assert_eq!(work.limit(16.0, None, false, true, true).label, "CPU/GPU activity unavailable");
    }
}
