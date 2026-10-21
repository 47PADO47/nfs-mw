//! Original effects records shared by sequenced scenery effects.
use crate::car::exhaust::{GroupEmitter, group_emitters};
use blackbox_attrib::{Database, vlt_hash};

#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    pub quadratic: [f32; 4],
    pub inherit_velocity: f32,
}

impl Default for Parameters {
    fn default() -> Self {
        Self { quadratic: [0.0, 1.0, 0.0, 0.0], inherit_velocity: 0.0 }
    }
}

impl Parameters {
    pub fn intensity(self, magnitude: f32) -> f32 {
        let [a, b, c, d] = self.quadratic;
        (((magnitude * d + c) * magnitude + b) * magnitude + a).clamp(0.0, 1.0)
    }
}

pub fn parameters(db: &Database, key: u32) -> Option<Parameters> {
    let effect = db.collection_by_key(vlt_hash("effects"), key)?;
    Some(Parameters {
        quadratic: effect.get_vector4("EmitterQuadratic").unwrap_or([0.0, 1.0, 0.0, 0.0]),
        inherit_velocity: effect.get_f32("InheritVelocity").unwrap_or(0.0).max(0.0),
    })
}

pub fn emitters(db: &Database, key: u32) -> Vec<GroupEmitter> {
    let Some(effect) = db.collection_by_key(vlt_hash("effects"), key) else { return Vec::new() };
    let Some(group) = effect.get_by_key(0xABA8_6E60).and_then(|v| v.as_ref_spec()).and_then(|r| db.resolve(r)) else {
        return Vec::new();
    };
    group_emitters(db, group)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapper_polynomials_can_activate_particles_at_zero_event_magnitude() {
        assert_eq!(Parameters::default().intensity(0.0), 0.0);
        let parameters = Parameters { quadratic: [0.8, 1.0, 0.0, 0.0], ..Default::default() };
        assert_eq!(parameters.intensity(0.0), 0.8);
        assert_eq!(parameters.intensity(0.5), 1.0);
        let parameters = Parameters { quadratic: [0.1, 0.2, 0.3, 0.4], ..Default::default() };
        assert!((parameters.intensity(0.5) - 0.325).abs() < 1e-6);
    }
}
