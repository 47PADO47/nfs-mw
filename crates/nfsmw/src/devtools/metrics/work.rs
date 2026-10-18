//! A conservative inference from measured work, waits and pacing; never GPU utilization.
#[derive(Clone, Copy, Debug, Default)]
pub struct Work {
    pub cpu_ms: f32,
    pub gpu_ms: Option<f32>,
    pub acquire_ms: f32,
    pub present_ms: f32,
    pub sleep_ms: f32,
}
impl Work {
    pub fn limit(self, frame_ms: f32, cap: Option<f32>, vsync: bool, focused: bool) -> &'static str {
        if !focused {
            return "Background / unfocused";
        }
        if frame_ms <= 0.0 {
            return "Measuring…";
        }
        if cap.is_some_and(|fps| (1000.0 / frame_ms - fps).abs() < fps * 0.07) {
            return "Frame cap / pacing";
        }
        let Some(gpu) = self.gpu_ms else {
            return "GPU timing unavailable";
        };
        if vsync && self.cpu_ms < frame_ms * 0.8 && gpu < frame_ms * 0.8 {
            return "VSync / presentation pacing";
        }
        if gpu >= frame_ms * 0.75 && gpu > self.cpu_ms * 1.15 {
            return "Likely GPU render limited";
        }
        if self.cpu_ms >= frame_ms * 0.75 && self.cpu_ms > gpu * 1.15 {
            return "Likely CPU limited";
        }
        if self.acquire_ms + self.present_ms > frame_ms * 0.2 {
            return "GPU / frame-gen / presentation wait";
        }
        if self.sleep_ms > frame_ms * 0.2 {
            return "Driver pacing";
        }
        "Mixed / no clear limit"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn distinguishes_work_from_caps_and_waits_without_guessing_missing_gpu_time() {
        let work = Work { cpu_ms: 3.0, gpu_ms: Some(15.0), ..Default::default() };
        assert_eq!(work.limit(16.0, None, false, true), "Likely GPU render limited");
        let cpu = Work { cpu_ms: 15.0, gpu_ms: Some(3.0), ..Default::default() };
        assert_eq!(cpu.limit(16.0, None, false, true), "Likely CPU limited");
        assert_eq!(cpu.limit(16.0, Some(60.0), false, true), "Frame cap / pacing");
        assert_eq!(Work::default().limit(16.0, None, false, true), "GPU timing unavailable");
        let wait = Work { cpu_ms: 3.0, gpu_ms: Some(4.0), present_ms: 10.0, ..Default::default() };
        assert_eq!(wait.limit(16.0, None, false, true), "GPU / frame-gen / presentation wait");
        assert_eq!(wait.limit(16.0, None, true, true), "VSync / presentation pacing");
        assert_eq!(wait.limit(16.0, None, false, false), "Background / unfocused");
    }
}
