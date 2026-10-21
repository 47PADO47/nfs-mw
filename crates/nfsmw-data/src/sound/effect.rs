//! Standalone world-effect sounds, independent of body collision admission.
use super::{CollisionZone, ImpactSound};
use blackbox_attrib::{Database, vlt_hash};

#[derive(Debug, Clone)]
pub struct EffectSound {
    pub sound: ImpactSound,
    pub quadratic: [f32; 4],
    pub cull_distance: f32,
}

impl EffectSound {
    pub fn read(db: &Database, key: u32, zone: CollisionZone) -> Option<Self> {
        let wrapper = db.collection_by_key(vlt_hash("effects"), key)?;
        let reference =
            |field| wrapper.get(field).and_then(|v| v.as_ref_spec()).filter(|r| r.class != 0 && r.collection != 0);
        let selected = reference(zone.field()).or_else(|| reference(CollisionZone::Default.field()))?;
        if !["audioevent", "audioimpact", "audioscrape"].into_iter().any(|name| vlt_hash(name) == selected.class) {
            return None;
        }
        let target = db.resolve(selected)?;
        Some(Self {
            sound: ImpactSound::read(target),
            quadratic: wrapper.get_vector4("AudioQuadratic").unwrap_or([0.0, 1.0, 0.0, 0.0]),
            cull_distance: wrapper.get_f32("AudioCullDist").unwrap_or(0.0),
        })
    }

    pub fn magnitude(&self, magnitude: f32, distance: f32) -> Option<f32> {
        if !magnitude.is_finite() || !distance.is_finite() || distance > self.cull_distance {
            return None;
        }
        let magnitude = magnitude.clamp(0.0, 1.0);
        let [a, b, c, d] = self.quadratic;
        let result = (((magnitude * d + c) * magnitude + b) * magnitude + a).clamp(0.0, 1.0);
        (result.is_finite() && result > 0.0).then_some(result)
    }
}
