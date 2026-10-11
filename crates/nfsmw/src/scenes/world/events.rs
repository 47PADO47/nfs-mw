//! Vehicle damage callbacks drive the installed sequencer on the shared world clock.
//! Spec: `docs/specs/world-sequencers.md`.

mod damage;
#[cfg(test)]
mod tests;
pub use damage::{DamageHit, OtherBody};

use std::{collections::BTreeSet, rc::Rc, sync::Arc};

use anyhow::{Context, Result, bail};
use blackbox_attrib::vlt_hash;
use blackbox_sequence::{Player, QueueMode};
use glam::Vec3;
use nfsmw_data::car::physics::{CarEventSpec, DAMAGE_ZONES};
use nfsmw_data::world::sequences::{TAGS, WorldEvent};

pub struct Effect {
    pub event: WorldEvent,
    /// Immediate damage callbacks supply pre-impact velocity. Timed/forwarded events use the live model.
    pub velocity: Option<Vec3>,
}

pub struct VehicleEvents {
    spec: Arc<CarEventSpec>,
    player: Player,
    hits: Vec<DamageHit>,
    failed: bool,
    pub hidden_slots: BTreeSet<usize>,
    pub spawned: BTreeSet<u32>,
    /// Child models live until their original parent is destroyed, independently of scenery tiles.
    pub owner: Rc<()>,
}

impl VehicleEvents {
    pub fn new(spec: Arc<CarEventSpec>) -> Result<Self> {
        let player = Player::new(&spec.sequence.package, spec.sequence.engine, TAGS)?;
        Ok(Self {
            spec,
            player,
            hits: Vec::new(),
            failed: false,
            hidden_slots: BTreeSet::new(),
            spawned: BTreeSet::new(),
            owner: Rc::new(()),
        })
    }

    pub fn hit(&mut self, hit: DamageHit) {
        if !self.failed && !hit.other.no_car_effect && hit.closing_speed >= 1.0 {
            self.hits.push(hit);
        }
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
        self.player.advance(&self.spec.sequence.package, 1.0 / 60.0)?;
        self.drain(None, &mut out)?;
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
            stimuli.extend((1..=level).map(|n| vlt_hash(&format!("IMPACT_{n}"))));
            for stimulus in stimuli {
                self.player.stimulate(&self.spec.sequence.package, target, stimulus, QueueMode::Allow, Some(1))?;
                self.drain(Some(hit.before_velocity), &mut out)?;
            }
        }
        Ok(out)
    }

    fn drain(&mut self, velocity: Option<Vec3>, out: &mut Vec<Effect>) -> Result<()> {
        let package = &self.spec.sequence.package;
        let mut remaining = 4096;
        while let Some(emission) = self.player.pop() {
            let mut pending: Vec<_> = package.event_lists[&emission.list].iter().rev().collect();
            while let Some(event) = pending.pop() {
                if remaining == 0 {
                    bail!("event dispatch cycle exceeded 4096 events");
                }
                remaining -= 1;
                let event = WorldEvent::decode(event)?;
                if let WorldEvent::FireList { list, delayed } = event {
                    if delayed {
                        bail!("delayed EFireEventList needs deferred context");
                    }
                    let children = package.event_lists.get(&list).context("EFireEventList has missing list")?;
                    pending.extend(children.iter().rev());
                    continue;
                }
                if let WorldEvent::Stimulus { system, stimulus, queue_mode } = event {
                    let target = match system {
                        0 => None,
                        h if h == vlt_hash("this") => Some(emission.system),
                        h => Some(h),
                    };
                    self.player.stimulate(package, target, stimulus, queue(queue_mode)?, None)?;
                    continue;
                }
                out.push(Effect { event, velocity: emission.context.and(velocity) });
            }
        }
        Ok(())
    }
}

fn queue(hash: u32) -> Result<QueueMode> {
    match hash {
        0 | 0x102A_BF91 => Ok(QueueMode::Allow),
        0xCC7F_660A => Ok(QueueMode::Disable),
        0x669E_C6BC => Ok(QueueMode::Shallow),
        0x3C56_741D => Ok(QueueMode::Flush),
        0x936F_462E => Ok(QueueMode::Abort),
        _ => bail!("unknown queue mode {hash:08X}"),
    }
}
