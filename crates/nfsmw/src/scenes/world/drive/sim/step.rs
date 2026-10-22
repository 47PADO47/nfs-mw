use super::*;

impl CarSim {
    /// One physics step. `world` is the collision the body's walls are tested against (the ground is
    /// `ground`); without it the car only meets the road.
    pub fn step(
        &mut self,
        input: &DriveInput,
        ground: &dyn Ground,
        mut world: Option<(&CollisionWorld, &mut PropWorld)>,
    ) -> Impact {
        self.reset_collision.advance(FIXED_STEP);
        self.refresh_ground_reference(world.as_ref().map(|(collision, _)| *collision));
        self.before_velocity = self.vehicle.body().linear_velocity;
        let input = input.to_vehicle();
        self.previous_contact_pose = (self.vehicle.position(), self.vehicle.body().orientation);
        if let (Some((_, props)), Some(events)) = (world.as_mut(), &self.events) {
            props.vehicle_explosions(self.vehicle.body_mut(), &events.owner, self.collision_role.player, FIXED_STEP);
        }
        if let Some(events) = &self.events {
            let punctured = events.punctured();
            for wheel in (0..4).filter(|&w| punctured[w]) {
                self.vehicle.tire_damage.puncture(wheel);
            }
        }
        self.step_with_ground(&input, ground, world.is_some());
        let mut impact = match world {
            Some((collision, props)) => {
                let frame = self
                    .geometry
                    .as_ref()
                    .map(|g| BodyGeometryFrame { geometry: &g.body, previous: self.previous_contact_pose });
                // The original prunes object pairs before its end-frame world collision callback.
                let mut objects = Impact::default();
                props.collide_vehicle_admitted(&mut self.vehicle, frame, &mut objects, || self.reset_collision.admit());
                let wall_exclusion = self.terrain_exclusion();
                let mut impact = walls::resolve_into(
                    &mut self.vehicle,
                    &walls::world_cast_with_mask(collision, wall_exclusion),
                    &walls::world_props(props),
                    &self.walls,
                    &|surface| ground.world_friction(surface),
                    self.collision_role.player,
                    &mut self.wall_scratch,
                );
                impact.impulse = impact.impulse.max(objects.impulse);
                impact.contacts.extend(objects.contacts);
                impact.damage.splice(0..0, objects.damage);
                impact.sounds.extend(objects.sounds);
                impact.effects.extend(objects.effects);
                impact
            }
            None => Impact::default(),
        };

        self.smacked.append(&mut impact.smacked);
        self.prop_struck |=
            impact.contacts.iter().any(|c| matches!(c.kind, ContactKind::PropRigid | ContactKind::PropMovable));

        let body = self.vehicle.body();
        let rotation = body.rotation();
        let bottom_point =
            body.position - rotation.y_axis * body.dimension().y - rotation.z_axis * (body.dimension().z * 0.75);
        impact.sounds.extend(self.vehicle.body_contacts().iter().filter(|c| c.impulse_delta_v > 0.0).map(|c| {
            let bottom = self.vehicle.wheels_on_ground() > 2 && rotation.y_axis.dot(c.normal) > 0.2;
            crate::audio::CarContact {
                sound_profile: 0,
                vehicle: None,
                audio_zone: Some(nfsmw_data::sound::CollisionZone::from_local_normal(rotation.transpose() * c.normal)),
                detached: false,
                source: crate::audio::ContactSource::Ground,
                identity: 0,
                surface: c.surface,
                point: if bottom { bottom_point } else { c.point },
                normal: c.normal,
                velocity: c.velocity,
                delta_v: c.impulse_delta_v,
                front: false,
                bottom,
            }
        }));
        self.append_ground_damage(&mut impact.damage);
        self.queue_ground_effects();
        for &raw in &impact.effects {
            self.queue_effect(raw);
        }
        if let Some(events) = self.events.as_mut() {
            for &hit in &impact.damage {
                if self.collision_role.cop && !hit.allows_cop() {
                    continue;
                }
                events.hit(hit);
            }
        }
        for (spin, &physics) in self.spin.iter_mut().zip(&PHYSICS_WHEEL) {
            *spin += self.vehicle.wheel(physics).angular_velocity * FIXED_STEP;
        }
        let body = self.vehicle.body();
        self.memo.record(&input, &impact, self.before_velocity, body.linear_velocity, body.mass());
        self.queue_non_player_sounds(&impact);
        self.advance_lamps(input.brake);
        impact
    }
}
