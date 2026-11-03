use super::terrain::TerrainQuery;
use super::*;

/// What the start of a step leaves for the rest of it: the library's input and the terrain query.
#[derive(Debug, Clone, Copy)]
pub(in crate::scenes::world) struct StepStart {
    pub(super) input: InputState,
    /// The rider's weight the player asks, -1 (back) to 1 (forward): only a bike's ride reads it.
    pub(super) weight: f32,
    pub(super) delta: f32,
    /// `None` without a collision world: the car then only meets the road.
    pub(super) terrain: Option<TerrainQuery>,
}

impl StepStart {
    /// Whether the step has a collision world, so the body's terrain caches prepare the wheels' ground.
    pub(super) fn prepared(&self) -> bool {
        self.terrain.is_some()
    }
}

impl CarSim {
    /// One physics step. `world` is the collision the body's walls are tested against (the ground is
    /// `ground`); without it the car only meets the road.
    ///
    /// Composes preparation, body integration, body contacts, driving response and publication.
    /// Shared callers place car pairs between body integration and contacts.
    #[cfg(test)]
    pub fn step(
        &mut self,
        input: &DriveInput,
        ground: &dyn Ground,
        world: Option<(&CollisionWorld, &mut PropWorld)>,
    ) -> Impact {
        self.step_with_delta(FIXED_STEP, input, ground, world)
    }

    /// One step with an explicit simulation delta; invalid deltas leave all state unchanged.
    /// The caller owns scheduling and shared-world contact order; see `docs/specs/player-timestep.md`.
    #[cfg(test)]
    pub fn step_with_delta(
        &mut self,
        dt: f32,
        input: &DriveInput,
        ground: &dyn Ground,
        mut world: Option<(&CollisionWorld, &mut PropWorld)>,
    ) -> Impact {
        if !dt.is_finite() || dt <= 0.0 {
            return Impact::default();
        }
        let collision = world.as_ref().map(|(collision, _)| *collision);
        let start =
            self.begin_step_with_delta(dt, input, collision.is_some(), world.as_mut().map(|(_, props)| &mut **props));
        self.begin_body(&start, collision);
        self.finish_body(&start, ground, world.as_mut().map(|(collision, props)| (*collision, &mut **props)));
        self.integration().run(&start, ground, collision);
        self.end_step(&start, ground, world)
    }

    /// The start of a step, up to the integration: the input, the terrain query and the explosions that push the body.
    /// `props` is `None` without a collision world.
    pub(in crate::scenes::world) fn begin_step_with_delta(
        &mut self,
        dt: f32,
        input: &DriveInput,
        has_world: bool,
        props: Option<&mut PropWorld>,
    ) -> StepStart {
        if !dt.is_finite() || dt <= 0.0 {
            return StepStart { input: input.to_vehicle(), weight: input.weight, delta: dt, terrain: None };
        }
        self.step_delta = dt;
        self.pull_bike();
        self.pending_body_impact = Impact::default();
        self.reset_collision.advance(dt);
        // Taken before explosions: they can wake a sleeping body. Body begin refreshes this captured query.
        let terrain = has_world.then(|| self.terrain_query());
        self.before_velocity = self.vehicle.body().linear_velocity;
        let weight = input.weight;
        let input = input.to_vehicle();
        self.previous_contact_pose = (self.vehicle.position(), self.vehicle.body().orientation);
        if let (Some(props), Some(events)) = (props, &self.events) {
            props.vehicle_explosions(self.vehicle.body_mut(), &events.owner, self.collision_role.player, dt);
        }
        if let Some(events) = &self.events {
            let punctured = events.punctured();
            for wheel in (0..4).filter(|&w| punctured[w]) {
                self.vehicle.tire_damage.puncture(wheel);
            }
        }
        StepStart { input, weight, delta: dt, terrain }
    }

    /// Resolves world contacts and body drag after the shared pair pass, before driving response.
    pub(in crate::scenes::world) fn finish_body(
        &mut self,
        start: &StepStart,
        ground: &dyn Ground,
        world: Option<(&CollisionWorld, &mut PropWorld)>,
    ) {
        if !start.delta.is_finite() || start.delta <= 0.0 {
            return;
        }
        self.pending_body_impact = match world {
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
                    self.previous_contact_pose.0,
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
        match self.ride.as_deref_mut() {
            Some(ride) => ride.finish_body(&mut self.vehicle, start.delta, ground),
            None => self.vehicle.finish_body_step(start.delta, ground),
        }
    }

    /// Publishes body contacts and driving response once both phases have completed.
    pub(in crate::scenes::world) fn end_step(
        &mut self,
        start: &StepStart,
        _ground: &dyn Ground,
        _world: Option<(&CollisionWorld, &mut PropWorld)>,
    ) -> Impact {
        if !start.delta.is_finite() || start.delta <= 0.0 {
            return Impact::default();
        }
        let input = start.input;
        let mut impact = std::mem::take(&mut self.pending_body_impact);

        self.smacked.append(&mut impact.smacked);
        self.prop_struck |=
            impact.contacts.iter().any(|c| matches!(c.kind, ContactKind::PropRigid | ContactKind::PropMovable));

        let body = self.vehicle.body();
        let rotation = body.rotation();
        let bottom_point =
            body.position - rotation.y_axis * body.dimension().y - rotation.z_axis * (body.dimension().z * 0.75);
        impact.sounds.extend(self.vehicle.body_contacts().iter().filter(|c| c.impulse_delta_v > 0.0).map(|c| {
            let bottom = self.wheels_down() > 2 && rotation.y_axis.dot(c.normal) > 0.2;
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
        match self.ride.as_deref_mut() {
            Some(ride) => ride.advance(start.delta),
            None => {
                for (spin, &physics) in self.spin.iter_mut().zip(&PHYSICS_WHEEL) {
                    *spin += self.vehicle.wheel(physics).angular_velocity * start.delta;
                }
            }
        }
        let body = self.vehicle.body();
        self.memo.record(&input, &impact, self.before_velocity, body.linear_velocity, body.mass());
        self.queue_non_player_sounds(&impact);
        self.advance_lamps(input.brake, start.delta);
        impact
    }
}
