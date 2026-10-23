use crate::settings::{RoadReflectionResolution, RoadReflections};

pub(crate) struct WetRoad {
    pub quality: RoadReflections,
    pub resolution: RoadReflectionResolution,
    pub instances: Vec<blackbox_gfx::Instance>,
    pub wetness: f32,
    pooling: f32,
    time: f32,
    rain: f32,
    pub override_amount: Option<f32>,
}
impl Default for WetRoad {
    fn default() -> Self {
        Self {
            quality: Default::default(),
            resolution: Default::default(),
            instances: Vec::new(),
            wetness: 0.0,
            pooling: 0.0,
            time: 0.0,
            rain: 0.0,
            override_amount: std::env::var("NFSMW_ROAD_WETNESS")
                .ok()
                .and_then(|v| v.parse::<f32>().ok())
                .filter(|v| v.is_finite())
                .map(|v| v.clamp(0.0, 1.0)),
        }
    }
}
impl WetRoad {
    #[cfg(test)]
    pub fn step(&mut self, dt: f32, raining: bool) {
        self.step_rain(dt, u8::from(raining) as f32);
    }
    pub fn step_rain(&mut self, dt: f32, intensity: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        let intensity = match intensity.is_finite() {
            true => intensity.clamp(0.0, 1.0),
            false => 0.0,
        };
        let raining = intensity > 0.0;
        let target = u8::from(raining) as f32;
        self.rain = intensity;
        self.time = (self.time + dt) % 4096.0;
        // Pooling builds after saturation and drains faster than damp asphalt dries.
        let pool_target = ((self.wetness - 0.45) / 0.55).clamp(0.0, 1.0) * intensity;
        let rate: f32 = match raining {
            true => 0.08,
            false => 0.45,
        };
        self.pooling += (pool_target - self.pooling) * (1.0 - (-rate * dt).exp());
        // Elapsed-time adaptation of the original per-render-call 0.005 recurrence at 60 Hz.
        self.wetness = target + (self.wetness - target) * 0.995_f32.powf(60.0 * dt);
        if !raining && self.wetness < 0.05 {
            self.wetness = 0.0;
        }
    }
    pub fn amount(&self) -> f32 {
        self.override_amount.unwrap_or(self.wetness)
    }
    pub fn water(&self) -> blackbox_gfx::SurfaceWater {
        blackbox_gfx::SurfaceWater {
            dampness: self.amount(),
            pooling: self.override_amount.unwrap_or(self.pooling),
            time: self.time,
            rain: self.rain,
        }
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
