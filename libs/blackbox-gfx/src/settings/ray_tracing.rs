//! Ray-traced lighting quality.

/// How much ray-traced lighting a renderer does. Only renderers with hardware ray-query support
/// offer it (`Capabilities::ray_tracing`); everywhere else it resolves to [`Off`](Self::Off).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum RayTracing {
    #[default]
    Off,
    Low,
    Medium,
    High,
}

impl RayTracing {
    pub const ALL: [RayTracing; 4] = [RayTracing::Off, RayTracing::Low, RayTracing::Medium, RayTracing::High];

    pub fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    pub fn is_on(self) -> bool {
        self != Self::Off
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_is_the_default_and_the_only_off() {
        assert_eq!(RayTracing::default(), RayTracing::Off);
        assert!(RayTracing::ALL.iter().filter(|r| !r.is_on()).eq([&RayTracing::Off]));
        assert_eq!(RayTracing::ALL.map(RayTracing::name), ["off", "low", "medium", "high"]);
    }
}
