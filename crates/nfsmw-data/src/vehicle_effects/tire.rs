//! Installed surface particle groups; selection rules are in `rain-pc.md`.
use super::{PC_EMITTERS, PcEmitter};
use blackbox_attrib::{Database, RefSpec, Value, vlt_hash};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub struct TireEffect {
    pub key: u32,
    pub min: f32,
    pub max: f32,
    pub emitters: [Option<PcEmitter>; PC_EMITTERS],
}
#[derive(Debug, Clone, Default)]
pub struct TireWaterData {
    surfaces: HashMap<u32, [Vec<Option<TireEffect>>; 3]>,
}
impl TireWaterData {
    pub fn read(db: &Database) -> Self {
        let surfaces = db
            .collections_of("simsurface")
            .map(|surface| {
                let records = ["TireDriveEffects", "TireSlipEffects", "TireSlideEffects"].map(|field| {
                    let Some(values) = surface.get(field).and_then(Value::as_array) else { return Vec::new() };
                    values.iter().take(3).map(|v| read_record(db, v)).collect()
                });
                (surface.key(), records)
            })
            .collect();
        Self { surfaces }
    }
    pub fn wet(&self, surface: u32, kind: usize, flat: bool) -> Option<TireEffect> {
        let records = self.surfaces.get(&surface)?.get(kind)?;
        let index = match (flat, records.len()) {
            (true, 3..) => 2,
            (_, 2..) => 1,
            _ => 0,
        };
        *records.get(index)?
    }
}
fn read_record(db: &Database, value: &Value) -> Option<TireEffect> {
    let Value::Raw { type_key, bytes } = value else { return None };
    if *type_key != vlt_hash("TireEffectRecord") || bytes.len() != 20 {
        return None;
    }
    let word = |i| u32::from_le_bytes(bytes[i..i + 4].try_into().expect("validated record"));
    let reference = RefSpec { class: word(0), collection: word(4) };
    if reference.class != vlt_hash("emittergroup") {
        return None;
    }
    let (min, max) = (f32::from_bits(word(12)), f32::from_bits(word(16)));
    if !min.is_finite() || !max.is_finite() || max <= min {
        return None;
    }
    let group = db.resolve(reference)?;
    let emitters = super::pc::read_group(db, group);
    if emitters.iter().all(Option::is_none) {
        return None;
    }
    Some(TireEffect { key: group.key(), min, max, emitters })
}
