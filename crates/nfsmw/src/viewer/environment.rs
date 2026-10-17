//! Session-local comparison controls and readbacks; not original-game clock hours.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SunMode {
    #[default]
    CustomClock,
    OriginalCycle,
    FixedPhase,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Environment {
    pub hour: f32,
    pub seconds_per_hour: f32,
    pub sun_mode: SunMode,
    pub sun_phase: f32,
    pub cloud_override: Option<f32>,
    /// One-shot command, never returned by a scene readback.
    pub settle_clouds: bool,
    pub cloud_intensity: f32,
    pub cloud_target: f32,
    pub rain_intensity: f32,
    /// Session comparison override, independent of accumulated rain history.
    pub road_wetness_override: Option<f32>,
    pub road_wetness: f32,
    pub authored_clouds: bool,
    pub sun_direction: [f32; 3],
    pub dusk_blend: f32,
}

impl Default for Environment {
    fn default() -> Self {
        Self {
            hour: 12.0,
            seconds_per_hour: 150.0,
            sun_mode: SunMode::CustomClock,
            sun_phase: 0.5,
            cloud_override: None,
            settle_clouds: false,
            cloud_intensity: 0.0,
            cloud_target: 0.0,
            rain_intensity: 0.0,
            road_wetness_override: None,
            road_wetness: 0.0,
            authored_clouds: false,
            sun_direction: [0.0; 3],
            dusk_blend: 0.0,
        }
    }
}
