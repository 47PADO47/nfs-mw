//! A car's collision sounds: the `audioimpact` and `audioscrape` collections its `pvehicle` links per event
//! kind and surface. Rules: `docs/specs/engine-sound-effects.md` §8.

use blackbox_attrib::{CollectionRef, Database, Value, vlt_hash};
use std::collections::HashMap;

use super::fields::Fields;

/// What happened, which also decides the list of links used (`OnHitGround`, ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventKind {
    HitGround,
    HitWorld,
    HitObject,
    BottomOut,
    ScrapeGround,
    ScrapeWorld,
    ScrapeObject,
    BottomScrape,
    Detached,
}

impl EventKind {
    pub const ALL: [EventKind; 9] = [
        Self::HitGround,
        Self::HitWorld,
        Self::HitObject,
        Self::BottomOut,
        Self::ScrapeGround,
        Self::ScrapeWorld,
        Self::ScrapeObject,
        Self::BottomScrape,
        Self::Detached,
    ];

    /// The `pvehicle` field with this kind's links.
    pub fn field(self) -> &'static str {
        match self {
            Self::HitGround => "OnHitGround",
            Self::HitWorld => "OnHitWorld",
            Self::HitObject => "OnHitObject",
            Self::BottomOut => "OnBottomOut",
            Self::ScrapeGround => "OnScrapeGround",
            Self::ScrapeWorld => "OnScrapeWorld",
            Self::ScrapeObject => "OnScrapeObject",
            Self::BottomScrape => "OnBottomScrape",
            Self::Detached => "OnDetached",
        }
    }
}

/// One `audioimpact` or `audioscrape` collection.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ImpactSound {
    pub name: String,
    /// `STITCH_LEVEL_0..3`: stitch ids, quietest to loudest; empty lists are unused.
    pub levels: [Vec<u32>; 4],
    /// `Volumes.Vol1..Vol4`, 0 to 1 (`0x4000` in the data is a half).
    pub volumes: [f32; 4],
    /// `DESCRIPTION[ ]`: `CAR`, `WALL`, `SMOKABLE`, ...
    pub description: Vec<String>,
    /// Authored continuous-effect identifier, not a bank sample index.
    pub scrape_effect: String,
}

fn word(bytes: &[u8], at: usize) -> u32 {
    bytes.get(at..at + 4).map_or(0, |s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

impl ImpactSound {
    fn read(c: CollectionRef<'_>) -> Self {
        let fields = Fields(c);
        let level = |name: &str| -> Vec<u32> { fields.raw_items(name).iter().map(|b| word(b, 0)).collect() };
        let volumes = fields.raw_items("Volumes").first().map_or([0.0; 4], |b| {
            std::array::from_fn(|i| {
                b.get(i * 2..i * 2 + 2).map_or(0.0, |s| f32::from(u16::from_le_bytes([s[0], s[1]])) / 32768.0)
            })
        });
        let key_string = |v: &Value| match v {
            Value::StringKey(k) => k.string.clone(),
            other => other.as_str().map(str::to_owned),
        };
        let description = match c.get("DESCRIPTION") {
            Some(Value::Array(items)) => items.iter().filter_map(key_string).collect(),
            Some(one) => key_string(one).into_iter().collect(),
            None => Vec::new(),
        };
        Self {
            name: fields.name(),
            levels: [
                level("STITCH_LEVEL_0"),
                level("STITCH_LEVEL_1"),
                level("STITCH_LEVEL_2"),
                level("STITCH_LEVEL_3"),
            ],
            volumes,
            description,
            scrape_effect: fields.string("CSIS_EFFECT"),
        }
    }

    /// Whether `DESCRIPTION` has `flag` (`WALL`, `SMOKABLE`, `CAR`).
    pub fn has(&self, flag: &str) -> bool {
        self.description.iter().any(|d| d.eq_ignore_ascii_case(flag))
    }
}

/// The `audioimpact` or `audioscrape` collections a link's wrapper collection (`carhitwall`) points at: one
/// for a hit on the car's side and others for a hit on its front, as the wrapper's own references (not the
/// inherited ones) list them.
fn sounds_of<'a>(db: &'a Database, wrapper: CollectionRef<'a>) -> Vec<CollectionRef<'a>> {
    let classes = [vlt_hash("audioimpact"), vlt_hash("audioscrape")];
    wrapper
        .attributes()
        .iter()
        .filter_map(|a| match &a.value {
            Value::RefSpec(r) if classes.contains(&r.class) => db.resolve(*r),
            _ => None,
        })
        .collect()
}

/// A link: the surface (or object) it applies to and the collection it names.
#[derive(Debug, Clone, PartialEq)]
struct Link {
    kind: EventKind,
    /// The `simsurface` name hash the link is for.
    surface: u32,
    /// The side hit first, then the front hits.
    sounds: Vec<ImpactSound>,
    minimum: f32,
    maximum: f32,
}

/// An authored collision rule and its input range (velocity change for hits, sliding speed for scrapes).
#[derive(Debug, Clone, Copy)]
pub struct CollisionRule<'a> {
    pub sound: &'a ImpactSound,
    pub minimum: f32,
    pub maximum: f32,
}

impl CollisionRule<'_> {
    pub fn magnitude(&self, value: f32) -> Option<f32> {
        if !value.is_finite()
            || !self.minimum.is_finite()
            || !self.maximum.is_finite()
            || self.maximum - self.minimum <= 0.000001
            || value < self.minimum
        {
            return None;
        }
        Some(((value - self.minimum) / (self.maximum - self.minimum)).clamp(0.0, 1.0))
    }
}

/// All of a car's collision sounds.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CollisionSounds {
    links: Vec<Link>,
    parents: HashMap<u32, u32>,
}

impl CollisionSounds {
    /// Reads the links of `pvehicle` car `type_name`; empty when the car or the links are missing.
    pub fn read(db: &Database, type_name: &str) -> Self {
        let Some(car) = db.collection("pvehicle", &type_name.to_ascii_lowercase()) else { return Self::default() };
        Self::from_collection(db, car)
    }

    pub fn prop(db: &Database, profile: u32) -> Self {
        let Some(prop) = db.collection_by_key(vlt_hash("smackable"), profile) else { return Self::default() };
        Self::from_collection(db, prop)
    }

    fn from_collection(db: &Database, car: CollectionRef<'_>) -> Self {
        let mut links = Vec::new();
        for kind in EventKind::ALL {
            let Some(Value::Array(items)) = car.get(kind.field()) else { continue };
            for item in items {
                let Value::Raw { bytes, .. } = item else { continue };
                // {selector class, selector key, 0, sound class, sound key, 0, min, max}
                let Some(wrapper) = db.collection_by_key(word(bytes, 12), word(bytes, 16)) else { continue };
                let name = Fields(wrapper).name();
                let sounds: Vec<ImpactSound> = sounds_of(db, wrapper)
                    .into_iter()
                    .map(|c| ImpactSound { name: name.clone(), ..ImpactSound::read(c) })
                    .collect();
                if sounds.is_empty() {
                    continue;
                }
                links.push(Link {
                    kind,
                    surface: word(bytes, 4),
                    sounds,
                    minimum: f32::from_bits(word(bytes, 24)),
                    maximum: f32::from_bits(word(bytes, 28)),
                });
            }
        }
        let parents = db
            .collections_of("simsurface")
            .filter_map(|surface| Some((surface.key(), surface.parent()?.key())))
            .collect();
        Self { links, parents }
    }

    /// The collection for an event of `kind` on `surface` (a `simsurface` hash): the link for that surface, else
    /// its parent surfaces, then `default`; of its collections the one for a `front` or a side hit when it
    /// has one, else the first.
    pub fn pick(&self, kind: EventKind, surface: u32, default: u32, front: bool) -> Option<&ImpactSound> {
        self.rule(kind, surface, default, front).map(|rule| rule.sound)
    }

    pub fn rule(&self, kind: EventKind, mut surface: u32, default: u32, front: bool) -> Option<CollisionRule<'_>> {
        // A malformed ancestry cycle cannot hang sound playback.
        for _ in 0..=self.parents.len() {
            if let Some(link) = self.links.iter().find(|l| l.kind == kind && l.surface == surface) {
                return Self::selected(link, front);
            }
            let Some(parent) = self.parents.get(&surface).copied() else { break };
            surface = parent;
        }
        let link = self.links.iter().find(|l| l.kind == kind && l.surface == default)?;
        Self::selected(link, front)
    }

    fn selected(link: &Link, front: bool) -> Option<CollisionRule<'_>> {
        let sound = link.sounds.iter().find(|s| s.has("FRONT") == front).or_else(|| link.sounds.first())?;
        Some(CollisionRule { sound, minimum: link.minimum, maximum: link.maximum })
    }

    pub fn is_empty(&self) -> bool {
        self.links.is_empty()
    }
}

#[cfg(test)]
mod tests;
