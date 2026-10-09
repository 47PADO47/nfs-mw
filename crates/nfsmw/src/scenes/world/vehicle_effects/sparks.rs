use std::collections::VecDeque;

use blackbox_render::EffectVertex;
use glam::Vec3;
use nfsmw_data::vehicle_effects::{CollisionEffects, SparkLink};

use super::super::{drive::VisualContact, space};
use super::{MAX_SPARKS, geometry};

#[cfg(test)]
#[path = "spark_tests.rs"]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Particle {
    point: Vec3,
    velocity: Vec3,
    age: f32,
    life: f32,
    color: [f32; 4],
}

pub(super) struct Sparks {
    particles: VecDeque<Particle>,
    carry: f32,
    pub emitted: u64,
}

impl Default for Sparks {
    fn default() -> Self {
        Self { particles: VecDeque::with_capacity(MAX_SPARKS), carry: 0.0, emitted: 0 }
    }
}

impl Sparks {
    pub fn len(&self) -> usize {
        self.particles.len()
    }
    pub fn clear(&mut self) {
        self.particles.clear();
        self.disconnect();
    }
    pub fn disconnect(&mut self) {
        self.carry = 0.0;
    }

    pub fn age(&mut self, dt: f32) {
        for p in &mut self.particles {
            p.age += dt;
            p.point += p.velocity * dt + Vec3::NEG_Z * (4.905 * dt * dt);
            p.velocity += Vec3::NEG_Z * (9.81 * dt);
        }
        self.particles.retain(|p| p.age < p.life && p.point.is_finite());
    }

    pub fn emit(&mut self, contacts: &[VisualContact], data: &CollisionEffects, dt: f32) {
        self.emit_selected(
            contacts,
            |surface, hit| match hit {
                true => data.hit(surface),
                false => data.scrape(surface),
            },
            dt,
        );
    }

    fn emit_selected(&mut self, contacts: &[VisualContact], pick: impl Fn(u32, bool) -> Option<SparkLink>, dt: f32) {
        let mut hit: Option<(&VisualContact, SparkLink, f32)> = None;
        let mut scrape: Option<(&VisualContact, SparkLink, f32)> = None;
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
            let normal_speed = c.velocity.dot(c.normal);
            let sliding = (c.velocity - c.normal * normal_speed).length();
            if -normal_speed > 1.0
                && c.impulse_delta_v > 0.0
                && let Some(link) = pick(surface, true)
            {
                let strength = intensity(link, c.impulse_delta_v, false);
                if strength > hit.map_or(0.0, |(_, _, s)| s) {
                    hit = Some((c, link, strength));
                }
            }
            if sliding > 1.0
                && normal_speed <= 0.1
                && let Some(link) = pick(surface, false)
            {
                let strength = intensity(link, sliding, true);
                if strength > scrape.map_or(0.0, |(_, _, s)| s) {
                    scrape = Some((c, link, strength));
                }
            }
        }
        if let Some((c, link, strength)) = hit {
            self.spawn(c, link, (strength * 48.0).ceil().min(48.0) as usize);
        }
        let Some((c, link, strength)) = scrape else {
            self.disconnect();
            return;
        };
        // Host rate policy: one strongest active contact; no backlog after contact is lost.
        self.carry = (self.carry + strength * 160.0 * dt).min(24.0);
        let count = self.carry.floor() as usize;
        self.carry -= count as f32;
        self.spawn(c, link, count);
    }

    fn spawn(&mut self, c: &VisualContact, link: SparkLink, count: usize) {
        let normal = space::to_render(c.normal.to_array());
        let velocity = space::to_render(c.velocity.to_array());
        let tangent = velocity - normal * velocity.dot(normal);
        let side = normal.cross(Vec3::Z).normalize_or(Vec3::X);
        let Some(first) = link.styles.into_iter().flatten().next() else { return };
        let second = link.styles.into_iter().flatten().nth(1).unwrap_or(first);
        for _ in 0..count {
            let style = match self.emitted.is_multiple_of(2) {
                true => first,
                false => second,
            };
            let n = self.emitted;
            let variation = geometry::noise(n, 0.0).abs();
            let life = (style.life + geometry::noise(n, 1.1) * style.life_variance).clamp(0.05, 1.5);
            if self.particles.len() == MAX_SPARKS {
                self.particles.pop_front();
            }
            self.particles.push_back(Particle {
                point: space::to_render(c.point.to_array()) + normal * 0.03,
                velocity: tangent * 0.15
                    + normal * (1.2 + variation * 2.0)
                    + Vec3::Z * (1.0 + variation * 2.5)
                    + side * geometry::noise(n, 2.0) * 2.0,
                age: 0.0,
                life,
                color: style.color,
            });
            self.emitted += 1;
        }
    }

    pub fn geometry(&self, camera: Vec3, forward: Vec3, out: &mut Vec<EffectVertex>) {
        for p in &self.particles {
            let length = (0.12 + p.velocity.length() * 0.025).clamp(0.12, 0.65);
            let tail = p.point - p.velocity.normalize_or(Vec3::Z) * length;
            let fade = (1.0 - p.age / p.life).powi(2);
            geometry::streak(out, p.point, tail, 0.016, geometry::color(p.color, fade), camera, forward);
        }
    }
}

fn intensity(link: SparkLink, input: f32, scrape: bool) -> f32 {
    if !link.min.is_finite() || !link.max.is_finite() || link.max <= link.min {
        return 0.0;
    }
    let value = ((input - link.min) / (link.max - link.min)).clamp(0.0, 1.0);
    if scrape {
        return value.max(0.1);
    }
    value
}
