//! Ordinary PC profiles, from `docs/specs/pc-collision-particles.md`.

use blackbox_attrib::{CollectionRef, Database, Value, vlt_hash};

pub const PC_EMITTERS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PcEmitter {
    pub key: u32,
    pub texture: u32,
    pub grid: u32,
    pub fps: u8,
    pub random_frame: bool,
    pub color: [[f32; 4]; 4],
    pub size: [f32; 4],
    pub keys: [f32; 4],
    pub angles: [f32; 4],
    pub angle_range: f32,
    pub rotation_variance: f32,
    pub random_direction: bool,
    pub center: [f32; 3],
    pub extent: [f32; 3],
    pub velocity: [f32; 3],
    pub velocity_delta: [f32; 3],
    pub acceleration: [f32; 3],
    pub acceleration_delta: [f32; 3],
    pub one_sided: bool,
    pub life: f32,
    pub life_variance: f32,
    pub rate: f32,
    pub rate_variance: f32,
    pub inherit: f32,
    pub inherit_variance: f32,
    pub speed: f32,
    pub speed_variance: f32,
    pub spread: f32,
    pub disc: bool,
    pub drag: f32,
    pub gravity: f32,
    pub constraint: u32,
    pub alpha_kill: u8,
    pub no_alpha_kill: bool,
    pub one_shot: bool,
    pub delay: f32,
    pub random_delay: bool,
    pub on: f32,
    pub on_variance: f32,
    pub off: f32,
    pub off_variance: f32,
    pub intensity: [f32; 2],
}

impl PcEmitter {
    fn read(c: CollectionRef<'_>, intensity: [f32; 2]) -> Option<Self> {
        let f = |name| c.get_f32(name).filter(|v| v.is_finite() && v.abs() <= 10000.0);
        let variance = |name| f(name).filter(|v| (0.0..=1.0).contains(v));
        let vector = |name| match c.get(name)? {
            Value::Vector4(v) if v.iter().all(|n| n.is_finite() && n.abs() <= 10000.0) => Some(*v),
            _ => None,
        };
        let v3 = |name| vector(name).map(|v| [v[0], v[1], v[2]]);
        let flag = |name| match c.get(name)? {
            Value::Bool(v) => Some(*v),
            Value::Int32(v) if (0..=1).contains(v) => Some(*v != 0),
            _ => None,
        };
        if flag("MotionLive")? {
            return None;
        }
        let texture = raw(c.get("Texture")?, "ParticleTextureRecord", 8)?;
        let animation = raw(c.get("TextureAnimation")?, "ParticleAnimationInfo", 8)?;
        let grid = word(animation);
        if !matches!(grid, 0 | 2 | 4 | 8) {
            return None;
        }
        let mut color = [[0.0; 4]; 4];
        for (i, name) in ["Color1", "Color2", "Color3", "Color4"].into_iter().enumerate() {
            let Value::UInt32(v) = c.get(name)? else { return None };
            color[i] = v.to_be_bytes().map(|b| f32::from(b) / 255.0);
        }
        let keys = vector("KeyPositions")?.map(|v| v / 3.0);
        if keys.windows(2).any(|p| p[1] - p[0] < 0.0001) {
            return None;
        }
        let constraint = word(raw(c.get("AxisConstraint")?, "EffectParticleConstraint", 4)?);
        if constraint > 4 {
            return None;
        }
        let alpha_kill = match c.get("AlphaToKillAt")? {
            Value::Int8(v) => *v as u8,
            _ => return None,
        };
        let out = Self {
            key: c.key(),
            texture: word(texture),
            grid,
            fps: animation[4],
            random_frame: animation[5] != 0,
            color,
            size: vector("Size")?,
            keys,
            angles: vector("RelativeAngle")?,
            angle_range: f("InitialAngleRange")?,
            rotation_variance: variance("RotationVariance")?,
            random_direction: flag("RandomRotationDirection")?,
            center: v3("VolumeCenter")?,
            extent: v3("VolumeExtent")?,
            velocity: v3("VelocityStart")?,
            velocity_delta: v3("VelocityDelta")?,
            acceleration: v3("AccelStart")?,
            acceleration_delta: v3("AccelDelta")?,
            one_sided: flag("EliminateUnnecessaryRandomness")?,
            life: f("Life")?,
            life_variance: variance("LifeVariance")?,
            rate: f("NumParticles")?,
            rate_variance: variance("NumParticlesVariance")?,
            inherit: f("MotionInherit")?,
            inherit_variance: variance("MotionInheritVariance")?,
            speed: f("Speed")?,
            speed_variance: variance("SpeedVariance")?,
            spread: f("SpreadAngle")?,
            disc: flag("SpreadAsDisc")?,
            drag: f("Drag")?,
            gravity: f("Gravity")?,
            constraint,
            alpha_kill,
            no_alpha_kill: flag("NoKillAtAlpha")?,
            one_shot: flag("IsOneShot")?,
            delay: f("StartDelay")?,
            random_delay: flag("StartDelayRandomVariance")?,
            on: f("OnCycle")?,
            on_variance: f("OnCycleVariance")?,
            off: f("OffCycle")?,
            off_variance: f("OffCycleVariance")?,
            intensity,
        };
        if out.life <= 0.0
            || out.life > 60.0
            || out.rate < 0.0
            || out.size.iter().any(|&s| s < 0.0)
            || out.extent.iter().any(|&v| v < 0.0)
            || [out.drag, out.delay, out.on, out.off, out.on_variance, out.off_variance].iter().any(|&v| v < 0.0)
        {
            return None;
        }
        Some(out)
    }
}

pub(super) fn read(db: &Database, wrapper: CollectionRef<'_>) -> [Option<PcEmitter>; PC_EMITTERS] {
    let mut out = [None; PC_EMITTERS];
    let Some(group) = wrapper.get("emittergroup").and_then(|v| super::resolve(db, v, "emittergroup")) else {
        return out;
    };
    let Some(emitters) = super::array(group, "Emitters") else { return out };
    if emitters.len() > PC_EMITTERS {
        return out;
    }
    let ranges = super::array(group, "IntensityRanges");
    for (i, v) in emitters.iter().enumerate() {
        let Some(c) = super::resolve(db, v, "emitterdata") else { continue };
        let range = ranges
            .filter(|r| r.len() == emitters.len())
            .and_then(|r| match r[i] {
                Value::Vector2(v) if v.iter().all(|n| n.is_finite()) => Some(v),
                _ => None,
            })
            .filter(|v| v[1] > v[0])
            .unwrap_or([0.0, 1.0]);
        out[i] = PcEmitter::read(c, range);
    }
    out
}

fn raw<'a>(v: &'a Value, name: &str, size: usize) -> Option<&'a [u8]> {
    let Value::Raw { type_key, bytes } = v else { return None };
    (*type_key == vlt_hash(name) && bytes.len() == size).then_some(bytes)
}

fn word(v: &[u8]) -> u32 {
    u32::from_le_bytes(v[..4].try_into().expect("validated record size"))
}
