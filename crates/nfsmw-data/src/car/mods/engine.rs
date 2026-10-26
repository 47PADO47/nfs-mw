//! A mod car's own engine sound (the `[engine_sound]` table of the car manifest, docs/modding/car-customization.md):
//! looping recordings the audio mixes by engine speed, in place of the clone's engine.

/// One looping recording and the engine speed it was made at.
#[derive(Debug, Clone, PartialEq)]
pub struct EngineLayer {
    /// The WAV file, a path of the install's layout.
    pub file: String,
    pub rpm: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EngineSound {
    /// The rpm the engine idles at and the rpm of its rev limiter: what the sound's `0 to 1` engine speed spans.
    pub idle_rpm: f32,
    pub redline_rpm: f32,
    /// Slowest first.
    pub layers: Vec<EngineLayer>,
}

impl EngineSound {
    /// The engine speed in rpm at `fraction` of the way from idle to redline.
    pub fn rpm_at(&self, fraction: f32) -> f32 {
        self.idle_rpm + fraction.clamp(0.0, 1.0) * (self.redline_rpm - self.idle_rpm)
    }
}
