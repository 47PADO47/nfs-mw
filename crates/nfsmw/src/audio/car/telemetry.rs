//! Ordered game-side producers for the authored car effects.
use super::ambient::RoadContext;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CarAudioTelemetry {
    pub forward_speed: f32,
    pub max_physics_rpm: f32,
    pub physics_position: Option<glam::Vec3>,
    pub environment: Option<crate::audio::environment::EnvironmentContext>,
    pub understeer: f32,
    pub oversteer: f32,
    pub road: Option<RoadContext>,
}
