//! A car's collision sounds: the `audioimpact` and `audioscrape` collections its `pvehicle` links per event
//! kind and surface. Rules: `docs/specs/engine-sound-effects.md` §8.

use blackbox_attrib::{CollectionRef, Database, Value, vlt_hash};

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
}

impl EventKind {
    pub const ALL: [EventKind; 8] = [
        Self::HitGround,
        Self::HitWorld,
        Self::HitObject,
        Self::BottomOut,
        Self::ScrapeGround,
        Self::ScrapeWorld,
        Self::ScrapeObject,
        Self::BottomScrape,
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
}

/// All of a car's collision sounds.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CollisionSounds {
    links: Vec<Link>,
}

impl CollisionSounds {
    /// Reads the links of `pvehicle` car `type_name`; empty when the car or the links are missing.
    pub fn read(db: &Database, type_name: &str) -> Self {
        let Some(car) = db.collection("pvehicle", &type_name.to_ascii_lowercase()) else { return Self::default() };
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
                links.push(Link { kind, surface: word(bytes, 4), sounds });
            }
        }
        Self { links }
    }

    /// The collection for an event of `kind` on `surface` (a `simsurface` hash): the link for that surface, else
    /// the one for `default`, else the first; of its collections the one for a `front` or a side hit when it
    /// has one, else the first.
    pub fn pick(&self, kind: EventKind, surface: u32, default: u32, front: bool) -> Option<&ImpactSound> {
        let of_kind = || self.links.iter().filter(move |l| l.kind == kind);
        let link = of_kind().find(|l| l.surface == surface).or_else(|| of_kind().find(|l| l.surface == default));
        let link = link.or_else(|| of_kind().next())?;
        link.sounds.iter().find(|s| s.has("FRONT") == front).or_else(|| link.sounds.first())
    }

    pub fn is_empty(&self) -> bool {
        self.links.is_empty()
    }
}
