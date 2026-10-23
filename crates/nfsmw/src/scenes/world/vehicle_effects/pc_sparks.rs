//! Stock collision particle groups; never replaced by the restoration fallback.

use super::super::{drive::VisualContact, space};
use super::{pc_emitter::PcSource, pc_particle::PcParticle};
use blackbox_gfx::{BlendMode, EffectVertex, TextureHandle, TexturedEffect};
use glam::{Mat3, Quat, Vec3};
use nfsmw_data::vehicle_effects::PcSparkLink;
use std::collections::HashMap;

pub(in crate::scenes::world) struct PcSparks {
    particles: Vec<PcParticle>,
    hits: Vec<PcSource>,
    scrape: Vec<PcSource>,
    pub emitted: u64,
    limit: usize,
}

impl Default for PcSparks {
    fn default() -> Self {
        Self { particles: Vec::new(), hits: Vec::new(), scrape: Vec::new(), emitted: 0, limit: 1024 }
    }
}

impl PcSparks {
    pub fn surface_group(
        &mut self,
        emitters: [Option<nfsmw_data::vehicle_effects::PcEmitter>; 8],
        point: Vec3,
        rotation: Quat,
        owner: Vec3,
        intensity: f32,
    ) {
        let profiles: Vec<_> = emitters.into_iter().flatten().collect();
        let same = profiles.len() == self.scrape.len()
            && profiles.iter().zip(&self.scrape).all(|(a, b)| a.key == b.profile.key);
        if !same {
            self.scrape = profiles
                .into_iter()
                .map(|p| PcSource::new(p, point, rotation, owner, intensity, false, self.emitted + u64::from(p.key)))
                .collect();
        }
        for source in &mut self.scrape {
            source.point = point;
            source.rotation = rotation;
            source.frame = Mat3::from_quat(rotation);
            source.owner = owner;
            source.intensity = intensity;
        }
    }
    pub fn set_limit(&mut self, limit: usize) {
        self.limit = limit.clamp(1, 1024);
    }
    pub fn stop_scrape(&mut self) {
        self.scrape.clear();
    }
    pub fn submit(&mut self, point: Vec3, normal: Vec3, link: PcSparkLink, owner: Vec3, intensity: f32, hit: bool) {
        // A one-shot's native wrapper creates no group at zero mapped intensity.
        if hit && intensity <= 0.0 {
            return;
        }
        let c = VisualContact {
            point,
            normal,
            velocity: Vec3::ZERO,
            impulse_delta_v: 0.0,
            surface: None,
            kind: super::super::drive::walls::ContactKind::Face,
            info: None,
            prop: None,
        };
        let mut fresh = sources(&c, link, owner, intensity, hit, self.emitted);
        let frame = contact_frame(normal, hit);
        for source in &mut fresh {
            source.frame = frame;
        }
        if hit {
            if self.hits.len() + fresh.len() <= 128 {
                self.hits.extend(fresh);
            }
            return;
        }
        let same = fresh.len() == self.scrape.len()
            && fresh.iter().zip(&self.scrape).all(|(a, b)| a.profile.key == b.profile.key);
        if !same {
            self.scrape = fresh;
            return;
        }
        for (source, new) in self.scrape.iter_mut().zip(fresh) {
            source.point = new.point;
            source.rotation = new.rotation;
            source.frame = new.frame;
            source.owner = new.owner;
            source.intensity = intensity;
        }
    }
    pub fn update(&mut self, dt: f32) {
        self.update_sources(dt);
    }
    pub fn pending(&self) -> bool {
        !self.hits.is_empty() || !self.scrape.is_empty()
    }
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

    fn update_sources(&mut self, dt: f32) {
        for source in self.hits.iter_mut().chain(&mut self.scrape) {
            // Disabled persistent groups keep their delay/cycle phase until re-enabled.
            if source.intensity <= 0.0 {
                continue;
            }
            for _ in 0..source.births(dt) {
                if self.particles.len() >= self.limit {
                    break;
                }
                let p = PcParticle::spawn_frame(source.profile, source.point, source.frame, source.owner, self.emitted);
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
        self.geometry_with_brightness(forward, textures, out, 1.0);
    }

    pub fn geometry_with_brightness(
        &self,
        forward: Vec3,
        textures: &HashMap<u32, (TextureHandle, BlendMode)>,
        out: &mut Vec<TexturedEffect>,
        brightness: f32,
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
                    detail: [brightness, 0.0],
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

fn contact_frame(normal: Vec3, hit: bool) -> Mat3 {
    if !hit {
        return Mat3::from_quat(Quat::from_rotation_arc(Vec3::Y, space::to_render(normal.to_array())));
    }
    let rotation = Mat3::from_quat(Quat::from_rotation_arc(Vec3::Y, normal));
    Mat3::from_cols(
        space::to_render(rotation.z_axis.to_array()),
        -space::to_render(rotation.x_axis.to_array()),
        space::to_render(rotation.y_axis.to_array()),
    )
}
#[cfg(test)]
mod contact_frame_tests {
    use super::*;
    #[test]
    fn one_shot_rotates_physics_up_then_swizzles_all_basis_vectors() {
        let normal = Vec3::new(0.3, 0.4, 0.5).normalize();
        let frame = contact_frame(normal, true);
        assert!(frame.z_axis.distance(space::to_render(normal.to_array())) < 1e-6);
        assert!((frame.determinant() - 1.0).abs() < 1e-6);
        assert_eq!(contact_frame(Vec3::Y, true), Mat3::IDENTITY);
    }
}
