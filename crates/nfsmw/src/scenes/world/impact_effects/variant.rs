//! The collision spark variant a mod gives (`SPARKS/<value>/profile.toml`, `docs/modding/effect-variants-sparks-xenon.md`): the
//! sparks a vehicle throws at an impact and at a scrape in the **Original PC** spark style, in place of the install's particle
//! groups for the channels the profile has layers for.
//!
//! A car's contact with a wall or a prop is played whenever it closes (an impact) or slides (a scrape) and the surface has a
//! particle group in the install, within that group's draw distance (`trigger.rs`); the ground and the underside keep the
//! install's acceptance of the contact. What changes is what it throws and how strong it is: the strength comes from the
//! profile's gate (`[hit]`: the closing speed, `[scrape]`: the sliding speed), and it scales the count and the speed of an
//! impact and the rate of a scrape. A channel the profile has no layer for and a surface the install has no group for are the
//! install's, exactly as they were. The sparks here are one small system apart from `PcSparks`: they need a quad along the
//! velocity and an overbright colour, which the install's emitter model does not have.

mod draw;
#[cfg(test)]
mod fixtures;
mod spark;
mod trigger;

pub(super) use trigger::culls;

use std::sync::Arc;

use blackbox_gfx::{BlendMode, PixelFormat, RenderBackend, TextureDesc, TextureHandle, TexturedEffect};
use blackbox_modding::content::{MAX_SPARK_LAYERS, SparkKind, SparkLayer, SparksProfile};
use game_install::GameDir;
use glam::Vec3;

use super::{RawContact, space};
use crate::scenes::world::exhaust::variant::upload_png;
use spark::{Birth, Spark};

/// The most live sparks (the PC budget the install's groups keep).
const LIMIT: usize = 1024;
/// The most cars that scrape at once.
const MOST_SCRAPERS: usize = 64;
/// Seconds a car keeps scraping from its last contact when a step brings none (a wall reacts to a car in some steps only).
const HOLD: f32 = 0.1;

#[cfg(test)]
#[path = "variant_tests.rs"]
mod tests;

/// What was loaded for one profile.
struct Loaded {
    profile: Arc<SparksProfile>,
    /// The picture and blend of each layer (the impact layers, then the scrape layers); `None` draws nothing.
    pictures: Vec<Option<(TextureHandle, BlendMode)>>,
    /// Every texture uploaded for the profile, to free.
    owned: Vec<TextureHandle>,
}

/// The strongest scraping contact a car had in this step.
#[derive(Clone, Copy)]
struct Offer {
    point: Vec3,
    normal: Vec3,
    owner: Vec3,
    strength: f32,
}

/// A car that scrapes: the part of a spark each scrape layer owes it, its contact of this step and its last one.
struct Scraper {
    owner: u64,
    carry: [f32; MAX_SPARK_LAYERS],
    offer: Option<Offer>,
    /// The last contact the car scraped with, and the seconds since.
    held: Option<Offer>,
    idle: f32,
}

/// The collision spark variant in effect and its live sparks.
#[derive(Default)]
pub(super) struct SparkVariantFx {
    /// The profile the scene says is in effect (`None`: the install's groups).
    wanted: Option<Arc<SparksProfile>>,
    loaded: Option<Loaded>,
    /// GPU textures to free at the next chance.
    stale: Vec<TextureHandle>,
    sparks: Vec<Spark>,
    scrapers: Vec<Scraper>,
    emitted: u64,
}

/// The layer `index` of a profile, counted over the impact layers and then the scrape layers.
fn layer_of(profile: &SparksProfile, index: usize) -> Option<&SparkLayer> {
    let hits = profile.hit.items.len();
    match index < hits {
        true => profile.hit.items.get(index),
        false => profile.scrape.items.get(index - hits),
    }
}

fn blend_of(additive: bool) -> BlendMode {
    match additive {
        true => BlendMode::Additive,
        false => BlendMode::AlphaBlend,
    }
}

impl SparkVariantFx {
    /// The profile in effect, or `None` for the install's groups. Nothing is dropped here: the next `maintain` loads the
    /// pictures of a new profile and clears the sparks of the old one.
    pub fn set_wanted(&mut self, profile: Option<Arc<SparksProfile>>) {
        self.wanted = profile.filter(|profile| !profile.hit.items.is_empty() || !profile.scrape.items.is_empty());
    }

    /// Whether the profile in effect is not the one that is loaded.
    fn needs_load(&self) -> bool {
        match (&self.wanted, &self.loaded) {
            (None, None) => false,
            (Some(want), Some(have)) => !Arc::ptr_eq(want, &have.profile) && **want != *have.profile,
            _ => true,
        }
    }

    /// The profile that plays: it is in effect and its pictures are loaded.
    fn ready(&self) -> Option<Arc<SparksProfile>> {
        match self.needs_load() {
            true => None,
            false => self.loaded.as_ref().map(|loaded| loaded.profile.clone()),
        }
    }

    /// Remove every spark and every scraping car.
    pub fn clear(&mut self) {
        self.sparks.clear();
        self.scrapers.clear();
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.sparks.len()
    }

    /// Free the textures that were dropped, then load the profile in effect when it changed.
    pub fn maintain(&mut self, renderer: &mut dyn RenderBackend, dir: &GameDir) {
        if self.needs_load() {
            self.unload();
        }
        for handle in self.stale.drain(..) {
            renderer.destroy_texture(handle);
        }
        if self.wanted.is_some() && self.loaded.is_none() {
            self.load(renderer, dir);
        }
    }

    /// Drop what is loaded and every spark; the textures are freed by the next `maintain`.
    fn unload(&mut self) {
        self.clear();
        let Some(loaded) = self.loaded.take() else { return };
        self.stale.extend(loaded.owned);
    }

    /// Free everything, at the end of the scene.
    pub fn release(&mut self, renderer: &mut dyn RenderBackend) {
        self.unload();
        for handle in self.stale.drain(..) {
            renderer.destroy_texture(handle);
        }
    }

    /// Load the profile in effect: the pictures of its layers from the install's layers, and the game's own streak picture for
    /// the streaks that name none. A layer whose picture cannot be read is kept and draws nothing, with a warning.
    fn load(&mut self, renderer: &mut dyn RenderBackend, dir: &GameDir) {
        let Some(profile) = self.wanted.clone() else { return };
        let mut owned: Vec<TextureHandle> = Vec::new();
        let mut known: Vec<(String, TextureHandle)> = Vec::new();
        let mut streak: Option<TextureHandle> = None;
        let mut pictures = Vec::new();
        for layer in profile.hit.items.iter().chain(&profile.scrape.items) {
            let mut handle = None;
            if let Some(path) = layer.texture.as_deref() {
                handle = match known.iter().find(|(known_path, _)| known_path == path) {
                    Some((_, handle)) => Some(*handle),
                    None => {
                        upload_png(renderer, dir, path, layer.additive, "collision spark variant").map(|(handle, _)| {
                            owned.push(handle);
                            known.push((path.to_owned(), handle));
                            handle
                        })
                    }
                };
            }
            if handle.is_none() && layer.kind == SparkKind::Streak {
                let own = *streak.get_or_insert_with(|| {
                    let (width, height, pixels) = draw::streak_pixels();
                    let handle = renderer.create_texture(&TextureDesc {
                        label: "collision spark streak",
                        width,
                        height,
                        format: PixelFormat::Rgba8,
                        mips: vec![&pixels[..]],
                    });
                    owned.push(handle);
                    handle
                });
                handle = Some(own);
            }
            pictures.push(handle.map(|handle| (handle, blend_of(layer.additive))));
        }
        self.loaded = Some(Loaded { profile, pictures, owned });
    }

    /// Age the live sparks by a step of `dt` seconds.
    pub fn age(&mut self, dt: f32) {
        let Some(profile) = self.loaded.as_ref().map(|loaded| loaded.profile.clone()) else {
            self.sparks.clear();
            return;
        };
        for spark in &mut self.sparks {
            match layer_of(&profile, spark.layer) {
                Some(layer) => spark.step(layer, dt),
                None => spark.age = spark.life,
            }
        }
        self.sparks.retain(Spark::alive);
    }

    /// Play a vehicle's contact from the profile where the profile has a channel for it. `hit` and `scrape` say whether the
    /// install's links accepted the contact and have a particle group, and then give the link's visual cull distance (metres from
    /// the camera). Returns whether the impact and the scrape were taken, so that the install's group is not thrown as well.
    pub fn play(
        &mut self,
        owner: u64,
        c: &RawContact,
        hit: Option<f32>,
        scrape: Option<f32>,
        camera: Vec3,
    ) -> (bool, bool) {
        let Some(profile) = self.ready() else { return (false, false) };
        let point = space::to_render(c.point.to_array());
        let distance = point.distance(camera);
        let (mut took_hit, mut took_scrape) = (false, false);
        if let Some(cull) = hit
            && let Some(strength) = profile.hit.strength(c.closing.length())
        {
            took_hit = true;
            if distance <= cull {
                self.throw_hit(&profile, &birth_of(c, strength, true));
            }
        }
        if let Some(cull) = scrape
            && let Some(strength) = profile.scrape.strength(c.sliding.length())
        {
            took_scrape = true;
            if distance < cull && strength > 0.0 {
                let birth = birth_of(c, strength, false);
                self.offer(owner, Offer { point: birth.point, normal: birth.normal, owner: birth.owner, strength });
            }
        }
        (took_hit, took_scrape)
    }

    /// An impact: every impact layer throws its burst at once, `count * strength` sparks.
    fn throw_hit(&mut self, profile: &SparksProfile, birth: &Birth) {
        for (index, layer) in profile.hit.items.iter().enumerate() {
            let births = (layer.count * birth.strength).round().max(0.0) as usize;
            self.spawn(layer, index, birth, births);
        }
    }

    /// Remember a car's scraping contact of this step if it is the strongest it had so far.
    fn offer(&mut self, owner: u64, offer: Offer) {
        let at = match self.scrapers.iter().position(|scraper| scraper.owner == owner) {
            Some(at) => at,
            None => {
                if self.scrapers.len() >= MOST_SCRAPERS {
                    return;
                }
                self.scrapers.push(Scraper {
                    owner,
                    carry: [0.0; MAX_SPARK_LAYERS],
                    offer: None,
                    held: None,
                    idle: 0.0,
                });
                self.scrapers.len() - 1
            }
        };
        let scraper = &mut self.scrapers[at];
        if scraper.offer.is_some_and(|have| have.strength >= offer.strength) {
            return;
        }
        scraper.offer = Some(offer);
    }

    /// The end of a physics step of `dt` seconds: every car that scraped throws its scrape layers' `rate * strength * dt`
    /// sparks from its strongest contact. A car that brings no contact keeps its last one for `HOLD` seconds, then starts again
    /// from nothing.
    pub fn finish(&mut self, dt: f32) {
        let Some(profile) = self.ready() else {
            self.scrapers.clear();
            return;
        };
        let mut scrapers = std::mem::take(&mut self.scrapers);
        let first = profile.hit.items.len();
        scrapers.retain_mut(|scraper| {
            let offer = match scraper.offer.take() {
                Some(offer) => {
                    scraper.held = Some(offer);
                    scraper.idle = 0.0;
                    offer
                }
                // A car pressed to a wall is reacted to in some steps only; its last contact carries it over the gap.
                None => {
                    scraper.idle += dt.max(0.0);
                    match scraper.held {
                        Some(held) if scraper.idle <= HOLD => held,
                        _ => return false,
                    }
                }
            };
            let birth = Birth {
                point: offer.point,
                normal: offer.normal,
                owner: offer.owner,
                strength: offer.strength,
                hit: false,
            };
            for (slot, layer) in profile.scrape.items.iter().enumerate() {
                let total = scraper.carry[slot] + layer.count * offer.strength * dt.max(0.0);
                let whole = total.floor().max(0.0);
                scraper.carry[slot] = total - whole;
                self.spawn(layer, first + slot, &birth, whole as usize);
            }
            true
        });
        self.scrapers = scrapers;
    }

    fn spawn(&mut self, layer: &SparkLayer, index: usize, birth: &Birth, births: usize) {
        for _ in 0..births {
            if self.sparks.len() >= LIMIT {
                return;
            }
            self.sparks.push(Spark::spawn(layer, index, birth, self.emitted));
            self.emitted += 1;
        }
    }

    /// Append the sparks to `out`, one batch for each picture and blend. `forward` is the camera's direction.
    pub fn textured(&self, forward: Vec3, out: &mut Vec<TexturedEffect>) {
        let Some(loaded) = self.loaded.as_ref().filter(|_| !self.sparks.is_empty()) else { return };
        let right = forward.cross(Vec3::Z).try_normalize().unwrap_or(Vec3::X);
        let up = right.cross(forward).try_normalize().unwrap_or(Vec3::Z);
        for spark in &self.sparks {
            let Some(layer) = layer_of(&loaded.profile, spark.layer) else { continue };
            let Some(&Some((texture, blend))) = loaded.pictures.get(spark.layer) else { continue };
            let look = spark.look(layer);
            if look.size <= 0.0 || look.color[3] == 0 {
                continue;
            }
            let at =
                out.iter().position(|batch| batch.texture == texture && batch.blend == blend).unwrap_or_else(|| {
                    out.push(TexturedEffect { texture, blend, vertices: Vec::new() });
                    out.len() - 1
                });
            draw::push(&mut out[at].vertices, spark, layer, &look, forward, right, up);
        }
    }
}

/// Where and how hard a contact throws, in render space.
fn birth_of(c: &RawContact, strength: f32, hit: bool) -> Birth {
    Birth {
        point: space::to_render(c.point.to_array()),
        normal: space::to_render(c.normal.to_array()),
        owner: space::to_render(c.owner_velocity.to_array()),
        strength,
        hit,
    }
}
