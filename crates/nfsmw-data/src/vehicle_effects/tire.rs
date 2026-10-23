//! Installed surface particle groups; selection rules are in `rain-pc.md` and `tire-surface-dust.md`.
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
/// The one dry emitter that paved surfaces keep: the leaves a sliding tire throws up. Their other dry groups are
/// tire smoke, which this port draws procedurally (`tire-effects.md`).
const LEAVES: &str = "emtd_dr_leaves";

#[derive(Debug, Clone, Default)]
struct Surface {
    records: [Vec<Option<TireEffect>>; 3],
    /// Loose ground (grass, sand, dirt, gravel, water): the dry driving record exists, so every dry record is used.
    loose: bool,
}
#[derive(Debug, Clone, Default)]
pub struct TireWaterData {
    surfaces: HashMap<u32, Surface>,
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
                let loose = records[0].first().is_some_and(Option::is_some);
                (surface.key(), Surface { records, loose })
            })
            .collect();
        Self { surfaces }
    }
    /// The group a tire on `surface` runs for one of drive (0), slip (1) and slide (2). `wet` is rain on the car and
    /// takes the surface's wet record when it has one, else its dry record. `flat` (a blown tire) is honoured only
    /// when wet: dry sparks of a blown tire come from the damage effects. Dry paved surfaces run leaves only.
    pub fn effect(&self, surface: u32, kind: usize, wet: bool, flat: bool) -> Option<TireEffect> {
        let surface = self.surfaces.get(&surface)?;
        let records = surface.records.get(kind)?;
        let index = match (wet && flat, wet, records.len()) {
            (true, _, 3..) => 2,
            (_, true, 2..) => 1,
            _ => 0,
        };
        let effect = (*records.get(index)?)?;
        if wet || surface.loose {
            return Some(effect);
        }
        effect.leaves_only()
    }
}
impl TireEffect {
    fn leaves_only(mut self) -> Option<Self> {
        let leaves = vlt_hash(LEAVES);
        for slot in &mut self.emitters {
            *slot = slot.filter(|emitter| emitter.key == leaves);
        }
        self.emitters.iter().any(Option::is_some).then_some(self)
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
