//! Native prop pairs and their per-owner callbacks.

use super::*;

impl PropWorld {
    pub(super) fn collide_props(&mut self, active: &[u32]) {
        // Sorted and deduplicated, so the pair order is the same as the BTreeSet it replaces.
        let mut pairs = std::mem::take(&mut self.pair_scratch);
        let mut others = std::mem::take(&mut self.candidate_scratch);
        pairs.clear();
        for &id in active {
            if !self.colliders[&id].can_collide_with_objects() {
                continue;
            }
            let bounds = self.colliders[&id].physics_bounds();
            self.candidates(&bounds, &mut others);
            for &other in &others {
                if id == other {
                    continue;
                }
                pairs.push((id.min(other), id.max(other)));
            }
        }
        self.candidate_scratch = others;
        pairs.sort_unstable();
        pairs.dedup();
        for &(a, b) in &pairs {
            let (Some(pa), Some(pb)) = (&self.colliders[&a].body, &self.colliders[&b].body) else { continue };
            let counts =
                [contact::shape_count(Some(pa.geometry_frame())), contact::shape_count(Some(pb.geometry_frame()))];
            for i in 0..counts[0] {
                for j in 0..counts[1] {
                    let [Some(ca), Some(cb)] = self.colliders.get_disjoint_mut([&a, &b]) else { continue };
                    let (Some(pa), Some(pb)) = (&mut ca.body, &mut cb.body) else { continue };
                    let (Some(sa), Some(sb)) = (&ca.shape.dynamics, &cb.shape.dynamics) else { continue };
                    let first = contact::shape(&pa.rigid, Some(pa.geometry_frame()), i).unwrap();
                    let second = contact::shape(&pb.rigid, Some(pb.geometry_frame()), j).unwrap();
                    let Some(c) = first.contact(&second) else { continue };
                    let attached = [pa.attached, pb.attached];
                    let relative = pa.rigid.point_velocity(c.point) - pb.rigid.point_velocity(c.point);
                    let closing = relative.length();
                    let ha = contact::parameters(&pa.rigid, c.normal, sa, pa.attached, sb.immobile || pb.attached);
                    let hb = contact::parameters(&pb.rigid, -c.normal, sb, pb.attached, sa.immobile || pa.attached);
                    let response = contact::resolve(&mut pa.rigid, &mut pb.rigid, c, ha, hb, Separation::NormalSpeed);
                    let Some(response) = response else { continue };
                    pa.lifetime.activate(sa);
                    pb.lifetime.activate(sb);
                    lifetime::track_lifetime(&mut self.live_lifetimes, a);
                    lifetime::track_lifetime(&mut self.live_lifetimes, b);
                    pa.object_contact = true;
                    pb.object_contact = true;
                    let [ia, ib] = response.immobile;
                    pa.attached &= ia;
                    pb.attached &= ib;
                    for (source, owner, life, rigid, surface, normal, velocity, delta, detached) in [
                        (
                            sa.sound_profile,
                            ca.effect_owner,
                            &ca.effect_lifetime,
                            &pa.rigid,
                            second.surface,
                            c.normal,
                            relative,
                            response.notification_delta_v[0],
                            attached[0] && !pa.attached,
                        ),
                        (
                            sb.sound_profile,
                            cb.effect_owner,
                            &cb.effect_lifetime,
                            &pb.rigid,
                            first.surface,
                            -c.normal,
                            -relative,
                            response.notification_delta_v[1],
                            attached[1] && !pb.attached,
                        ),
                    ] {
                        let mut raw = super::super::impact_effects::RawContact::new(
                            super::super::impact_effects::Kind::Object,
                            surface,
                            c.point,
                            normal,
                            velocity,
                            delta,
                            rigid.linear_velocity,
                        );
                        raw.detached = detached;
                        raw.detached_normal = c.normal;
                        effects::push(&mut self.effect_contacts, source, owner, life, raw);
                    }
                    sounds::Pair {
                        owners: [(a, sa.sound_profile), (b, sb.sound_profile)],
                        surfaces: [second.surface, first.surface],
                        zones: [
                            nfsmw_data::sound::CollisionZone::from_local_normal(
                                pa.rigid.rotation().transpose() * c.normal,
                            ),
                            nfsmw_data::sound::CollisionZone::from_local_normal(
                                pb.rigid.rotation().transpose() * -c.normal,
                            ),
                        ],
                        contact: c,
                        velocity: relative,
                        detached: [attached[0] && !pa.attached, attached[1] && !pb.attached],
                    }
                    .emit(&mut self.sound_contacts, response.notification_delta_v);
                    pa.moved |= !ia;
                    pb.moved |= !ib;
                    self.contact_event(a, attached[0], closing);
                    self.contact_event(b, attached[1], closing);
                }
            }
            self.regrid(a);
            self.regrid(b);
        }
        self.pair_scratch = pairs;
    }
}
