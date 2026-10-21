//! The pursuit breakers among the resident scenery: which placed models open a cop zone when they collapse, where they
//! stand and whether they already fired. A diagnostic and console view of the event data; the breakers themselves run
//! through the scenery sequences (`effects.rs`). Spec: `docs/specs/pursuit-breakers.md`.

use std::collections::{BTreeMap, VecDeque};

use glam::{Vec2, Vec3};
use nfsmw_data::{car::physics::PhysicsData, world::sequences::BreakerProfile};

use super::PropWorld;

/// How many recent breaker events the world remembers for the console.
const HISTORY: usize = 8;

/// One placed model that opens a cop zone.
#[derive(Debug, Clone, PartialEq)]
pub struct BreakerSite {
    pub id: u32,
    /// The name of its event engine, e.g. `gasstationroof_seq`.
    pub engine: String,
    /// Physics space.
    pub position: Vec3,
    /// The largest zone it opens: radius (m) and duration (s).
    pub zone: (f32, f32),
    /// The cinematic moments its sequence names (the original plays them as scripted scenes).
    pub moments: Vec<String>,
    /// How many trigger parts (supports, pillars) stand within their reach of it.
    pub supports: usize,
    /// It already opened its zone since it was placed.
    pub fired: bool,
}

impl PropWorld {
    /// The resident breakers, nearest to `from` (physics x and z) first.
    pub fn breaker_sites(&self, data: &PhysicsData, from: Vec2) -> Vec<BreakerSite> {
        let mut profiles: BTreeMap<u32, Option<(String, BreakerProfile)>> = BTreeMap::new();
        let mut profile = |key: u32| {
            profiles
                .entry(key)
                .or_insert_with(|| {
                    let sequence = data.sequence(key)?;
                    let name = sequence.package.engines[sequence.engine].name.clone();
                    Some((name, sequence.breaker_profile()))
                })
                .clone()
        };
        let mut triggers = Vec::new();
        let mut sites = Vec::new();
        for (&id, model) in &self.scenery.models {
            // A fragment is not a placed breaker: it lives only until its parent goes.
            if model.parent_owner.is_none() || model.key == 0 {
                continue;
            }
            let Some(position) = self.colliders.get(&id).and_then(|c| c.body.as_ref()).map(|b| b.rigid.position) else {
                continue;
            };
            let Some((name, profile)) = profile(model.key) else { continue };
            if profile.is_trigger() {
                triggers.push((position, profile.reach));
                continue;
            }
            let Some(zone) = profile.largest_stop() else { continue };
            sites.push(BreakerSite {
                id,
                engine: name,
                position,
                zone,
                moments: profile.moments,
                supports: 0,
                fired: model.fired,
            });
        }
        for site in &mut sites {
            site.supports = triggers.iter().filter(|(at, reach)| at.distance(site.position) <= *reach).count();
        }
        let away = |site: &BreakerSite| Vec2::new(site.position.x, site.position.z).distance(from);
        sites.sort_by(|a, b| away(a).total_cmp(&away(b)).then(a.id.cmp(&b.id)));
        sites
    }

    /// The last breaker events, oldest first (a zone opened, a moment named).
    pub fn breaker_history(&self) -> &VecDeque<String> {
        &self.scenery.history
    }

    pub(super) fn note_breaker(&mut self, line: String) {
        log::info!("pursuit breaker: {line}");
        if self.scenery.history.len() == HISTORY {
            self.scenery.history.pop_front();
        }
        self.scenery.history.push_back(line);
    }
}
