//! Two cars hitting each other.
//! Spec: `docs/specs/vehicle-rigid-body.md` (Ã‚Â§6).

use crate::scenes::world::events::DamageHit;
use crate::scenes::world::object_geometry::{BodyGeometryFrame, object_shapes};
use blackbox_collision::bounds_flags;
use blackbox_vehicle::rigid_body::{BodyContact, BodyHit, Separation, resolve_bodies};
use glam::Vec3;

use super::CarSim;
use super::pair_contact::PairContact;

/// What a hit did.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarHit {
    /// Normal impulse, N s.
    pub impulse: f32,
    /// Where the cars touched (physics space).
    pub point: Vec3,
    /// How the cars met, for the police radio (`pair_contact.rs`).
    pub pair: PairContact,
    /// Original callback inputs, measured separately for each body before/after resolution.
    pub damage: [DamageHit; 2],
}

impl CarSim {
    /// The collision box in physics space.
    #[cfg(test)]
    pub fn collision_box(&self) -> blackbox_vehicle::rigid_body::Obb {
        let v = &self.vehicle;
        blackbox_vehicle::rigid_body::Obb { centre: v.position(), axes: v.rotation(), half: v.spec().dimension }
    }

    /// Encloses padded child shapes, which can extend past the original root box.
    pub fn collision_radius(&self) -> f32 {
        let Some(geometry) = &self.geometry else { return self.vehicle.body().radius() };
        geometry
            .body
            .primitives
            .iter()
            .filter(|p| p.collides(bounds_flags::PRIM_VS_OBJECTS))
            .map(|p| p.centre.length() + p.half.length())
            .fold(0.0_f32, f32::max)
    }

    pub fn collision_sweep_radius(&self) -> f32 {
        self.collision_radius() + self.vehicle.position().distance(self.previous_contact_pose.0)
    }

    fn geometry_frame(&self) -> Option<BodyGeometryFrame<'_>> {
        Some(BodyGeometryFrame { geometry: &self.geometry.as_ref()?.body, previous: self.previous_contact_pose })
    }

    /// Tests this car against `other` and, when their boxes overlap, pushes them apart and changes their
    /// velocities. Returns the hit, if the cars were approaching.
    pub fn collide_with(&mut self, other: &mut CarSim) -> Option<CarHit> {
        let a = object_shapes(self.body(), self.geometry_frame());
        let b = object_shapes(other.body(), other.geometry_frame());
        let contact = a
            .iter()
            .flat_map(|a| b.iter().filter_map(move |b| a.contact(b).map(|c| (c, a.surface, b.surface))))
            .max_by(|a, b| a.0.overlap.total_cmp(&b.0.overlap))?;
        let (contact, surface_a, surface_b) = contact;
        let material_a = self.vehicle.body().spec().objects;
        let material_b = other.vehicle.body().spec().objects;
        let grounded = self.vehicle.wheels_on_ground() > 2 && other.vehicle.wheels_on_ground() > 2;
        let hit_a = self.object_hit(
            contact.point,
            contact.normal,
            grounded,
            other.collision_role.player || other.collision_racer,
        );
        let hit_b = other.object_hit(
            contact.point,
            -contact.normal,
            grounded,
            self.collision_role.player || self.collision_racer,
        );
        let relative = self.body().point_velocity(contact.point) - other.body().point_velocity(contact.point);
        let closing = relative.length();
        let mut pair = PairContact::capture(self.body(), other.body(), contact.point, contact.normal, closing);
        let sound = crate::audio::CarContact {
            sound_profile: 0,
            audio_zone: Some(nfsmw_data::sound::CollisionZone::from_local_normal(
                self.body().rotation().transpose() * contact.normal,
            )),
            detached: false,
            source: crate::audio::ContactSource::Object,
            identity: other.sound_identity,
            surface: surface_b,
            point: contact.point,
            normal: contact.normal,
            velocity: relative,
            delta_v: 0.0,
            front: contact.normal.dot(self.body().rotation().z_axis).abs()
                > contact.normal.dot(self.body().rotation().x_axis).abs(),
            bottom: false,
        };
        let other_sound = crate::audio::CarContact {
            audio_zone: Some(nfsmw_data::sound::CollisionZone::from_local_normal(
                other.body().rotation().transpose() * -contact.normal,
            )),
            identity: self.sound_identity,
            surface: surface_a,
            normal: -sound.normal,
            velocity: -relative,
            front: contact.normal.dot(other.body().rotation().z_axis).abs()
                > contact.normal.dot(other.body().rotation().x_axis).abs(),
            ..sound
        };
        let damage_a = DamageHit::capture(self.body(), contact.point, contact.normal, closing, other.collision_role);
        let damage_b = DamageHit::capture(other.body(), contact.point, -contact.normal, closing, self.collision_role);
        let (a, b) = (self.vehicle.body_mut(), other.vehicle.body_mut());
        let body = BodyContact::from_geometry(contact, material_a.friction_with(&material_b));
        let response = resolve_bodies(a, b, &body, &hit_a, &hit_b, Separation::Mass)?;
        pair.delta_v = response.impulse_delta_v;
        if self.collision_role.player {
            self.pending_sound_contacts
                .push(crate::audio::CarContact { delta_v: response.impulse_delta_v[0], ..sound });
        }
        if other.collision_role.player {
            other
                .pending_sound_contacts
                .push(crate::audio::CarContact { delta_v: response.impulse_delta_v[1], ..other_sound });
        }
        let mut damage_a = damage_a.resolved(a, response.notification_delta_v[0], true);
        let mut damage_b = damage_b.resolved(b, response.notification_delta_v[1], true);
        // Callback position remains the original collider point, while arms use separated bodies.
        let closing = response.reaction.closing_speed.abs();
        damage_a.arm = a.rotation().transpose() * (contact.point - a.position);
        damage_b.arm = b.rotation().transpose() * (contact.point - b.position);
        damage_a.closing_speed = closing;
        damage_b.closing_speed = closing;
        damage_a.shock_force = response.reaction.impulse;
        damage_b.shock_force = response.reaction.impulse;
        if let Some(events) = self.events.as_mut() {
            events.hit(damage_a);
        }
        if let Some(events) = other.events.as_mut() {
            events.hit(damage_b);
        }
        self.queue_effect(crate::scenes::world::impact_effects::RawContact::new(
            crate::scenes::world::impact_effects::Kind::Object,
            surface_b,
            contact.point,
            contact.normal,
            relative,
            response.notification_delta_v[0],
            self.vehicle.body().linear_velocity,
        ));
        other.queue_effect(crate::scenes::world::impact_effects::RawContact::new(
            crate::scenes::world::impact_effects::Kind::Object,
            surface_a,
            contact.point,
            -contact.normal,
            -relative,
            response.notification_delta_v[1],
            other.vehicle.body().linear_velocity,
        ));
        self.vehicle.notify_collision(response.impulse_delta_v[0]);
        other.vehicle.notify_collision(response.impulse_delta_v[1]);
        Some(CarHit { impulse: response.reaction.impulse, point: contact.point, pair, damage: [damage_a, damage_b] })
    }

    /// Temporary parameters selected for this contact, including the grounded player's roll centre.
    pub(super) fn object_hit(
        &self,
        point: Vec3,
        normal: Vec3,
        grounded: bool,
        partner_human_or_racer: bool,
    ) -> BodyHit {
        let body = self.vehicle.body();
        let material = body.spec().objects;
        let mut hit = BodyHit {
            immobile: material.immobile,
            restitution: material.restitution(body.rotation().transpose() * normal),
            inertia_scale: material.inertia_scale,
            mass_scale: material.mass_scale,
            ..Default::default()
        };
        let mut cog = body.cog();
        if self.collision_role.player {
            self.vehicle.modify_object_hit(&mut hit);
            if grounded {
                cog.y = point.y - body.position.y - 0.125;
                hit.centre = Some(body.position + body.rotation() * cog);
            }
            return hit;
        }
        if !partner_human_or_racer {
            return hit;
        }
        let Some(profile) = self.reactions.as_ref().and_then(|r| r.profile(self.collision_goal)) else { return hit };
        let forward = body.rotation().z_axis;
        let front = (point - body.position).dot(forward) > 0.0;
        let side = normal.dot(forward).abs() < 0.707;
        let reaction = profile.choose(front, side);
        hit.restitution = (hit.restitution + reaction.elasticity).clamp(0.0, 1.0);
        if reaction.mass_scale > 0.0 {
            hit.mass_scale *= reaction.mass_scale;
            hit.inertia_scale *= reaction.mass_scale;
        }
        cog.y += reaction.roll_height;
        cog.z += reaction.weight_bias;
        hit.centre = Some(body.position + body.rotation() * cog);
        hit
    }
}
