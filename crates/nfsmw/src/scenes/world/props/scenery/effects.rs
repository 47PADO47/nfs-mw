//! Host effects; area messages retain authored target systems and repetition counts.

use super::{PropWorld, Stimulus, StopArea, queue};
use glam::Vec3;
use nfsmw_data::{car::physics::PhysicsData, world::sequences::WorldEvent};

impl PropWorld {
    pub(super) fn dispatch_scenery(&mut self, id: u32, event: WorldEvent, data: &PhysicsData) {
        let Some(position) = self.colliders.get(&id).and_then(|c| c.body.as_ref()).map(|b| b.rigid.position) else {
            return;
        };
        match event {
            WorldEvent::ObjectEffect(effect) => self.object_effect(id, effect, data),
            WorldEvent::StopObjectEffect(instance) => self.stop_object_effect(id, Some(instance)),
            WorldEvent::StopObjectEffects => self.stop_object_effect(id, None),
            WorldEvent::AreaStimulus { system, stimulus, queue_mode, radius } => {
                if radius <= 0.0 {
                    return;
                }
                let Ok(mode) = queue(queue_mode) else {
                    log::warn!("unsupported area queue mode {queue_mode:08X}");
                    return;
                };
                let target = (system != 0).then_some(system);
                for &other in self.scenery.models.keys() {
                    let Some(body) = self.colliders.get(&other).and_then(|c| c.body.as_ref()) else {
                        continue;
                    };
                    if body.rigid.position.distance_squared(position) > radius * radius {
                        continue;
                    }
                    self.scenery.pending.push_back(Stimulus { id: other, system: target, stimulus, mode });
                }
            }
            WorldEvent::Fragment(fragment) => {
                self.spawn_scenery_fragment(id, fragment, data);
            }
            WorldEvent::Explosion(explosion) => {
                self.create_explosion(id, explosion);
            }
            WorldEvent::HideObject => {
                if let Some(model) = self.scenery.models.get_mut(&id) {
                    model.hidden = true;
                }
            }
            WorldEvent::HidePart(part) => {
                if let Some(model) = self.scenery.models.get_mut(&id) {
                    model.hidden_parts.insert(part);
                }
            }
            WorldEvent::KillObject => {
                if self.fragments.contains_key(&id) {
                    self.remove_fragment(id);
                    return;
                }
                let Some(collider) = self.colliders.get_mut(&id) else {
                    return;
                };
                if let (Some(body), Some(spec)) = (&mut collider.body, &collider.shape.dynamics) {
                    body.restore(spec);
                }
                self.reset_model(id);
                self.regrid(id);
            }
            WorldEvent::WakeObject => {
                if let Some(body) = self.colliders.get_mut(&id).and_then(|c| c.body.as_mut()) {
                    body.rigid.wake();
                }
            }
            WorldEvent::StopCops { radius, duration } => {
                if radius > 0.0 && duration > 0.0 {
                    self.scenery.stops.push(StopArea { position, radius, duration });
                }
            }
            WorldEvent::PursuitBreaker { begin } => {
                let event = match begin {
                    true => crate::audio::CommonEvent::SpeedbreakerBegin,
                    false => crate::audio::CommonEvent::SpeedbreakerEnd,
                };
                self.scenery.common_sounds.push(event);
                log::debug!("pursuit breaker model {id}, begin {begin}");
            }
            other => {
                log::debug!("scenery {id} effect pending: {other:?}");
            }
        }
    }

    pub(in super::super) fn marker(&self, id: u32, hash: u32) -> Option<Vec3> {
        let model = self.scenery.models.get(&id)?;
        let body = self.colliders.get(&id)?.body.as_ref()?;
        if hash == 0 {
            return Some(body.rigid.position);
        }
        let root = &model.bounds.nodes[model.bounds_root];
        let node = model.bounds.children(root).iter().find(|n| n.name_hash == hash)?;
        Some(body.rigid.position + body.rigid.orientation * Vec3::from_array(node.position))
    }
}
