//! Optional collision streaks and contrails, selected from the installed attribute database.
//! Spec and evidence: `docs/specs/vehicle-visual-effects.md`.

#[cfg(test)]
#[allow(dead_code)]
mod fixture;
mod links;
mod style;
#[cfg(test)]
mod synthetic;
#[cfg(test)]
mod tests;

pub use links::{CollisionEffects, SparkLink};
pub use style::EmitterStyle;

use blackbox_attrib::{CollectionRef, Database, Value, vlt_hash};

#[derive(Debug, Clone, Default)]
pub struct VisualEffectsData {
    pub collision: CollisionEffects,
    pub trail: Option<EmitterStyle>,
}

impl VisualEffectsData {
    pub fn read(db: &Database, car_type: &str) -> Self {
        let Some(car) = db.collection("pvehicle", &car_type.to_ascii_lowercase()) else { return Self::default() };
        let trail = db
            .collection("fuelcell_effect", "contrail")
            .and_then(|effect| styles(db, effect, &[vlt_hash("trail3")]).into_iter().next().flatten());
        Self { collision: CollisionEffects::read(db, car), trail }
    }
}

fn resolve<'a>(db: &'a Database, value: &Value, class: &str) -> Option<CollectionRef<'a>> {
    let reference = value.as_ref_spec()?;
    if reference.class != vlt_hash(class) {
        return None;
    }
    db.resolve(reference)
}

fn array<'a>(collection: CollectionRef<'a>, field: &str) -> Option<&'a [Value]> {
    let values = collection.get(field)?.as_array()?;
    (values.len() <= 64).then_some(values)
}

fn styles<const N: usize>(db: &Database, effect: CollectionRef<'_>, keys: &[u32; N]) -> [Option<EmitterStyle>; N] {
    let mut out = [None; N];
    let Some(emitters) = array(effect, "NGEmitter") else { return out };
    for value in emitters {
        let Some(emitter) = resolve(db, value, "fuelcell_emitter") else { continue };
        let Some(index) = keys.iter().position(|&key| key == emitter.key()) else { continue };
        out[index] = EmitterStyle::read(emitter);
    }
    out
}

fn sparks(db: &Database, wrapper: CollectionRef<'_>) -> [Option<EmitterStyle>; 2] {
    let Some(group) = wrapper.get("emittergroup").and_then(|v| resolve(db, v, "emittergroup")) else {
        return [None; 2];
    };
    let Some(emitters) = array(group, "Emitters") else { return [None; 2] };
    for value in emitters {
        let Some(emitter) = resolve(db, value, "emitterdata") else { continue };
        let Some(effects) = array(emitter, "XenonEffect") else { continue };
        for value in effects {
            let Some(effect) = resolve(db, value, "fuelcell_effect") else { continue };
            if effect.key() != vlt_hash("fxsprk_line") {
                continue;
            }
            let found = styles(db, effect, &[vlt_hash("emsprk_line1"), vlt_hash("emsprk_line2")]);
            if found.iter().any(Option::is_some) {
                return found;
            }
        }
    }
    [None; 2]
}
