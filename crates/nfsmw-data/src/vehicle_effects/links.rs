use std::collections::HashMap;

use blackbox_attrib::{CollectionRef, Database, RefSpec, Value, vlt_hash};

use super::{EmitterStyle, sparks};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SparkLink {
    pub min: f32,
    pub max: f32,
    pub styles: [Option<EmitterStyle>; 2],
    pub inherit_velocity: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PcSparkLink {
    pub min: f32,
    pub max: f32,
    pub emitters: [Option<super::PcEmitter>; super::PC_EMITTERS],
    pub quadratic: [f32; 4],
    pub inherit_velocity: f32,
}

#[derive(Debug, Clone, Default)]
pub struct CollisionEffects {
    hit: HashMap<u32, Option<SparkLink>>,
    scrape: HashMap<u32, Option<SparkLink>>,
    pc_hit: HashMap<u32, Option<PcSparkLink>>,
    pc_scrape: HashMap<u32, Option<PcSparkLink>>,
    parents: HashMap<u32, Option<u32>>,
}

impl CollisionEffects {
    pub(super) fn read(db: &Database, car: CollectionRef<'_>) -> Self {
        Self {
            hit: read_links(db, car, "OnHitWorld", Record::link),
            scrape: read_links(db, car, "OnScrapeWorld", Record::link),
            pc_hit: read_links(db, car, "OnHitWorld", Record::pc_link),
            pc_scrape: read_links(db, car, "OnScrapeWorld", Record::pc_link),
            parents: db.collections_of("simsurface").map(|c| (c.key(), c.parent().map(|p| p.key()))).collect(),
        }
    }

    pub fn hit(&self, surface: u32) -> Option<SparkLink> {
        self.pick(&self.hit, surface)
    }

    pub fn scrape(&self, surface: u32) -> Option<SparkLink> {
        self.pick(&self.scrape, surface)
    }

    pub fn pc_hit(&self, surface: u32) -> Option<PcSparkLink> {
        self.pick(&self.pc_hit, surface)
    }
    pub fn pc_scrape(&self, surface: u32) -> Option<PcSparkLink> {
        self.pick(&self.pc_scrape, surface)
    }

    fn pick<T: Copy>(&self, links: &HashMap<u32, Option<T>>, mut surface: u32) -> Option<T> {
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
        let wrapper = db.resolve(self.effect)?;
        let inherit_velocity = wrapper.get_f32("InheritVelocity")?;
        if !inherit_velocity.is_finite() || !(0.0..=10.0).contains(&inherit_velocity) {
            return None;
        }
        let styles = sparks(db, wrapper);
        if styles.iter().all(Option::is_none) {
            return None;
        }
        Some(SparkLink { min: self.min, max: self.max, styles, inherit_velocity })
    }

    fn pc_link(&self, db: &Database) -> Option<PcSparkLink> {
        if self.effect.class != vlt_hash("effects")
            || !self.min.is_finite()
            || self.min < 0.0
            || !self.max.is_finite()
            || self.max <= self.min
        {
            return None;
        }
        let wrapper = db.resolve(self.effect)?;
        let inherit_velocity = wrapper.get_f32("InheritVelocity")?;
        let Value::Vector4(quadratic) = wrapper.get("EmitterQuadratic")? else { return None };
        if !inherit_velocity.is_finite()
            || !(0.0..=10.0).contains(&inherit_velocity)
            || quadratic.iter().any(|v| !v.is_finite())
        {
            return None;
        }
        let emitters = super::pc::read(db, wrapper);
        if emitters.iter().all(Option::is_none) {
            return None;
        }
        Some(PcSparkLink { min: self.min, max: self.max, emitters, quadratic: *quadratic, inherit_velocity })
    }
}

fn read_links<T>(
    db: &Database,
    car: CollectionRef<'_>,
    field: &str,
    pick: impl Fn(&Record, &Database) -> Option<T>,
) -> HashMap<u32, Option<T>> {
    let mut out = HashMap::new();
    let Some(values) = super::array(car, field) else { return out };
    for value in values {
        let Some(record) = Record::read(value) else { return HashMap::new() };
        if record.surface.class != vlt_hash("simsurface") || db.resolve(record.surface).is_none() {
            return HashMap::new();
        }
        out.entry(record.surface.collection).or_insert_with(|| pick(&record, db));
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
