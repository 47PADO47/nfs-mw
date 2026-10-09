use std::collections::HashMap;

use blackbox_attrib::{CollectionRef, Database, RefSpec, Value, vlt_hash};

use super::{EmitterStyle, sparks};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SparkLink {
    pub min: f32,
    pub max: f32,
    pub styles: [Option<EmitterStyle>; 2],
}

#[derive(Debug, Clone, Default)]
pub struct CollisionEffects {
    hit: HashMap<u32, Option<SparkLink>>,
    scrape: HashMap<u32, Option<SparkLink>>,
    parents: HashMap<u32, Option<u32>>,
}

impl CollisionEffects {
    pub(super) fn read(db: &Database, car: CollectionRef<'_>) -> Self {
        Self {
            hit: read_links(db, car, "OnHitWorld"),
            scrape: read_links(db, car, "OnScrapeWorld"),
            parents: db.collections_of("simsurface").map(|c| (c.key(), c.parent().map(|p| p.key()))).collect(),
        }
    }

    pub fn hit(&self, surface: u32) -> Option<SparkLink> {
        self.pick(&self.hit, surface)
    }

    pub fn scrape(&self, surface: u32) -> Option<SparkLink> {
        self.pick(&self.scrape, surface)
    }

    fn pick(&self, links: &HashMap<u32, Option<SparkLink>>, mut surface: u32) -> Option<SparkLink> {
        // Native world lookup resolves missing hashes to unknown; the effect lookup seeds default
        // before traversing any surface. Missing prop metadata is excluded by the caller instead.
        if !self.parents.contains_key(&surface) {
            surface = vlt_hash("unknown");
        }
        for _ in 0..64 {
            if surface == vlt_hash("null") {
                return links.get(&vlt_hash("default")).copied().flatten();
            }
            let Some(parent) = self.parents.get(&surface) else {
                return links.get(&vlt_hash("default")).copied().flatten();
            };
            // An explicit unsupported effect stops lookup, so wood never inherits metal streaks.
            if let Some(link) = links.get(&surface) {
                return *link;
            }
            let Some(parent) = parent else { return links.get(&vlt_hash("default")).copied().flatten() };
            surface = *parent;
        }
        // A malformed ancestry cycle cannot establish a native override.
        None
    }
}

struct Record {
    surface: RefSpec,
    effect: RefSpec,
    min: f32,
    max: f32,
}

impl Record {
    fn read(value: &Value) -> Option<Self> {
        let Value::Raw { type_key, bytes } = value else { return None };
        if *type_key != vlt_hash("EffectLinkageRecord") || bytes.len() != 32 {
            return None;
        }
        let word = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().expect("validated record length"));
        Some(Self {
            surface: RefSpec { class: word(0), collection: word(4) },
            effect: RefSpec { class: word(12), collection: word(16) },
            min: f32::from_bits(word(24)),
            max: f32::from_bits(word(28)),
        })
    }

    fn link(&self, db: &Database) -> Option<SparkLink> {
        if self.effect.class != vlt_hash("effects")
            || !self.min.is_finite()
            || self.min < 0.0
            || !self.max.is_finite()
            || self.max <= self.min
        {
            return None;
        }
        let styles = sparks(db, db.resolve(self.effect)?);
        if styles.iter().all(Option::is_none) {
            return None;
        }
        Some(SparkLink { min: self.min, max: self.max, styles })
    }
}

fn read_links(db: &Database, car: CollectionRef<'_>, field: &str) -> HashMap<u32, Option<SparkLink>> {
    let mut out = HashMap::new();
    let Some(values) = super::array(car, field) else { return out };
    for value in values {
        let Some(record) = Record::read(value) else { return HashMap::new() };
        if record.surface.class != vlt_hash("simsurface") || db.resolve(record.surface).is_none() {
            return HashMap::new();
        }
        out.entry(record.surface.collection).or_insert_with(|| record.link(db));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_type_and_exact_size_are_required_before_reading_references() {
        for size in [0, 12, 31, 33, 64] {
            let value = Value::Raw { type_key: vlt_hash("EffectLinkageRecord"), bytes: vec![0; size] };
            assert!(Record::read(&value).is_none());
        }
        let value = Value::Raw { type_key: vlt_hash("OtherRecord"), bytes: vec![0; 32] };
        assert!(Record::read(&value).is_none());
    }
}
