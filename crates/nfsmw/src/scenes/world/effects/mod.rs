//! The tire visual layer, consuming existing physics intensities (spec: tire-effects.md).

mod contacts;
#[cfg(test)]
mod ground_tests;
mod marks;
mod smoke;
#[cfg(test)]
mod tests;
mod world;

use blackbox_render::EffectLayer;
use glam::Vec3;

pub use contacts::{Contact, project};
use marks::Marks;
use smoke::Smoke;

pub const MAX_PARTICLES: usize = 512;
pub const MAX_MARKS: usize = 2048;

pub struct TireEffects {
    smoke: Smoke,
    marks: Marks,
    enabled: [bool; 2],
    layer: EffectLayer,
}

impl Default for TireEffects {
    fn default() -> Self {
        Self { smoke: Smoke::default(), marks: Marks::default(), enabled: [true; 2], layer: EffectLayer::default() }
    }
}

impl TireEffects {
    pub fn set_enabled(&mut self, smoke: bool, marks: bool) {
        self.enabled = [smoke, marks];
        if !smoke {
            self.smoke.clear();
        }
        if !marks {
            self.marks.clear();
        }
    }

    pub fn clear(&mut self) {
        self.smoke.clear();
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
        self.marks.age(dt);
    }

    pub fn step(&mut self, contacts: [Option<Contact>; 4], velocity: Vec3, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 || !velocity.is_finite() {
            return;
        }
        self.age(dt);
        for (wheel, contact) in contacts.into_iter().enumerate() {
            if self.enabled[0] {
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

    pub fn build(&mut self, camera: Vec3, forward: Vec3) -> &EffectLayer {
        self.layer.clear();
        self.marks.geometry(&mut self.layer.surfaces);
        self.smoke.geometry(camera, forward, &mut self.layer.particles);
        &self.layer
    }

    pub fn status(&self) -> String {
        format!(
            "smoke {}: {}/{} live, oldest {:.2}s, {} emitted; marks {}: {}/{} quads, oldest {:.2}s, {} created",
            self.enabled[0],
            self.smoke.len(),
            MAX_PARTICLES,
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
