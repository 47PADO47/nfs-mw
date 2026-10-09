//! Stock collision particle groups; never replaced by the restoration fallback.

use super::super::{drive::VisualContact, space};
use super::{pc_emitter::PcSource, pc_particle::PcParticle};
use blackbox_gfx::{BlendMode, EffectVertex, TextureHandle, TexturedEffect};
use glam::{Quat, Vec3};
use nfsmw_data::vehicle_effects::{CollisionEffects, PcSparkLink};
use std::collections::HashMap;

#[derive(Default)]
pub(super) struct PcSparks {
    particles: Vec<PcParticle>,
    hits: Vec<PcSource>,
    scrape: Vec<PcSource>,
    pub emitted: u64,
}

impl PcSparks {
    pub fn len(&self) -> usize {
        self.particles.len()
    }
    pub fn clear(&mut self) {
        self.particles.clear();
        self.disconnect();
    }
    pub fn disconnect(&mut self) {
        self.hits.clear();
        self.scrape.clear();
    }
    pub fn age(&mut self, dt: f32) {
        self.particles.iter_mut().for_each(|p| p.step(dt));
        self.particles.retain(PcParticle::alive);
    }

    pub fn emit(&mut self, contacts: &[VisualContact], data: &CollisionEffects, owner: Vec3, dt: f32) {
        let mut hit = None;
        let mut scrape = None;
        for c in contacts {
            let Some(surface) = c.surface else { continue };
            if !c.point.is_finite()
                || !c.velocity.is_finite()
                || !c.normal.is_finite()
                || (c.normal.length_squared() - 1.0).abs() > 0.05
                || !c.impulse_delta_v.is_finite()
            {
                continue;
            }
            let inward = c.velocity.dot(c.normal);
            let sliding = (c.velocity - c.normal * inward).length();
            if inward < -1.0
                && c.impulse_delta_v > 0.0
                && let Some(link) = data.pc_hit(surface)
            {
                let i = intensity(link, c.impulse_delta_v);
                if i > hit.map_or(0.0, |(_, _, value)| value) {
                    hit = Some((c, link, i));
                }
            }
            if inward <= 0.1
                && let Some(link) = data.pc_scrape(surface)
                && sliding >= link.min
            {
                let i = intensity(link, sliding);
                if i > scrape.map_or(0.0, |(_, _, value)| value) {
                    scrape = Some((c, link, i));
                }
            }
        }
        if let Some((c, link, i)) = hit {
            // Each impact owns its one-shot clocks, so its small glow can outlast contact.
            if self.hits.len() + link.emitters.iter().flatten().count() <= 128 {
                self.hits.extend(sources(c, link, owner, i, true, self.emitted));
            }
            scrape = None;
        }
        let Some((c, link, i)) = scrape else {
            self.scrape.clear();
            self.update_sources(dt);
            return;
        };
        let fresh = sources(c, link, owner, i, false, self.emitted);
        let same = fresh.len() == self.scrape.len()
            && fresh.iter().zip(&self.scrape).all(|(a, b)| a.profile.key == b.profile.key);
        if !same {
            self.scrape = fresh;
            self.update_sources(dt);
            return;
        }
        for (source, new) in self.scrape.iter_mut().zip(fresh) {
            source.point = new.point;
            source.rotation = new.rotation;
            source.owner = new.owner;
            source.intensity = i;
        }
        self.update_sources(dt);
    }

    fn update_sources(&mut self, dt: f32) {
        for source in self.hits.iter_mut().chain(&mut self.scrape) {
            for _ in 0..source.births(dt) {
                if self.particles.len() == 1024 {
                    break;
                }
                let p = PcParticle::spawn(source.profile, source.point, source.rotation, source.owner, self.emitted);
                self.particles.push(p);
                self.emitted += 1;
            }
        }
        self.hits.retain(|source| !source.done());
    }

    pub fn geometry(
        &self,
        forward: Vec3,
        textures: &HashMap<u32, (TextureHandle, BlendMode)>,
        out: &mut Vec<TexturedEffect>,
    ) {
        let right = forward.cross(Vec3::Z).try_normalize().unwrap_or(Vec3::X);
        let up = right.cross(forward).try_normalize().unwrap_or(Vec3::Z);
        for p in &self.particles {
            let Some(&(texture, blend)) = textures.get(&p.profile.texture) else { continue };
            let (half, angle) = p.size_angle();
            if !half.is_finite() || half <= 0.0 || !angle.is_finite() {
                continue;
            }
            let (a, b) = match p.profile.constraint {
                1 => (Vec3::X, Vec3::Y),
                2 => (Vec3::X, Vec3::Z),
                3 => (Vec3::Y, Vec3::Z),
                _ => (right, up),
            };
            let (sin, cos) = angle.sin_cos();
            let x = (a * cos + b * sin) * half;
            let y = (-a * sin + b * cos) * half;
            let corners = [p.point - x + y, p.point + x + y, p.point + x - y, p.point - x - y];
            let at = out.iter().position(|l| l.texture == texture && l.blend == blend).unwrap_or_else(|| {
                out.push(TexturedEffect { texture, blend, vertices: Vec::new() });
                out.len() - 1
            });
            let uv = p.uv();
            for i in [0, 1, 2, 0, 2, 3] {
                out[at].vertices.push(EffectVertex {
                    position: corners[i].to_array(),
                    color: p.color(),
                    uv: uv[i],
                    detail: [0.0; 2],
                });
            }
        }
    }
}

fn sources(c: &VisualContact, link: PcSparkLink, owner: Vec3, intensity: f32, hit: bool, seed: u64) -> Vec<PcSource> {
    let normal = space::to_render(c.normal.to_array());
    let axis = match hit {
        true => Vec3::Z,
        false => Vec3::Y,
    };
    let rotation = Quat::from_rotation_arc(axis, normal);
    link.emitters
        .into_iter()
        .flatten()
        .map(|p| {
            PcSource::new(
                p,
                space::to_render(c.point.to_array()),
                rotation,
                owner * link.inherit_velocity,
                intensity,
                hit,
                seed + u64::from(p.key),
            )
        })
        .collect()
}

fn intensity(link: PcSparkLink, input: f32) -> f32 {
    let x = ((input - link.min) / (link.max - link.min)).clamp(0.0, 1.0);
    let [a, b, c, d] = link.quadratic;
    (a + x * (b + x * (c + x * d))).clamp(0.0, 1.0)
}
