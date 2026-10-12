//! The time-of-day lighting presets: the 8 `timeofdaylighting` collections, loaded in the game's order, and
//! the blend between two of them. Spec: `docs/specs/time-of-day.md`. The clock that picks the blend is not
//! implemented yet.

use blackbox_attrib::{Database, Value, vlt_hash};

/// The attribute class of a lighting preset.
pub const CLASS: &str = "timeofdaylighting";
/// The 8 presets, by collection name hash, in the order the game loads them.
pub const PRESET_KEYS: [u32; 8] =
    [0x3DCF_A673, 0xB902_CFAD, 0x59C3_73C3, 0xD835_D0C8, 0xA01A_3942, 0x0E1C_797C, 0x94AB_DFF0, 0x919B_6689];

/// One field of a preset. Only the two kinds the blend can interpolate are kept.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Field {
    Scalar(f32),
    Vector4([f32; 4]),
}

/// A preset: its fields by name hash.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Preset {
    pub fields: Vec<(u32, Field)>,
}

impl Preset {
    /// The field with this name hash.
    pub fn get(&self, key: u32) -> Option<Field> {
        self.fields.iter().find(|(k, _)| *k == key).map(|(_, f)| *f)
    }

    /// Whether the preset holds at least one field.
    pub fn is_usable(&self) -> bool {
        !self.fields.is_empty()
    }
}

/// The 8 presets, in `PRESET_KEYS` order.
#[derive(Debug, Clone, PartialEq)]
pub struct Presets {
    pub presets: Vec<Preset>,
}

/// Loads the 8 presets. Returns `None` if the class or any of the 8 collections is missing.
pub fn load(db: &Database) -> Option<Presets> {
    let class = vlt_hash(CLASS);
    let presets = PRESET_KEYS
        .iter()
        .map(|&key| db.collection_by_key(class, key).map(|c| preset(c.attributes())))
        .collect::<Option<Vec<_>>>()?;
    Some(Presets { presets })
}

/// Every field of one collection that is a float or a `Vector4`.
fn preset(attributes: &[blackbox_attrib::Attribute]) -> Preset {
    let fields = attributes.iter().filter_map(|a| field(&a.value).map(|f| (a.key, f))).collect();
    Preset { fields }
}

fn field(value: &Value) -> Option<Field> {
    if let Some(v) = value.as_vector4() {
        return Some(Field::Vector4(v));
    }
    let v = value.as_f32()?;
    Some(Field::Scalar(v))
}

/// The length of the day cycle, in seconds (the clock reads the counter modulo 3600 seconds; see the spec).
pub const DAY_SECONDS: i64 = 3600;

/// The clock's phase in `[0, 1]`, from the world's whole-second counter. `base_ms` is the engine's base time (see the spec) and
/// `offset_ms` is the counter's offset, both in milliseconds. The phase is a triangle over one hour: it
/// rises from 0 to 1 in the first 30 minutes and falls back to 0 by the hour's end. Spec: `docs/specs/time-of-day.md`.
pub fn clock_phase(ticks_seconds: u32, base_ms: i64, offset_ms: i64) -> f32 {
    let ms = (i64::from(ticks_seconds) % DAY_SECONDS) * 1000 + base_ms - offset_ms;
    let wrapped = ms % (DAY_SECONDS * 1000);
    let f = wrapped as f32 * (1.0 / 1_800_000.0);
    if f >= 1.0 {
        return 2.0 - f;
    }
    f
}

/// Blends `a` towards `b` by `t` (clamped to `[0, 1]`). A field present in both is interpolated per
/// component; a field in only one of them is kept from that preset.
pub fn blend(a: &Preset, b: &Preset, t: f32) -> Preset {
    let t = if t.is_finite() { t.clamp(0.0, 1.0) } else { 0.0 };
    let mut fields: Vec<(u32, Field)> = a
        .fields
        .iter()
        .map(|&(key, fa)| match b.get(key) {
            Some(fb) => (key, lerp(fa, fb, t)),
            None => (key, fa),
        })
        .collect();
    fields.extend(b.fields.iter().filter(|(key, _)| a.get(*key).is_none()));
    Preset { fields }
}

fn lerp(a: Field, b: Field, t: f32) -> Field {
    match (a, b) {
        (Field::Scalar(x), Field::Scalar(y)) => Field::Scalar(x + (y - x) * t),
        (Field::Vector4(x), Field::Vector4(y)) => Field::Vector4(std::array::from_fn(|i| x[i] + (y[i] - x[i]) * t)),
        (kept, _) => kept,
    }
}

/// The colour fields of a preset, by name hash. The names were matched by hashing candidate names with the
/// attribute hash (`AmbientColour`, `DiffuseColour`, `SpecularColour`, `FogHazeColour`, `FogSkyColour`).
pub const AMBIENT_COLOUR: u32 = 0x5219_4D72;
pub const DIFFUSE_COLOUR: u32 = 0x7623_FEB2;
pub const SPECULAR_COLOUR: u32 = 0x5855_B1E7;
pub const FOG_HAZE_COLOUR: u32 = 0xC91C_60C7;
pub const FOG_SKY_COLOUR: u32 = 0x1D76_EA96;

/// The dusk factor from the sun's height `sun_z` (the game's `+0x188`): `1 − (clamp(z, 0.25, 0.5) − 0.25) · 4`.
/// It is 1 when the sun is at or below 0.25 and 0 at or above 0.5.
pub fn dusk_factor(sun_z: f32) -> f32 {
    1.0 - (sun_z.clamp(0.25, 0.5) - 0.25) * 4.0
}

/// A preset's colour by name hash: the first three components of its `Vector4`.
pub fn colour(preset: &Preset, key: u32) -> Option<[f32; 3]> {
    match preset.get(key)? {
        Field::Vector4(v) => Some([v[0], v[1], v[2]]),
        Field::Scalar(_) => None,
    }
}

/// The lighting colours of one frame. Each is the game's blend, mixed by the dusk factor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lighting {
    pub diffuse: [f32; 3],
    pub ambient: [f32; 3],
    pub specular: [f32; 3],
    pub fog_haze: [f32; 3],
    pub fog_sky: [f32; 3],
}

/// The lighting for the sun's height and the blend weight `t`, as the game's lighting update mixes it. The
/// low-sun side is presets 1 and 3 blended by `t` (the game's `local_150`), and the high-sun side is presets 0
/// and 2 blended by `t` (the game's `local_b0`, from the lighting object's `+0xC0` and `+0xE8` holders). They are
/// mixed by the dusk factor as `out = high + (low − high)·dusk` (the lighting update's mix; see the spec). See `docs/specs/time-of-day.md`.
pub fn lighting(presets: &Presets, t: f32, sun_z: f32) -> Option<Lighting> {
    let low = blend(&presets.presets[1], &presets.presets[3], t);
    let high = blend(&presets.presets[0], &presets.presets[2], t);
    let k = dusk_factor(sun_z);
    let mix = |key: u32| -> Option<[f32; 3]> {
        let (lo, hi) = (colour(&low, key)?, colour(&high, key)?);
        Some(std::array::from_fn(|i| hi[i] + (lo[i] - hi[i]) * k))
    };
    Some(Lighting {
        diffuse: mix(DIFFUSE_COLOUR)?,
        ambient: mix(AMBIENT_COLOUR)?,
        specular: mix(SPECULAR_COLOUR)?,
        fog_haze: mix(FOG_HAZE_COLOUR)?,
        fog_sky: mix(FOG_SKY_COLOUR)?,
    })
}

/// The fog colour for the sun's height and the blend weight `t`: the lighting's fog-haze colour.
pub fn fog_colour(presets: &Presets, t: f32, sun_z: f32) -> Option<[f32; 3]> {
    let low = colour(&blend(&presets.presets[1], &presets.presets[3], t), FOG_HAZE_COLOUR)?;
    let high = colour(&blend(&presets.presets[0], &presets.presets[2], t), FOG_HAZE_COLOUR)?;
    let k = dusk_factor(sun_z);
    Some(std::array::from_fn(|i| high[i] + (low[i] - high[i]) * k))
}

/// The sun's phase limit `lo`: 0.35 in game state 3, otherwise 0.05. The phase stays in `[lo, 1 - lo]`.
pub fn sun_phase_limit(state: u32) -> f32 {
    if state == 3 {
        return 0.35;
    }
    0.05
}

/// One frame of the sun's phase (the sun builder's integrator branch, see the spec). `rate` is the object's rate (1.0),
/// `dir` is ±1, and `dt` is the frame time in seconds. The phase moves by `dt · dir · rate · 0.05 / 60`, and
/// at either limit it stops there and the direction flips. Returns the new phase and direction.
pub fn advance_sun_phase(p: f32, dir: i32, rate: f32, dt: f32, lo: f32) -> (f32, i32) {
    let next = p + dt * dir as f32 * rate * 0.05 / 60.0;
    let hi = 1.0 - lo;
    if next < lo {
        return (lo, -dir);
    }
    if next > hi {
        return (hi, -dir);
    }
    (next, dir)
}

/// The sun's direction (unit vector, world axes of the lighting object) for the phase `p`, clamped to
/// `[lo, 1 - lo]`, and the two angle offsets `a` (`+0x0C`) and `b` (`+0x10`). `u = (p - 0.5)·π`:
/// `v = (sin u·cos a + sin a·sin b·cos u, sin b·cos a·cos u + sin u·sin a, cos u·cos b)`, normalised.
pub fn sun_direction(p: f32, lo: f32, a: f32, b: f32) -> [f32; 3] {
    let p = p.clamp(lo, 1.0 - lo);
    let u = (p - 0.5) * std::f32::consts::PI;
    let (su, cu) = (u.sin(), u.cos());
    let v = [su * a.cos() + a.sin() * b.sin() * cu, b.sin() * a.cos() * cu + su * a.sin(), cu * b.cos()];
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    [v[0] / len, v[1] / len, v[2] / len]
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY_A: u32 = 1;

    #[test]
    fn the_dusk_factor_follows_the_sun_height() {
        assert_eq!(dusk_factor(0.1), 1.0);
        assert_eq!(dusk_factor(0.25), 1.0);
        assert_eq!(dusk_factor(0.5), 0.0);
        assert_eq!(dusk_factor(0.9), 0.0);
        assert!((dusk_factor(0.4) - 0.4).abs() < 1e-6);
    }

    fn colour_preset(key: u32, c: [f32; 4]) -> Preset {
        Preset { fields: vec![(key, Field::Vector4(c))] }
    }

    #[test]
    fn the_fog_colour_mixes_the_day_and_dusk_presets_by_the_sun() {
        let low = colour_preset(FOG_HAZE_COLOUR, [1.0, 0.0, 0.0, 1.0]);
        let high = colour_preset(FOG_HAZE_COLOUR, [0.0, 0.0, 1.0, 1.0]);
        let presets = Presets {
            presets: vec![
                Preset::default(),
                low.clone(),
                high,
                low,
                Preset::default(),
                Preset::default(),
                Preset::default(),
                Preset::default(),
            ],
        };
        // Sun at horizon height (dusk 1): the low-sun pair shows.
        assert_eq!(fog_colour(&presets, 0.0, 0.2), Some([1.0, 0.0, 0.0]));
        // Sun high (dusk 0): preset 2 shows.
        assert_eq!(fog_colour(&presets, 0.0, 0.9), Some([0.0, 0.0, 1.0]));
    }

    #[test]
    fn the_sun_at_noon_is_the_tilted_orbit_point() {
        let (a, b) = (3.28_f32, 0.5_f32);
        let d = sun_direction(0.5, 0.05, a, b);
        let expect = [a.sin() * b.sin(), b.sin() * a.cos(), b.cos()];
        let len = (expect[0] * expect[0] + expect[1] * expect[1] + expect[2] * expect[2]).sqrt();
        for i in 0..3 {
            assert!((d[i] - expect[i] / len).abs() < 1e-5);
        }
    }

    #[test]
    fn the_sun_matches_a_live_sample() {
        // p = 0.1403, a = 3.28, b = 0.5: the game's vector normalised to (0.9131, -0.0847, 0.3998).
        let d = sun_direction(0.140_303_42, 0.05, 3.28, 0.5);
        assert!((d[0] - 0.9131).abs() < 0.01, "{d:?}");
        assert!((d[1] + 0.0847).abs() < 0.01, "{d:?}");
        assert!((d[2] - 0.3998).abs() < 0.01, "{d:?}");
    }

    #[test]
    fn the_sun_is_a_unit_vector() {
        for k in 0..=20 {
            let p = 0.05 + 0.9 * k as f32 / 20.0;
            let d = sun_direction(p, 0.05, 3.28, 0.5);
            let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
            assert!((len - 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn the_phase_bounces_at_its_limits() {
        assert_eq!(advance_sun_phase(0.051, -1, 1.0, 1000.0, 0.05), (0.05, 1));
        assert_eq!(advance_sun_phase(0.949, 1, 1.0, 1000.0, 0.05), (0.95, -1));
        let (p, dir) = advance_sun_phase(0.5, 1, 1.0, 1.0 / 60.0, 0.05);
        assert_eq!(dir, 1);
        assert!(p > 0.5);
    }

    #[test]
    fn a_half_cycle_takes_eighteen_minutes() {
        // 0.9 of phase at 0.05 / 60 per second is 1080 s.
        let (mut p, mut dir) = (0.05_f32, 1);
        let dt = 1.0 / 60.0;
        for _ in 0..(1080 * 60) {
            (p, dir) = advance_sun_phase(p, dir, 1.0, dt, 0.05);
        }
        assert!((p - 0.95).abs() < 0.01, "{p}");
        assert_eq!(dir, 1);
    }

    const KEY_B: u32 = 2;

    fn preset(fields: &[(u32, Field)]) -> Preset {
        Preset { fields: fields.to_vec() }
    }

    #[test]
    fn scalars_and_vectors_interpolate_per_component() {
        let a = preset(&[(KEY_A, Field::Scalar(2.0)), (KEY_B, Field::Vector4([0.0, 1.0, 0.0, 1.0]))]);
        let b = preset(&[(KEY_A, Field::Scalar(4.0)), (KEY_B, Field::Vector4([1.0, 1.0, 0.0, 0.0]))]);
        let mid = blend(&a, &b, 0.5);
        assert_eq!(mid.get(KEY_A), Some(Field::Scalar(3.0)));
        assert_eq!(mid.get(KEY_B), Some(Field::Vector4([0.5, 1.0, 0.0, 0.5])));
    }

    #[test]
    fn the_weight_is_clamped_and_ends_are_exact() {
        let a = preset(&[(KEY_A, Field::Scalar(1.0))]);
        let b = preset(&[(KEY_A, Field::Scalar(5.0))]);
        assert_eq!(blend(&a, &b, -3.0).get(KEY_A), Some(Field::Scalar(1.0)));
        assert_eq!(blend(&a, &b, 9.0).get(KEY_A), Some(Field::Scalar(5.0)));
        assert_eq!(blend(&a, &b, f32::NAN).get(KEY_A), Some(Field::Scalar(1.0)));
    }

    #[test]
    fn a_field_in_only_one_preset_is_kept() {
        let a = preset(&[(KEY_A, Field::Scalar(1.0))]);
        let b = preset(&[(KEY_B, Field::Scalar(7.0))]);
        let out = blend(&a, &b, 0.5);
        assert_eq!(out.get(KEY_A), Some(Field::Scalar(1.0)));
        assert_eq!(out.get(KEY_B), Some(Field::Scalar(7.0)));
    }

    #[test]
    fn the_clock_is_a_one_hour_triangle() {
        assert_eq!(clock_phase(0, 0, 0), 0.0);
        assert_eq!(clock_phase(900, 0, 0), 0.5);
        assert_eq!(clock_phase(1800, 0, 0), 1.0);
        assert_eq!(clock_phase(2700, 0, 0), 0.5);
        assert_eq!(clock_phase(3600, 0, 0), 0.0);
        assert_eq!(clock_phase(3600 * 5 + 900, 0, 0), 0.5);
    }

    #[test]
    fn the_clock_offsets_shift_the_phase() {
        assert_eq!(clock_phase(0, 900_000, 0), 0.5);
        assert_eq!(clock_phase(900, 0, 900_000), 0.0);
    }

    #[test]
    fn an_empty_preset_is_not_usable() {
        assert!(!Preset::default().is_usable());
        assert!(preset(&[(KEY_A, Field::Scalar(0.0))]).is_usable());
    }

    #[test]
    #[ignore = "needs the base-game install selected by NFSMW_GAME_DIR"]
    fn base_game_has_eight_usable_presets() {
        let dir = game_install::GameDir::open(std::env::var("NFSMW_GAME_DIR").unwrap()).unwrap();
        let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
        let presets = load(&db).expect("the 8 installed presets");
        assert_eq!(presets.presets.len(), 8);
        assert!(presets.presets.iter().all(Preset::is_usable));
    }
}
