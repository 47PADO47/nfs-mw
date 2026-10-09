//! The `ray_tracing` setting: ray-traced lighting (docs/renderers.md). Only the Bevy renderer on a GPU with ray
//! queries has it; everywhere else it runs as `off`.

use blackbox_gfx::RayTracing;

/// How much ray-traced lighting to draw. Switching between `off` and any other level needs a restart.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RayTracingLevel {
    #[default]
    Off,
    Low,
    Medium,
    High,
}

names!(
    RayTracingLevel,
    "off, low, medium or high",
    [(Self::Off, "off"), (Self::Low, "low"), (Self::Medium, "medium"), (Self::High, "high")]
);

impl RayTracingLevel {
    /// The renderer's ray-tracing level.
    pub fn level(self) -> RayTracing {
        match self {
            Self::Off => RayTracing::Off,
            Self::Low => RayTracing::Low,
            Self::Medium => RayTracing::Medium,
            Self::High => RayTracing::High,
        }
    }
}
