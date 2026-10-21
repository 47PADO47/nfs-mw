//! Native object contacts precede ground/drag/sleep; the world can place vehicle contacts between phases.
use super::*;

impl PropWorld {
    #[cfg(test)]
    pub fn step(&mut self, ground: &dyn Ground, dt: f32) {
        self.begin_native(dt);
        self.finish_native(ground, dt);
    }

    /// One world tick; fallen props keep their render/collision identity.
    pub(in crate::scenes::world) fn begin_native(&mut self, dt: f32) {
        let _span = bevy_log::info_span!("world::props_step").entered();
        self.streaming.advance(dt);
        self.knocked.retain(|_, left| {
            *left -= dt;
            *left > 0.0
        });
        self.remove_orphaned_fragments();
        self.advance_explosions(dt);
        let mut active = std::mem::take(&mut self.active_scratch);
        active.clear();
        for (&id, c) in &mut self.colliders {
            let Some(b) = c.body.as_mut() else { continue };
            b.previous = (b.rigid.position, b.rigid.orientation);
            b.object_contact = false;
            if b.rigid.state != BodyState::Awake || b.attached || b.lifetime.sinking {
                continue;
            }
            b.rigid.begin_frame(dt);
            active.push(id);
        }
        active.sort_unstable();
        for &id in &active {
            self.regrid(id);
        }
        self.collide_props(&active);
        self.active_scratch = active;
    }

    #[cfg(test)]
    pub(in crate::scenes::world) fn finish_native(&mut self, ground: &dyn Ground, dt: f32) {
        self.finish_contacts(ground, None, dt);
    }

    pub(in crate::scenes::world) fn finish_native_world(
        &mut self,
        ground: &super::super::drive::WorldGround<'_>,
        dt: f32,
    ) {
        self.finish_contacts(ground, Some(ground), dt);
    }

    fn finish_contacts(&mut self, ground: &dyn Ground, world: Option<&super::super::drive::WorldGround<'_>>, dt: f32) {
        let mut ids = std::mem::take(&mut self.active_scratch);
        ids.clear();
        ids.extend(self.colliders.iter().filter_map(|(&id, c)| {
            c.body
                .as_ref()
                .filter(|b| b.rigid.state == BodyState::Awake && !b.attached && !b.lifetime.sinking)
                .map(|_| id)
        }));
        ids.sort_unstable();
        for &id in &ids {
            let collider = self.colliders.get_mut(&id).unwrap();
            let prop = collider.body.as_mut().unwrap();
            prop.rigid.apply_drag();
            if let Some(world) = world
                && let Some(spec) = &collider.shape.dynamics
            {
                let start = self.sound_contacts.len();
                let mut visuals = Vec::new();
                world_contacts::step(
                    prop,
                    spec,
                    world.collision,
                    world.surfaces,
                    &mut self.barrier_scratch,
                    &mut self.sound_contacts,
                    &mut visuals,
                );
                for raw in visuals {
                    effects::push(
                        &mut self.effect_contacts,
                        spec.sound_profile,
                        collider.effect_owner,
                        &collider.effect_lifetime,
                        raw,
                    );
                }
                for contact in &mut self.sound_contacts[start..] {
                    contact.identity = u64::from(id);
                }
            }
            if let Some(contact) = ground::step(prop, ground, dt, &mut self.contact_scratch)
                && let Some(spec) = &collider.shape.dynamics
            {
                let raw = super::super::impact_effects::RawContact::ground(&contact, prop.rigid.linear_velocity);
                effects::push(
                    &mut self.effect_contacts,
                    spec.sound_profile,
                    collider.effect_owner,
                    &collider.effect_lifetime,
                    raw,
                );
                if spec.sound_profile == 0 {
                    continue;
                }
                self.sound_contacts.push(CarContact {
                    sound_profile: spec.sound_profile,
                    vehicle: None,
                    audio_zone: Some(nfsmw_data::sound::CollisionZone::from_local_normal(
                        prop.rigid.rotation().transpose() * contact.normal,
                    )),
                    detached: false,
                    source: ContactSource::Ground,
                    identity: u64::from(id),
                    surface: contact.surface,
                    point: contact.point,
                    normal: contact.normal,
                    velocity: contact.velocity,
                    delta_v: contact.impulse_delta_v,
                    front: false,
                    bottom: false,
                });
            }
        }
        for &id in &ids {
            self.regrid(id);
        }
        self.active_scratch = ids;
    }
}
