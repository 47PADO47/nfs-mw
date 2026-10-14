//! A caller-supplied maneuver assist; all defaults are neutral.

/// Coupled forces/steering for a temporary maneuver. Game-specific coefficients belong to the caller.
#[derive(Clone, Copy, Debug)]
pub struct ManeuverAssist {
    /// Extra acceleration opposite body up in m/s², independent of mass.
    pub extra_downward_acceleration: f32,
    /// Tire steering-drag coefficient; `None` retains normal tire setup.
    pub tire_steering_drag: Option<f32>,
    /// Temporary collision mass relative to actual body mass, after material scaling.
    pub collision_mass_scale: Option<f32>,
    /// Aerodynamic drag multiplier.
    pub drag_scale: f32,
    /// Gain applied to front lateral force and traction, multiplied by absolute steering input.
    pub front_grip_gain: f32,
    /// Override steering range in degrees. `None` retains normal steering shaping.
    pub steering_degrees: Option<f32>,
}

impl Default for ManeuverAssist {
    fn default() -> Self {
        Self {
            extra_downward_acceleration: 0.0,
            tire_steering_drag: None,
            collision_mass_scale: None,
            drag_scale: 1.0,
            front_grip_gain: 0.0,
            steering_degrees: None,
        }
    }
}

impl super::Vehicle {
    /// Override the temporary object-contact moment without modifying integration mass/inertia.
    pub fn modify_object_hit(&self, hit: &mut crate::rigid_body::BodyHit) {
        let Some(scale) = self.maneuver.collision_mass_scale else { return };
        if hit.immobile || hit.mass_scale <= 0.0 || scale <= 0.0 {
            return;
        }
        hit.inertia_scale *= scale / hit.mass_scale;
        hit.mass_scale = scale;
    }
}
