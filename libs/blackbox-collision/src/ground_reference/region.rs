use crate::math::{Vec3, add, dot, normalize, scale, sub};

/// Coefficients of the predictive candidate region, supplied by the caller.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RegionSettings {
    pub initial_expansion: f32,
    pub motion_threshold: f32,
    pub prediction: f32,
    pub padding: f32,
    pub maximum_radius: f32,
}

#[derive(Clone, Debug, Default)]
pub(super) struct Region {
    pub center: Vec3,
    pub radius: f32,
    previous_request: Vec3,
    initialized: bool,
}

impl Region {
    pub fn refresh(&mut self, position: Vec3, radius: f32, settings: &RegionSettings, force: bool) -> bool {
        let margin = self.radius - radius;
        let distance = sub(position, self.center);
        let inside = margin >= 0.0 && dot(distance, distance) < margin * margin;
        let rebuild = force || !self.initialized || !inside;
        if rebuild {
            let travel = sub(position, self.previous_request);
            let distance = dot(travel, travel).sqrt();
            self.center = position;
            self.radius = radius * settings.initial_expansion;
            if self.initialized && distance <= settings.motion_threshold {
                let direction = normalize(travel).unwrap_or([0.0; 3]);
                let extension = scale(direction, distance * settings.prediction);
                self.center = add(position, extension);
                self.radius =
                    (radius + dot(extension, extension).sqrt() + settings.padding).min(settings.maximum_radius);
            }
            self.initialized = true;
        }
        self.previous_request = position;
        rebuild
    }
}
