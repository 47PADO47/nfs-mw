//! Optional presentation services. Unsupported backends retain normal rendering and pacing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u32)]
pub enum ReflexMode {
    Off = 0,
    #[default]
    On = 1,
    Boost = 2,
}

impl ReflexMode {
    pub const ALL: [Self; 3] = [Self::Off, Self::On, Self::Boost];
}

impl std::fmt::Display for ReflexMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Off => "off",
            Self::On => "on",
            Self::Boost => "boost",
        })
    }
}

impl std::str::FromStr for ReflexMode {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "off" => Ok(Self::Off),
            "on" => Ok(Self::On),
            "boost" => Ok(Self::Boost),
            _ => Err("expected off, on or boost".into()),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LatencySettings {
    pub reflex: ReflexMode,
    /// Generated frames per rendered frame: 0 is off, 1 is 2×, 2 is 3×, etc.
    pub generated_frames: u32,
    /// Frame limit in microseconds; 0 disables the driver limiter.
    pub frame_limit_us: u32,
    /// False during menus, pause, loading and loss of focus.
    pub gameplay: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LatencyStatus {
    pub reflex_available: bool,
    pub max_generated_frames: u32,
    pub min_dimension: u32,
    pub generation_active: bool,
    /// Actual frames presented since the preceding status sample, never an estimated multiplier.
    pub presented_frames: u32,
    /// Cumulative SDK-reported presentations; None when the SDK cannot measure them.
    pub presented_total: Option<u64>,
    pub driver_pacing: bool,
    pub note: &'static str,
}
