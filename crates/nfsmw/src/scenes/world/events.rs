//! Vehicle damage callbacks drive the installed sequencer on the shared world clock.
//! Spec: `docs/specs/world-sequencers.md`.

mod damage;
mod dispatch;
pub(super) use dispatch::{drain, queue};
#[cfg(test)]
mod tests;
pub use damage::{DamageHit, OtherBody};

use std::{collections::BTreeSet, rc::Rc, sync::Arc};

use anyhow::Result;
use blackbox_attrib::vlt_hash;
use blackbox_damage::Damage;
use blackbox_sequence::{Player, QueueMode};
use glam::Vec3;
use nfsmw_data::car::damage::{PUNCTURE_SEVERITY, ZONE_HAS_GLASS, ZONE_WHEEL, revealed_overlays};
use nfsmw_data::car::physics::{CarEventSpec, DAMAGE_ZONES};
use nfsmw_data::world::sequences::{TAGS, WorldEvent};

pub struct Effect {
    pub event: WorldEvent,
    /// Immediate damage callbacks supply pre-impact velocity. Timed/forwarded events use the live model.
    pub velocity: Option<Vec3>,
}

pub struct VehicleEvents {
    spec: Arc<CarEventSpec>,
    /// The car's sequencer, when it has one. Damage (the model and the visible overlays) does not need it.
    player: Option<Player>,
    hits: Vec<DamageHit>,
    failed: bool,
    pub hidden_slots: BTreeSet<usize>,
    pub spawned: BTreeSet<u32>,
    /// Child models live until their original parent is destroyed, independently of scenery tiles.
    pub owner: Rc<()>,
    /// Visible damage accumulated from impacts; it lasts for the car's life in this session.
    pub damage: Damage,
}

impl VehicleEvents {
    pub fn new(spec: Arc<CarEventSpec>) -> Result<Self> {
        let player = spec.sequence.as_ref().map(|s| Player::new(&s.package, s.engine, TAGS)).transpose()?;
        Ok(Self {
            spec,
            player,
            hits: Vec::new(),
            failed: false,
            hidden_slots: BTreeSet::new(),
            spawned: BTreeSet::new(),
            owner: Rc::new(()),
            damage: Damage::new(DAMAGE_ZONES.len()),
        })
    }

    /// Which tires are punctured, in the vehicle's wheel order.
    pub fn punctured(&self) -> [bool; 4] {
        std::array::from_fn(|wheel| self.damage.is_punctured(wheel))
    }

    /// The corner damage overlays to draw: those of zones whose panel has dented.
    pub fn revealed_slots(&self) -> BTreeSet<usize> {
        revealed_overlays(&self.damage)
    }

    pub fn hit(&mut self, hit: DamageHit) {
        if !self.failed && !hit.other.no_car_effect && hit.closing_speed >= 1.0 {
            self.hits.push(hit);
        }
    }

    #[cfg(test)]
    pub(in crate::scenes::world) fn pending_hits(&self) -> &[DamageHit] {
        &self.hits
    }

    pub fn tick(&mut self, view_distance: f32) -> Vec<Effect> {
        if self.failed {
            return Vec::new();
        }
        match self.advance(view_distance) {
            Ok(events) => events,
            Err(error) => {
                self.failed = true;
                self.hits.clear();
                log::warn!("vehicle event engine stopped: {error:#}");
                Vec::new()
            }
        }
    }

    fn advance(&mut self, view_distance: f32) -> Result<Vec<Effect>> {
        let mut out = Vec::new();
        if let (Some(sequence), Some(player)) = (&self.spec.sequence, self.player.as_mut()) {
            player.advance(&sequence.package, 1.0 / 60.0)?;
            drain(sequence, player, None, &mut out)?;
        }
        let hits = std::mem::take(&mut self.hits);
        for hit in hits {
            if self.spec.suppress_distance > 0.0 && view_distance > self.spec.suppress_distance {
                continue;
            }
            if self.spec.strength <= 0.0 || !hit.force.is_finite() {
                continue;
            }
            let zone = hit.zone();
            let target = Some(vlt_hash(DAMAGE_ZONES[zone]));
            let mut stimuli = vec![vlt_hash("COLLISION")];
            if hit.other.vehicle {
                stimuli.push(vlt_hash("COLLISION_VEHICLE"));
            }
            if hit.other.cop {
                stimuli.push(vlt_hash("COLLISION_COP"));
            }
            if hit.other.player {
                stimuli.push(vlt_hash("COLLISION_PLAYER"));
            }
            let level = (hit.force / self.spec.strength * self.spec.visual_scale[zone]).clamp(0.0, 6.0) as usize;
            log::debug!("vehicle damage {}: force {:.1}, impact level {level}", DAMAGE_ZONES[zone], hit.force);
            if level >= PUNCTURE_SEVERITY
                && let Some(wheel) = ZONE_WHEEL[zone]
                && self.damage.puncture(wheel)
            {
                log::info!("tire {wheel} punctured by a hit on {}", DAMAGE_ZONES[zone]);
            }
            if self.damage.impact(zone, level as u8, ZONE_HAS_GLASS[zone]) {
                log::info!("vehicle damage {} now {:?}", DAMAGE_ZONES[zone], self.damage.zone(zone));
            }
            stimuli.extend((1..=level).map(|n| vlt_hash(&format!("IMPACT_{n}"))));
            if let (Some(sequence), Some(player)) = (&self.spec.sequence, self.player.as_mut()) {
                for stimulus in stimuli {
                    player.stimulate(&sequence.package, target, stimulus, QueueMode::Allow, Some(1))?;
                    drain(sequence, player, Some(hit.before_velocity), &mut out)?;
                }
            }
        }
        Ok(out)
    }
}
