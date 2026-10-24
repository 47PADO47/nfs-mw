use super::partial::Percent;
use super::{Settings, TrafficPolicy};

/// The settings the world's traffic follows, told to the scene together so that it can see any of them change.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TrafficSettings {
    /// Active vehicle budget including player, police and trailers (0 stops new traffic).
    pub cars: u32,
    /// Share of them that are patrol cops, in percent.
    pub cop_share: Percent,
    /// Whether they obey traffic lights.
    pub lights: bool,
    /// Whether they go round a car that blocks their lane (`traffic_avoidance`).
    pub avoidance: bool,
    pub policy: TrafficPolicy,
    pub pursuit: super::PursuitOptions,
}

impl From<&Settings> for TrafficSettings {
    fn from(s: &Settings) -> Self {
        Self {
            cars: s.traffic,
            cop_share: s.cop_share,
            lights: s.traffic_lights,
            avoidance: s.traffic_avoidance,
            policy: s.traffic_policy,
            pursuit: s.pursuit_options,
        }
    }
}
