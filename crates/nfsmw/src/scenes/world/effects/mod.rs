//! The tire visual layer, consuming existing physics intensities (spec: tire-effects.md).

mod contacts;
pub(super) mod cop_health;
pub(in crate::scenes::world) mod entry_marks;
#[cfg(test)]
mod ground_tests;
mod marks;
mod profile;
mod rain;
mod smoke;
#[cfg(test)]
mod tests;
pub mod tint;
mod world;

use blackbox_gfx::EffectLayer;
use glam::Vec3;

pub use contacts::{Contact, project};
use marks::Marks;
use profile::SmokeVariant;
use smoke::Smoke;

/// The wheels of a car.
const WHEELS: usize = 4;
pub const MAX_PARTICLES: usize = 512;
pub const MAX_HIGH_PARTICLES: usize = 1536;
pub const MAX_MARKS: usize = 2048;

pub struct TireEffects {
    wet: bool,
    smoke: Smoke,
    /// The smoke of a mod's tire smoke variant, when the player chose one: it is drawn in place of `smoke`.
    variant: SmokeVariant,
    tint: crate::settings::SmokeTint,
    marks: Marks,
    enabled: [bool; 2],
    layer: EffectLayer,
}

impl Default for TireEffects {
    fn default() -> Self {
        Self {
            wet: false,
            smoke: Smoke::default(),
            variant: SmokeVariant::default(),
            tint: crate::settings::SmokeTint::default(),
            marks: Marks::default(),
            enabled: [true; 2],
            layer: EffectLayer::default(),
        }
    }
}

impl TireEffects {
    pub fn set_wet(&mut self, wet: bool) {
        if wet && !self.wet {
            self.smoke.clear();
            self.variant.clear();
        }
        self.wet = wet;
    }
    pub fn set_quality(&mut self, quality: crate::settings::SmokeQuality) {
        self.smoke.set_quality(quality);
    }
    /// How the smoke is coloured (`smoke_color_mode`, `smoke_color`), for the game's smoke and a variant's alike.
    pub fn set_tint(&mut self, tint: crate::settings::SmokeTint) {
        self.tint = tint;
        self.smoke.set_tint(tint);
    }

    /// The tire smoke variant the player chose, if it is a mod's (`None`: the game's own smoke). It is loaded at the next frame
    /// ([`Self::load_variant`]).
    pub fn set_variant(&mut self, profile: Option<std::sync::Arc<blackbox_modding::content::TireSmokeProfile>>) {
        self.variant.set_wanted(profile);
    }

    /// Load the variant's pictures when it changed, or drop them when it went. Does nothing when nothing changed.
    pub fn load_variant(&mut self, renderer: &mut dyn blackbox_gfx::RenderBackend, dir: &game_install::GameDir) {
        self.variant.release(renderer);
        if !self.variant.needs_load() {
            return;
        }
        self.variant.load(renderer, dir);
        self.smoke.clear();
    }

    /// Forget the loaded variant (the car goes): its textures are freed by [`Self::load_variant`] or [`Self::release`].
    pub fn unload_variant(&mut self) {
        self.variant.unload();
    }

    /// Free the GPU textures of a variant that was dropped.
    pub fn release(&mut self, renderer: &mut dyn blackbox_gfx::RenderBackend) {
        self.variant.release(renderer);
    }

    /// Take the vertex buffers of the variant back from the layer `build` made, once it has been uploaded.
    pub fn reclaim_built(&mut self) {
        self.variant.reclaim(&mut self.layer);
    }

    pub fn set_enabled(&mut self, smoke: bool, marks: bool) {
        self.enabled = [smoke, marks];
        if !smoke {
            self.smoke.clear();
            self.variant.clear();
        }
        if !marks {
            self.marks.clear();
        }
    }

    pub fn clear(&mut self) {
        self.smoke.clear();
        self.variant.clear();
        self.marks.clear();
        self.layer.clear();
    }

    /// Break track continuity on respawn or while the physics is parked; old marks may remain.
    pub fn disconnect(&mut self) {
        self.marks.disconnect();
        self.smoke.disconnect();
    }

    pub fn age(&mut self, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        self.smoke.age(dt);
        self.variant.age(dt);
        self.marks.age(dt);
    }

    pub fn step(&mut self, contacts: [Option<Contact>; 4], velocity: Vec3, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 || !velocity.is_finite() {
            return;
        }
        self.smoke.age(dt);
        self.marks.age(dt);
        let variant = self.variant.active();
        self.variant.step(&contacts, variant && self.enabled[0] && !self.wet, velocity, self.tint, dt);
        for (wheel, contact) in contacts.into_iter().enumerate() {
            if self.enabled[0] && !self.wet && !variant {
                self.smoke.emit(wheel, contact, velocity, dt);
            }
            if self.enabled[1] {
                self.marks.sample(wheel, contact);
            }
        }
    }

    pub fn retain_sections(&mut self, loaded: impl Fn(u32) -> bool) {
        self.marks.retain_sections(loaded);
    }

    pub fn build(&mut self, camera: Vec3, forward: Vec3) -> &mut EffectLayer {
        self.layer.clear();
        self.layer.detailed_particles = self.smoke.quality == crate::settings::SmokeQuality::High;
        self.marks.geometry(&mut self.layer.surfaces);
        self.smoke.geometry(camera, forward, &mut self.layer.particles);
        self.variant.geometry(camera, forward, self.tint, &mut self.layer);
        &mut self.layer
    }

    pub fn status(&self) -> String {
        format!(
            "smoke {} ({}): {}/{} live, oldest {:.2}s, {} emitted; marks {}: {}/{} quads, oldest {:.2}s, {} created",
            self.enabled[0],
            self.smoke.quality,
            self.smoke.len(),
            self.smoke.limit(),
            self.smoke.oldest(),
            self.smoke.emitted,
            self.enabled[1],
            self.marks.len(),
            MAX_MARKS,
            self.marks.oldest(),
            self.marks.created,
        )
    }

    pub fn command(&mut self, args: &[&str]) -> Result<String, String> {
        match args {
            [] | ["status"] => return Ok(self.status()),
            ["clear"] => self.clear(),
            [effect @ ("smoke" | "marks"), value @ ("on" | "off")] => {
                let mut enabled = self.enabled;
                enabled[usize::from(*effect == "marks")] = *value == "on";
                self.set_enabled(enabled[0], enabled[1]);
            }
            _ => return Err("usage: tire-effects [status|clear|smoke on/off|marks on/off]".into()),
        }
        Ok(self.status())
    }
}
