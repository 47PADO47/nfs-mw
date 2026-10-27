//! Source-owned contact links, including non-spark groups.
use super::{PcSparkLink, links::Record};
use blackbox_attrib::{CollectionRef, Database, vlt_hash};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    HitWorld,
    HitObject,
    HitGround,
    ScrapeWorld,
    ScrapeObject,
    ScrapeGround,
    BottomOut,
    BottomScrape,
    Detached,
}

impl Action {
    pub const ALL: [Self; 9] = [
        Self::HitWorld,
        Self::HitObject,
        Self::HitGround,
        Self::ScrapeWorld,
        Self::ScrapeObject,
        Self::ScrapeGround,
        Self::BottomOut,
        Self::BottomScrape,
        Self::Detached,
    ];
    pub fn field(self) -> &'static str {
        match self {
            Self::HitWorld => "OnHitWorld",
            Self::HitObject => "OnHitObject",
            Self::HitGround => "OnHitGround",
            Self::ScrapeWorld => "OnScrapeWorld",
            Self::ScrapeObject => "OnScrapeObject",
            Self::ScrapeGround => "OnScrapeGround",
            Self::BottomOut => "OnBottomOut",
            Self::BottomScrape => "OnBottomScrape",
            Self::Detached => "OnDetached",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Link {
    pub effect: u32,
    pub min: f32,
    pub max: f32,
    pub visual_cull: f32,
    pub particles: Option<PcSparkLink>,
}

impl Link {
    pub fn hit(self, closing_squared: f32, impact: f32) -> Option<f32> {
        if !closing_squared.is_finite() || closing_squared <= 1.0 || !impact.is_finite() {
            return None;
        }
        let ramp = ((impact - self.min) / (self.max - self.min)).clamp(0.0, 1.0);
        (ramp > 0.0).then_some(ramp)
    }
    pub fn scrape(self, speed: f32) -> Option<f32> {
        if !speed.is_finite() || speed <= 1.0 || speed < self.min {
            return None;
        }
        Some(((speed - self.min) / (self.max - self.min)).clamp(0.1, 1.0))
    }
    pub fn intensity(self, magnitude: f32) -> f32 {
        let Some(p) = self.particles else { return 0.0 };
        let [a, b, c, d] = p.quadratic;
        (a + magnitude * (b + magnitude * (c + magnitude * d))).clamp(0.0, 1.0)
    }
}

#[derive(Default)]
pub struct SourceEffects {
    links: HashMap<u32, HashMap<u32, Option<Link>>>,
    parents: HashMap<u32, Option<u32>>,
}

impl SourceEffects {
    pub fn read(db: &Database, source: CollectionRef<'_>) -> Self {
        let mut out = Self {
            links: HashMap::new(),
            parents: db.collections_of("simsurface").map(|c| (c.key(), c.parent().map(|p| p.key()))).collect(),
        };
        for action in Action::ALL {
            let mut links = HashMap::new();
            for value in super::array(source, action.field()).unwrap_or_default() {
                let Some(record) = Record::read(value) else {
                    links.clear();
                    break;
                };
                if record.surface.class != vlt_hash("simsurface") || db.resolve(record.surface).is_none() {
                    links.clear();
                    break;
                }
                let link = (|| {
                    if record.effect.class != vlt_hash("effects")
                        || !record.min.is_finite()
                        || !record.max.is_finite()
                        || record.max <= record.min
                    {
                        return None;
                    }
                    let effect = db.resolve(record.effect)?;
                    let visual_cull = effect.get_f32("VisualCullDist")?;
                    if !visual_cull.is_finite() || visual_cull < 0.0 {
                        return None;
                    }
                    Some(Link {
                        effect: effect.key(),
                        min: record.min,
                        max: record.max,
                        visual_cull,
                        particles: record.pc_link(db),
                    })
                })();
                links.entry(record.surface.collection).or_insert(link);
            }
            out.links.insert(vlt_hash(action.field()), links);
        }
        out
    }
    /// These effects with `link` for `action` on the surface `surface` (the `default` surface answers every surface that has no
    /// ancestry): the effects of a source that is not in the database, for the code that tests with them.
    pub fn with_link(mut self, action: Action, surface: u32, link: Option<Link>) -> Self {
        self.links.entry(vlt_hash(action.field())).or_default().insert(surface, link);
        self
    }
    pub fn pick(&self, action: Action, mut surface: u32) -> Option<Link> {
        let links = self.links.get(&vlt_hash(action.field()))?;
        if !self.parents.contains_key(&surface) {
            surface = vlt_hash("unknown");
        }
        for _ in 0..64 {
            if surface == vlt_hash("null") {
                return links.get(&vlt_hash("default")).copied().flatten();
            }
            if let Some(link) = links.get(&surface) {
                return *link;
            }
            let Some(Some(parent)) = self.parents.get(&surface) else {
                return links.get(&vlt_hash("default")).copied().flatten();
            };
            surface = *parent;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn physical_acceptance_is_independent_of_particle_support_and_scrapes_keep_the_source_floor() {
        let link = Link { effect: 1, min: 5.0, max: 15.0, visual_cull: 20.0, particles: None };
        assert_eq!(link.hit(1.0, 15.0), None);
        assert_eq!(link.hit(4.0, 5.0), None);
        assert_eq!(link.hit(4.0, 10.0), Some(0.5));
        assert_eq!(link.scrape(5.0), Some(0.1));
        assert_eq!(link.scrape(4.0), None);
        assert_eq!(link.intensity(1.0), 0.0);
    }
    #[test]
    #[ignore = "needs the game (set NFSMW_GAME_DIR)"]
    fn installed_cars_have_particle_groups_for_world_hits_and_scrapes_on_every_surface() {
        let path = std::env::var_os(crate::game::SPEC.env_var).expect("set NFSMW_GAME_DIR");
        let dir = game_install::GameDir::open(std::path::PathBuf::from(path)).unwrap();
        let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
        for car in ["bmwm3gtr", "cobaltss", "copmidsize", "traftaxi"] {
            let profile = SourceEffects::read(&db, db.collection("pvehicle", car).unwrap());
            let surfaces = db.collections_of("simsurface").map(|surface| surface.key()).chain([0]);
            for surface in surfaces {
                // A wall of any material (or none) gets a group; a hit starts at 0.5 or 1 m/s of reaction, a scrape at 5 m/s.
                let hit =
                    profile.pick(Action::HitWorld, surface).unwrap_or_else(|| panic!("{car}: no hit on {surface:#x}"));
                let scrape = profile.pick(Action::ScrapeWorld, surface).expect("a scrape link");
                assert!(hit.particles.is_some() && scrape.particles.is_some(), "{car} {surface:#x}");
                assert!(hit.min <= 1.0 && scrape.min == 5.0 && hit.visual_cull >= 100.0, "{car} {surface:#x}");
            }
        }
    }
    #[test]
    fn source_contact_lookup_respects_unknown_ancestry_and_explicit_null_override() {
        let default = vlt_hash("default");
        let unknown = vlt_hash("unknown");
        let parent = vlt_hash("parent");
        let child = vlt_hash("child");
        let link = Link { effect: 1, min: 0.0, max: 10.0, visual_cull: 20.0, particles: None };
        let mut profile = SourceEffects {
            parents: HashMap::from([(unknown, None), (parent, None), (child, Some(parent))]),
            ..Default::default()
        };
        profile
            .links
            .insert(vlt_hash(Action::HitWorld.field()), HashMap::from([(default, Some(link)), (parent, None)]));
        assert!(profile.pick(Action::HitWorld, child).is_none(), "explicit null blocks ancestor/default substitution");
        assert_eq!(
            profile.pick(Action::HitWorld, 123).unwrap().effect,
            1,
            "unknown uses default when it has no override"
        );
        profile.parents.insert(unknown, Some(unknown));
        assert!(profile.pick(Action::HitWorld, 123).is_none(), "malformed ancestry cycles fail closed");
    }
}
