use blackbox_attrib::CollectionRef;

/// Runtime fuelcell tuning. Dimensional conversions are specified in xenon-particle-motion.md.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EmitterStyle {
    pub color: [f32; 4],
    pub life: f32,
    pub life_variance: f32,
    pub count: f32,
    pub count_variance: f32,
    pub center: [f32; 3],
    pub extent: [f32; 3],
    pub velocity: [f32; 3],
    pub variation: [f32; 3],
    pub inherit: [f32; 3],
    pub gravity: f32,
    pub gravity_delta: f32,
    pub length: f32,
    pub length_delta: f32,
    pub height: f32,
}

impl EmitterStyle {
    pub(super) fn read(emitter: CollectionRef<'_>) -> Option<Self> {
        let vector = |name| {
            let v = emitter.get(name)?.as_vector4()?;
            Some([v[0], v[1], v[2]])
        };
        Self {
            color: emitter.get("Colour1")?.as_vector4()?,
            life: emitter.get_f32("Life")?,
            life_variance: emitter.get_f32("LifeVariance")?,
            count: emitter.get_f32("NumParticles")?,
            count_variance: emitter.get_f32("NumParticlesVariance")?,
            center: vector("VolumeCenter")?,
            extent: vector("VolumeExtent")?,
            velocity: vector("VelocityStart")?,
            variation: vector("VelocityDelta")?,
            inherit: vector("VelocityInherit")?,
            gravity: emitter.get_f32("GravityStart")?,
            gravity_delta: emitter.get_f32("GravityDelta")?,
            length: emitter.get_f32("LengthStart")?,
            length_delta: emitter.get_f32("LengthDelta")?,
            height: emitter.get_f32("HeightStart")?,
        }
        .validated()
    }

    pub(super) fn validated(self) -> Option<Self> {
        if self.color.iter().any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
            || !self.life.is_finite()
            || !(0.0..=10.0).contains(&self.life)
            || self.life == 0.0
            || !self.life_variance.is_finite()
            || !(0.0..1.0).contains(&self.life_variance)
            || !self.count.is_finite()
            || !(0.0..=256.0).contains(&self.count)
            || !self.count_variance.is_finite()
            || !(0.0..=0.01).contains(&self.count_variance)
        {
            return None;
        }
        for values in [self.center, self.extent, self.velocity, self.variation, self.inherit] {
            if values.iter().any(|v| !v.is_finite() || v.abs() > 1000.0) {
                return None;
            }
        }
        if self.extent.iter().chain(&self.variation).any(|v| *v < 0.0)
            || [self.gravity, self.gravity_delta].iter().any(|v| !v.is_finite() || v.abs() > 1000.0)
            || [self.length, self.length_delta, self.height].iter().any(|v| !v.is_finite() || *v < 0.0 || *v > 4096.0)
        {
            return None;
        }
        Some(self)
    }
}
