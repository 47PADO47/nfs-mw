use crate::settings::{RoadReflectionResolution, RoadReflections};

pub(crate) struct WetRoad {
    pub quality: RoadReflections,
    pub resolution: RoadReflectionResolution,
    pub instances: Vec<blackbox_gfx::Instance>,
    pub wetness: f32,
    pub override_amount: Option<f32>,
}
impl Default for WetRoad {
    fn default() -> Self {
        Self {
            quality: Default::default(),
            resolution: Default::default(),
            instances: Vec::new(),
            wetness: 0.0,
            override_amount: std::env::var("NFSMW_ROAD_WETNESS")
                .ok()
                .and_then(|v| v.parse::<f32>().ok())
                .filter(|v| v.is_finite())
                .map(|v| v.clamp(0.0, 1.0)),
        }
    }
}
impl WetRoad {
    pub fn step(&mut self, dt: f32, raining: bool) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        let target = u8::from(raining) as f32;
        // Elapsed-time adaptation of the original per-render-call 0.005 recurrence at 60 Hz.
        self.wetness = target + (self.wetness - target) * 0.995_f32.powf(60.0 * dt);
        if !raining && self.wetness < 0.05 {
            self.wetness = 0.0;
        }
    }
    pub fn amount(&self) -> f32 {
        self.override_amount.unwrap_or(self.wetness)
    }
    pub fn eligible(&self) -> bool {
        self.quality != RoadReflections::Off && self.amount() > 0.001
    }
    pub fn contribution(&self) -> f32 {
        if self.quality == RoadReflections::Off {
            return 0.0;
        }
        self.amount()
    }
}
