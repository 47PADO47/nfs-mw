//! Fixed-clock expanding explosions and target filters.

use super::PropWorld;
use blackbox_attrib::vlt_hash;
use blackbox_vehicle::rigid_body::RigidBody;
use glam::Vec3;
use nfsmw_data::world::explosion_fx::is_petrol_station_sequence;
use nfsmw_data::world::sequences::Explosion;
use std::{
    collections::BTreeSet,
    rc::{Rc, Weak},
};

pub(super) struct Wave {
    source: u32,
    position: Vec3,
    spec: Explosion,
    radius: f32,
    complete: bool,
    bodies: BTreeSet<u32>,
    vehicles: Vec<Weak<()>>,
}

impl PropWorld {
    /// Native camera explosions use maximum range independently of physical target flags.
    pub fn camera_explosions(&self) -> Vec<crate::viewer::camera::original::Explosion> {
        self.scenery
            .waves
            .iter()
            .filter(|wave| !wave.complete)
            .map(|wave| crate::viewer::camera::original::Explosion {
                position: crate::scenes::world::space::to_render(wave.position.to_array()),
                expansion_speed: wave.spec.speed,
                maximum_radius: wave.spec.radius,
            })
            .collect()
    }

    pub(super) fn create_explosion(&mut self, id: u32, spec: Explosion) {
        if spec.speed <= 0.0 || spec.radius <= 0.0 || spec.start_radius >= spec.radius {
            return;
        }
        let Some(position) = self.marker(id, spec.position) else {
            log::debug!("scenery {id}: unresolved explosion marker {:08X}", spec.position);
            return;
        };
        let petrol = self.scenery.models.get(&id).is_some_and(|m| is_petrol_station_sequence(m.key));
        self.scenery.waves.push(Wave {
            source: id,
            position,
            spec,
            radius: spec.start_radius.max(0.0),
            complete: false,
            bodies: BTreeSet::new(),
            vehicles: Vec::new(),
        });
        if petrol {
            match self.scenery.models.get(&id).is_some_and(|model| model.authored_visual) {
                true => self.scenery.authored_blasts.push(position),
                false => self.scenery.blasts.push(position),
            }
        }
    }

    /// The points where petrol-station explosions started since the last call, for the world's blast effect.
    pub fn take_blasts(&mut self) -> Vec<Vec3> {
        std::mem::take(&mut self.scenery.blasts)
    }

    pub(in super::super) fn advance_explosions(&mut self, dt: f32) {
        self.scenery.waves.retain(|w| !w.complete);
        let mut ids = std::mem::take(&mut self.scenery.sweep_ids);
        let mut stimuli = std::mem::take(&mut self.scenery.sweep_stimuli);
        ids.clear();
        stimuli.clear();
        let mut sorted = false;
        for wave in &mut self.scenery.waves {
            wave.radius = (wave.radius + wave.spec.speed * dt).min(wave.spec.radius);
            wave.complete = wave.radius >= wave.spec.radius;
            if wave.spec.exclude_smackables {
                continue;
            }
            // Bodies do not come or go during the sweep, so the sorted id list is built once, when first needed.
            if !sorted {
                ids.extend(self.colliders.keys().copied());
                ids.sort_unstable();
                sorted = true;
            }
            for &id in &ids {
                if (id == wave.source && !wave.spec.affect_this) || wave.bodies.contains(&id) {
                    continue;
                }
                let collider = self.colliders.get_mut(&id).unwrap();
                if !collider.collidable {
                    continue;
                }
                let (Some(body), Some(spec)) = (&mut collider.body, &collider.shape.dynamics) else {
                    continue;
                };
                if spec.explosion_effect <= 0.0 || !wave.intersects(&body.rigid) {
                    continue;
                }
                wave.bodies.insert(id);
                if !body.attached && !spec.immobile {
                    wave.apply(&mut body.rigid, dt);
                    body.moved = true;
                }
                body.lifetime.activate(spec);
                super::super::lifetime::track_lifetime(&mut self.live_lifetimes, id);
                stimuli.push((id, vlt_hash("EXPLOSION")));
                if wave.spec.damage {
                    stimuli.push((id, vlt_hash("EXPLOSION_DAMAGE")));
                }
            }
        }
        for (id, stimulus) in stimuli.drain(..) {
            self.enqueue(id, stimulus);
        }
        self.scenery.sweep_ids = ids;
        self.scenery.sweep_stimuli = stimuli;
    }

    pub fn vehicle_explosions(&mut self, body: &mut RigidBody, owner: &Rc<()>, player: bool, dt: f32) {
        let owner = Rc::downgrade(owner);
        for wave in &mut self.scenery.waves {
            if (!player && wave.spec.exclude_vehicles)
                || (player && !wave.spec.include_players)
                || wave.vehicles.iter().any(|w| w.ptr_eq(&owner))
                || !wave.intersects(body)
            {
                continue;
            }
            wave.vehicles.push(owner.clone());
            wave.apply(body, dt);
        }
    }
}

#[cfg(test)]
mod tests;

impl Wave {
    fn intersects(&self, body: &RigidBody) -> bool {
        body.position.distance(self.position) <= self.radius + body.radius()
    }

    fn apply(&self, body: &mut RigidBody, dt: f32) {
        let normal = (body.position - self.position).normalize_or_zero();
        let point = body.position;
        let outward = body.point_velocity(point).dot(normal);
        let speed = (self.spec.speed - outward).max(0.0);
        if speed > 0.0 && dt > 0.0 {
            body.apply_force_at(normal * (speed * body.mass() / dt), point);
        }
    }
}
