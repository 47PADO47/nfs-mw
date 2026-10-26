//! Vehicle damage callbacks drive the installed sequencer on the shared world clock.
//! Spec: `docs/specs/world-sequencers.md`.

mod damage;
mod dispatch;
mod lights;
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
use nfsmw_data::car::damage::{ZONE_HAS_GLASS, revealed_overlays};
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
    /// Original per-zone sequencer damage levels, independent of the cosmetic aspect model.
    source_levels: [u8; 10],
    destroyed: bool,
    light_damage: u32,
    failed: bool,
    pub hidden_slots: BTreeSet<usize>,
    /// How far each hinged part of a mod car that moves with the speed is deployed (one per hinged part).
    pub deployment: Vec<nfsmw_data::car::mods::hinged::Deployment>,
    /// The state of the car's wing machine and what it plays now (`[wing]`).
    pub wing: nfsmw_data::car::mods::hinged::WingState,
    pub wing_blend: Option<nfsmw_data::car::mods::hinged::Blend>,
    pub spawned: BTreeSet<u32>,
    /// Child models live until their original parent is destroyed, independently of scenery tiles.
    pub owner: Rc<()>,
    /// Visible damage accumulated from impacts; it lasts for the car's life in this session.
    pub damage: Damage,
}

impl VehicleEvents {
    /// Health destruction breaks lamps without replacing the ordinary hit's zone tiers.
    pub fn note_health_destroyed(&mut self) {
        self.destroyed = true;
    }

    /// Whether the car's health ran out: it is wrecked (the HUD's vehicle condition reads it).
    pub fn is_destroyed(&self) -> bool {
        self.destroyed
    }

    pub fn destroy(&mut self) {
        self.note_health_destroyed();
        self.source_levels.fill(6);
        for (zone, &glass) in ZONE_HAS_GLASS.iter().enumerate() {
            self.damage.impact(zone, 6, glass);
        }
    }
    pub fn spec(&self) -> &CarEventSpec {
        &self.spec
    }

    pub fn source_levels(&self) -> &[u8; 10] {
        &self.source_levels
    }

    pub fn new(spec: Arc<CarEventSpec>) -> Result<Self> {
        let player = spec.sequence.as_ref().map(|s| Player::new(&s.package, s.engine, TAGS)).transpose()?;
        Ok(Self {
            spec,
            player,
            hits: Vec::new(),
            source_levels: [0; 10],
            destroyed: false,
            light_damage: 0,
            failed: false,
            hidden_slots: BTreeSet::new(),
            deployment: Vec::new(),
            wing: Default::default(),
            wing_blend: None,
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
        let mut slots = revealed_overlays(&self.damage);
        // The panel level of each damaged zone, for the hinged parts of a mod car (`nfsmw_data::car::mods::hinged`).
        for zone in 0..self.damage.zone_count() {
            let level = self.damage.zone(zone).panel;
            if level > 0 {
                slots.insert(nfsmw_data::car::mods::hinged::level_mark(zone, level));
            }
        }
        for (part, deployed) in self.deployment.iter().enumerate() {
            slots.extend(nfsmw_data::car::mods::hinged::deploy_mark(part, deployed.amount));
        }
        slots.extend(self.wing_blend.and_then(|blend| blend.mark()));
        slots
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
            Ok(mut events) => {
                events.retain(|effect| {
                    let WorldEvent::DamageLights(name) = effect.event else { return true };
                    if !self.set_light_damage(name, true) {
                        log::warn!("unknown named vehicle light {name:08X}");
                    }
                    false
                });
                events
            }
            Err(error) => {
                self.failed = true;
                self.hits.clear();
                log::warn!("vehicle event engine stopped: {error:#}");
                Vec::new()
            }
        }
    }

    /// Applies `hit` to the visible damage: its zone and the impact level it reached. `None` when it does nothing
    /// (no strength, or a force that is not finite).
    fn wear(&mut self, hit: &DamageHit) -> Option<(usize, usize)> {
        if self.spec.strength <= 0.0 || !hit.force.is_finite() {
            return None;
        }
        let zone = hit.zone();
        let level = (hit.force / self.spec.strength * self.spec.visual_scale[zone]).clamp(0.0, 6.0) as usize;
        log::debug!("vehicle damage {}: force {:.1}, impact level {level}", DAMAGE_ZONES[zone], hit.force);
        if self.damage.impact(zone, level as u8, ZONE_HAS_GLASS[zone]) {
            log::info!("vehicle damage {} now {:?}", DAMAGE_ZONES[zone], self.damage.zone(zone));
        }
        Some((zone, level))
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
            let Some((zone, level)) = self.wear(&hit) else { continue };
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
            stimuli.extend((1..=level).map(|n| vlt_hash(&format!("IMPACT_{n}"))));
            let target_level = level.min(2) as u8;
            let previous = self.source_levels[zone];
            if previous <= 5 && target_level > previous {
                stimuli.extend((previous + 1..=target_level).map(|n| vlt_hash(&format!("DAMAGE_{n}"))));
                self.source_levels[zone] = target_level;
            }
            if let (Some(sequence), Some(player)) = (&self.spec.sequence, self.player.as_mut()) {
                for stimulus in stimuli {
                    let handled = player.stimulate(&sequence.package, target, stimulus, QueueMode::Allow, Some(1))?;
                    log::debug!("vehicle sequencer {}: stimulus {stimulus:08X}, handled={handled}", DAMAGE_ZONES[zone]);
                    drain(sequence, player, Some(hit.before_velocity), &mut out)?;
                }
            }
        }
        Ok(out)
    }
}
