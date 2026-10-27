//! Source-owned native PC contact particles; independent of audio and damage sequencers.
mod contact;
mod variant;
use super::{space, vehicle_effects::pc_sparks::PcSparks};
use blackbox_attrib::{Database, vlt_hash};
use blackbox_gfx::{BlendMode, RenderBackend, TextureHandle, TexturedEffect};
pub use contact::{Contact, Kind, RawContact, next_owner};
use game_install::GameDir;
use glam::Vec3;
use nfsmw_data::{
    car::exhaust::{ADDITIVE_BLEND, particle_textures},
    vehicle_effects::contacts::{Action, Link, SourceEffects},
};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    rc::Weak,
};

struct Bank {
    particles: PcSparks,
    owner: Weak<()>,
    timeout: f32,
}

#[derive(Default)]
pub struct ImpactEffects {
    profiles: HashMap<(u32, u32), (u64, SourceEffects)>,
    banks: BTreeMap<u64, Bank>,
    textures: HashMap<u32, (TextureHandle, BlendMode)>,
    /// The collision spark variant a mod gives, for vehicles (`variant.rs`).
    variant: variant::SparkVariantFx,
    pending: BTreeSet<u32>,
    missing: BTreeSet<u32>,
    pub accepted: u64,
    clock: u64,
    disabled: bool,
}

impl ImpactEffects {
    pub fn clear(&mut self) {
        self.banks.clear();
        self.variant.clear();
        self.pending.clear();
    }
    pub fn status(&self) -> String {
        let live: usize = self.banks.values().map(|bank| bank.particles.len()).sum();
        let emitted: u64 = self.banks.values().map(|bank| bank.particles.emitted).sum();
        format!(
            "source collision particles {}: {live}/16384 live, {} owners, {emitted} active-owner births, {} accepted hits",
            !self.disabled,
            self.banks.len(),
            self.accepted
        )
    }
    /// The collision spark variant a mod gives (`SPARKS/<value>/profile.toml`), or `None` for the install's groups. Its pictures
    /// load at the next `maintain`.
    pub fn set_variant(&mut self, profile: Option<std::sync::Arc<blackbox_modding::content::SparksProfile>>) {
        self.variant.set_wanted(profile);
    }
    pub fn set_enabled(&mut self, enabled: bool) {
        self.disabled = !enabled;
        if !enabled {
            self.clear();
        }
    }

    pub fn step(&mut self, db: &Database, contacts: impl IntoIterator<Item = Contact>, camera: Vec3, dt: f32) {
        if self.disabled || !dt.is_finite() || dt <= 0.0 {
            return;
        }
        self.clock = self.clock.wrapping_add(1);
        self.variant.age(dt);
        let vehicle = vlt_hash("pvehicle");
        for bank in self.banks.values_mut() {
            bank.particles.age(dt);
            bank.timeout -= dt;
            if bank.owner.upgrade().is_none() {
                bank.timeout = -1.0;
            }
            if bank.timeout < 0.0 {
                bank.particles.stop_scrape();
            }
        }
        for contact in contacts {
            let c = contact.raw;
            if contact.lifetime.upgrade().is_none() {
                continue;
            }
            if !c.point.is_finite()
                || !c.normal.is_finite()
                || !c.closing.is_finite()
                || !c.sliding.is_finite()
                || !c.owner_velocity.is_finite()
                || (c.normal.length_squared() - 1.0).abs() > 0.05
            {
                continue;
            }
            let key = (contact.source.class, contact.source.collection);
            if !self.profiles.contains_key(&key) {
                if self.profiles.len() >= 256 {
                    let oldest = self.profiles.iter().min_by_key(|(key, (age, _))| (*age, **key)).map(|(key, _)| *key);
                    if let Some(oldest) = oldest {
                        self.profiles.remove(&oldest);
                    }
                }
                let Some(source) = db.resolve(contact.source) else { continue };
                self.profiles.insert(key, (self.clock, SourceEffects::read(db, source)));
            }
            let (age, profile) = self.profiles.get_mut(&key).unwrap();
            *age = self.clock;
            let detached = (contact.source.class == vlt_hash("smackable") && c.detached)
                .then(|| profile.pick(Action::Detached, c.surface))
                .flatten()
                .and_then(|link| link.hit(c.closing.length_squared(), c.closing.length()).map(|i| (link, i)));
            let (hit, scrape) = actions(c.kind, c.bottom);
            let impact = if c.bottom { c.closing.length() } else { c.delta_v };
            let mut accepted_hit = detached.or_else(|| {
                profile
                    .pick(hit, c.surface)
                    .and_then(|link| link.hit(c.closing.length_squared(), impact).map(|i| (link, i)))
            });
            let mut scrape = if detached.is_none() {
                profile.pick(scrape, c.surface).and_then(|link| link.scrape(c.sliding.length()).map(|i| (link, i)))
            } else {
                None
            };
            // A collision spark variant plays a vehicle's contact from its profile where it has a channel for it, and the
            // install's group is then not thrown as well; the install's own acceptance of the contact (above) stays.
            if contact.source.class == vehicle {
                let cull =
                    |taken: Option<(Link, f32)>| taken.and_then(|(link, _)| link.particles.map(|_| link.visual_cull));
                let (took_hit, took_scrape) =
                    self.variant.play(contact.owner, &c, cull(accepted_hit), cull(scrape), camera);
                if took_hit {
                    accepted_hit = None;
                }
                if took_scrape {
                    scrape = None;
                }
            }
            if !self.banks.contains_key(&contact.owner) {
                if self.banks.len() >= 64 || (accepted_hit.is_none() && scrape.is_none()) {
                    continue;
                }
                let mut particles = PcSparks::default();
                particles.set_limit(256);
                self.banks.insert(contact.owner, Bank { particles, owner: contact.lifetime.clone(), timeout: 0.0 });
            }
            let bank = self.banks.get_mut(&contact.owner).unwrap();
            if let Some((link, intensity)) = accepted_hit {
                self.accepted += 1;
                let hit_contact = if detached.is_some() { RawContact { normal: c.detached_normal, ..c } } else { c };
                submit(bank, &mut self.pending, hit_contact, link, intensity, camera, true);
            }
            if detached.is_some() {
                continue;
            }
            if let Some((link, intensity)) = scrape {
                bank.timeout = 0.2;
                submit(bank, &mut self.pending, c, link, intensity, camera, false);
                continue;
            }
            // DoScrape's low-speed branch leaves the existing timeout untouched.
            if c.sliding.length() > 1.0 {
                bank.particles.stop_scrape();
            }
        }
        self.variant.finish(dt);
        for bank in self.banks.values_mut() {
            bank.particles.update(dt);
        }
        self.banks.retain(|_, bank| bank.timeout >= 0.0 || bank.particles.len() != 0 || bank.particles.pending());
    }
    pub fn maintain(&mut self, renderer: &mut dyn RenderBackend, dir: &GameDir) {
        self.variant.maintain(renderer, dir);
        let wanted: Vec<_> = self
            .pending
            .iter()
            .copied()
            .filter(|h| !self.textures.contains_key(h) && !self.missing.contains(h))
            .collect();
        if wanted.is_empty() {
            return;
        }
        if let Ok(found) = particle_textures(dir, &wanted) {
            for texture in found {
                let Some(handle) = blackbox_scene::upload_texture(renderer, &texture) else { continue };
                let blend =
                    if texture.alpha_blend == ADDITIVE_BLEND { BlendMode::Additive } else { BlendMode::AlphaBlend };
                self.textures.insert(texture.name_hash, (handle, blend));
            }
        }
        for hash in wanted.into_iter().filter(|h| !self.textures.contains_key(h)) {
            if self.missing.insert(hash) {
                log::warn!("contact particle texture {hash:08X} is unavailable");
            }
        }
        self.pending.clear();
    }
    pub fn textured(&self, forward: Vec3, out: &mut Vec<TexturedEffect>) {
        for bank in self.banks.values() {
            bank.particles.geometry(forward, &self.textures, out);
        }
        self.variant.textured(forward, out);
    }
    pub fn release(&mut self, renderer: &mut dyn RenderBackend) {
        self.variant.release(renderer);
        self.banks.clear();
        self.profiles.clear();
        self.pending.clear();
        self.missing.clear();
        for (_, (handle, _)) in self.textures.drain() {
            renderer.destroy_texture(handle);
        }
    }
}

fn actions(kind: Kind, bottom: bool) -> (Action, Action) {
    if bottom {
        return (Action::BottomOut, Action::BottomScrape);
    }
    match kind {
        Kind::World => (Action::HitWorld, Action::ScrapeWorld),
        Kind::Object => (Action::HitObject, Action::ScrapeObject),
        Kind::Ground => (Action::HitGround, Action::ScrapeGround),
    }
}

fn submit(
    bank: &mut Bank,
    wanted: &mut BTreeSet<u32>,
    c: RawContact,
    link: Link,
    magnitude: f32,
    camera: Vec3,
    hit: bool,
) {
    let distance = space::to_render(c.point.to_array()).distance(camera);
    let visible = if hit { distance <= link.visual_cull } else { distance < link.visual_cull };
    let Some(particles) = link.particles.filter(|_| visible) else {
        if !hit {
            bank.particles.stop_scrape();
        }
        return;
    };
    let intensity = link.intensity(magnitude);
    for emitter in particles.emitters.iter().flatten() {
        wanted.insert(emitter.texture);
    }
    bank.particles.submit(c.point, c.normal, particles, space::to_render(c.owner_velocity.to_array()), intensity, hit);
}

#[cfg(test)]
mod gpu_tests;
#[cfg(test)]
mod tests;
