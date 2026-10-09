use std::collections::VecDeque;

use blackbox_render::EffectVertex;
use glam::{Quat, Vec3};
use nfsmw_data::vehicle_effects::{CollisionEffects, SparkLink};

use super::super::{drive::VisualContact, space};
use super::body::BodyClip;
use super::particle::{Particle, count};
use super::{MAX_SPARKS, geometry};

#[cfg(test)]
#[path = "spark_tests.rs"]
mod tests;

pub(super) struct Sparks {
    particles: VecDeque<Particle>,
    flashes: VecDeque<Flash>,
    scrape_flash: Option<Flash>,
    owner_velocity: Vec3,
    carry: f32,
    pub emitted: u64,
}

struct Flash {
    point: Vec3,
    strength: f32,
    color: [f32; 4],
    age: f32,
}

impl Default for Sparks {
    fn default() -> Self {
        Self {
            particles: VecDeque::with_capacity(MAX_SPARKS),
            flashes: VecDeque::with_capacity(16),
            scrape_flash: None,
            owner_velocity: Vec3::ZERO,
            carry: 0.0,
            emitted: 0,
        }
    }
}

impl Sparks {
    pub fn len(&self) -> usize {
        self.particles.len()
    }
    pub fn clear(&mut self) {
        self.particles.clear();
        self.flashes.clear();
        self.scrape_flash = None;
        self.disconnect();
    }
    pub fn disconnect(&mut self) {
        self.carry = 0.0;
    }

    pub fn age(&mut self, dt: f32) {
        self.particles.iter_mut().for_each(|p| p.age(dt));
        self.particles.retain(Particle::alive);
        self.flashes.iter_mut().for_each(|f| f.age += dt);
        self.flashes.retain(|f| f.age < 0.12);
        if let Some(flash) = &mut self.scrape_flash {
            flash.age += dt;
            if flash.age >= 0.12 {
                self.scrape_flash = None;
            }
        }
    }

    pub fn emit(&mut self, contacts: &[VisualContact], data: &CollisionEffects, velocity: Vec3, dt: f32) {
        self.owner_velocity = velocity;
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
                && sliding >= link.min
            {
                let strength = intensity(link, sliding, true);
                if strength > scrape.map_or(0.0, |(_, _, s)| s) {
                    scrape = Some((c, link, strength));
                }
            }
        }
        if let Some((c, link, strength)) = hit {
            self.spawn(c, link, 1, true);
            if let Some(style) = link.styles.into_iter().flatten().next() {
                if self.flashes.len() == 16 {
                    self.flashes.pop_front();
                }
                self.flashes.push_back(Flash {
                    point: space::to_render(c.point.to_array()) + space::to_render(c.normal.to_array()) * 0.04,
                    strength,
                    color: style.color,
                    age: 0.0,
                });
            }
            self.disconnect();
            return;
        }
        let Some((c, link, strength)) = scrape else {
            self.disconnect();
            return;
        };
        // The restoration forces spark intensity to one; weak scrapes do not reduce density.
        self.carry = (self.carry + 60.0 * dt).min(4.0);
        let dispatches = self.carry.floor() as usize;
        self.carry -= dispatches as f32;
        self.spawn(c, link, dispatches, false);
        if let Some(style) = link.styles.into_iter().flatten().next() {
            self.scrape_flash = Some(Flash {
                point: space::to_render(c.point.to_array()) + space::to_render(c.normal.to_array()) * 0.04,
                strength: 0.35 + 0.65 * strength,
                color: style.color,
                age: 0.0,
            });
        }
    }

    fn spawn(&mut self, c: &VisualContact, link: SparkLink, dispatches: usize, hit: bool) {
        let normal = space::to_render(c.normal.to_array());
        let velocity = self.owner_velocity * link.inherit_velocity;
        let origin = space::to_render(c.point.to_array());
        let local_axis = match hit {
            true => Vec3::Z,
            false => Vec3::Y,
        };
        let rotation = Quat::from_rotation_arc(local_axis, normal);
        for (index, style) in link.styles.into_iter().enumerate() {
            let Some(style) = style else { continue };
            for _ in 0..count(style, 1.0) * dispatches {
                let mut p = Particle::spawn(style, origin, rotation, velocity, self.emitted, 1.0);
                p.origin += normal * (0.02 - (p.origin - origin).dot(normal)).max(0.0);
                p.previous = p.origin;
                p.elasticity = [160.0, 120.0][index] / 255.0;
                if self.particles.len() == MAX_SPARKS {
                    self.particles.pop_front();
                }
                self.particles.push_back(p);
                self.emitted += 1;
            }
        }
    }

    pub fn bounce(&mut self, world: &blackbox_collision::CollisionWorld, dt: f32) {
        for p in &mut self.particles {
            if p.bounces >= 4 || p.age == 0.0 {
                continue;
            }
            let Some(hit) = super::sweep::world_hit(
                world,
                Vec3::from(space::to_physics(p.previous)),
                Vec3::from(space::to_physics(p.position(p.age))),
            ) else {
                continue;
            };
            p.bounce(hit.t, space::to_render(hit.point), space::to_render(hit.normal), dt);
        }
    }

    pub fn geometry(&self, camera: Vec3, forward: Vec3, body: Option<BodyClip>, out: &mut Vec<EffectVertex>) {
        for p in &self.particles {
            let head = p.position(p.age + p.duration);
            let tail = p.position(p.age);
            let ranges = body.map_or([[0.0, 1.0], [1.0, 1.0]], |b| b.outside(head, tail));
            for range in ranges {
                geometry::streak_range(
                    out,
                    head,
                    tail,
                    p.width * 0.5,
                    geometry::color(p.color, 1.0),
                    camera,
                    forward,
                    range,
                );
            }
        }
    }

    pub fn glows(&self, camera: Vec3, forward: Vec3, body: Option<BodyClip>, out: &mut Vec<EffectVertex>) {
        for p in &self.particles {
            let head = p.position(p.age + p.duration);
            if body.is_some_and(|b| b.contains(head)) {
                continue;
            }
            geometry::glow(out, head, p.width, geometry::color(p.color, 0.3), camera, forward);
        }
        for f in self.flashes.iter().chain(self.scrape_flash.iter()) {
            let fade = (1.0 - f.age / 0.12).powi(2) * f.strength;
            geometry::glow(out, f.point, 0.2 + 0.4 * f.strength, geometry::color(f.color, fade), camera, forward);
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
