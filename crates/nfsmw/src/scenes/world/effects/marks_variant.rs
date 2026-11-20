//! The skid marks of a mod's variant (`docs/modding/effect-variants-marks-rain.md`): the pictures of the profile's sets, drawn on
//! the strips the tyres leave in place of the game's dark untextured ones.
//!
//! The strips, where they begin and end, and how wide the tread is are the game's own (`marks.rs`, the physics' skid intensity and
//! for a bike `bike_slide.rs`); the variant only says what a strip looks like. Each mark of a road kind the profile has a set for
//! is cut at every repeat of the set's picture along its track (so the picture's texture coordinates never leave 0 to 1 and the
//! sampler's wrapping does not matter) and drawn with the slip picture when the tyre slid sideways or the spin picture when it spun
//! or locked. A mark the profile has no set (or no readable picture) for keeps the game's dark strip.

use std::sync::Arc;

use blackbox_gfx::RenderBackend;
use blackbox_modding::content::{MarkSet, SkidMarksProfile};
use blackbox_render::{BlendMode, EffectLayer, EffectVertex, TextureHandle, TexturedEffect};
use game_install::GameDir;
use glam::Vec3;

use super::marks::{Mark, Marks};
use crate::scenes::world::exhaust::variant::upload_png;

/// How far above the road the textured strips are lifted, metres. The textured pass has no depth bias of its own.
const LIFT: f32 = 0.03;

/// The two pictures of a set, as slots of `Loaded::textures`; the one a set lacks reads the other.
struct Pictures {
    slip: Option<usize>,
    spin: Option<usize>,
}

/// What was loaded for one profile.
struct Loaded {
    profile: Arc<SkidMarksProfile>,
    /// A picture on the GPU; `None` for one that could not be read.
    textures: Vec<Option<TextureHandle>>,
    /// For each set of the profile, its pictures.
    sets: Vec<Pictures>,
    /// The vertices of each picture's batch, kept between frames.
    batches: Vec<Vec<EffectVertex>>,
    /// The batches handed to the layer by the latest `geometry`: the picture's slot, its texture and the batch's place in the layer.
    handed: Vec<(usize, TextureHandle, usize)>,
}

/// The skid mark variant of the car's tyres.
#[derive(Default)]
pub(super) struct MarksVariant {
    /// The profile the scene says is in effect (`None`: the game's own marks).
    wanted: Option<Arc<SkidMarksProfile>>,
    loaded: Option<Loaded>,
    /// GPU textures to free at the next chance.
    stale: Vec<TextureHandle>,
}

impl MarksVariant {
    pub(super) fn set_wanted(&mut self, profile: Option<Arc<SkidMarksProfile>>) {
        self.wanted = profile;
    }

    /// The profile in effect, which the marks read the width and the life of a set from.
    pub(super) fn profile(&self) -> Option<&SkidMarksProfile> {
        self.wanted.as_deref()
    }

    /// Whether the profile in effect is not the one that is loaded.
    pub(super) fn needs_load(&self) -> bool {
        match (&self.wanted, &self.loaded) {
            (None, None) => false,
            (Some(want), Some(have)) => !Arc::ptr_eq(want, &have.profile) && **want != *have.profile,
            _ => true,
        }
    }

    /// Drop what is loaded; its textures are freed by [`MarksVariant::release`].
    pub(super) fn unload(&mut self) {
        let Some(loaded) = self.loaded.take() else { return };
        self.stale.extend(loaded.textures.iter().flatten());
    }

    /// Free the GPU textures that were dropped.
    pub(super) fn release(&mut self, renderer: &mut dyn RenderBackend) {
        for handle in self.stale.drain(..) {
            renderer.destroy_texture(handle);
        }
    }

    /// Load the pictures of the profile in effect from the install's layers. A picture that cannot be read is left out with a
    /// warning, and the marks that would use it keep the game's dark strip.
    pub(super) fn load(&mut self, renderer: &mut dyn RenderBackend, dir: &GameDir) {
        self.unload();
        let Some(profile) = self.wanted.clone() else { return };
        let mut paths: Vec<String> = Vec::new();
        let mut textures = Vec::new();
        let mut slot = |path: &Option<String>, renderer: &mut dyn RenderBackend| -> Option<usize> {
            let path = path.as_deref()?;
            let found = paths.iter().position(|known| known == path);
            let at = found.unwrap_or_else(|| {
                paths.push(path.to_owned());
                textures.push(upload_png(renderer, dir, path, false, "skid marks variant").map(|(handle, _)| handle));
                textures.len() - 1
            });
            textures[at].map(|_| at)
        };
        let sets = profile
            .sets
            .iter()
            .map(|set| {
                let (slip, spin) = (slot(&set.slip, renderer), slot(&set.spin, renderer));
                Pictures { slip: slip.or(spin), spin: spin.or(slip) }
            })
            .collect();
        let batches = vec![Vec::new(); textures.len()];
        self.loaded = Some(Loaded { profile, textures, sets, batches, handed: Vec::new() });
    }

    /// Whether this variant draws `mark` with a picture (the game's dark strip is then left out of the layer).
    pub(super) fn covers(&self, mark: &Mark) -> bool {
        self.pictures_of(mark).is_some()
    }

    fn pictures_of(&self, mark: &Mark) -> Option<(usize, &MarkSet, usize)> {
        let loaded = self.loaded.as_ref().filter(|_| !self.needs_load())?;
        pictures_of(loaded, mark)
    }

    /// Append one textured batch per picture with marks to `layer`. Call [`MarksVariant::reclaim`] once the layer has been uploaded.
    pub(super) fn geometry(&mut self, marks: &Marks, layer: &mut EffectLayer) {
        let Some(loaded) = self.loaded.as_mut() else { return };
        loaded.handed.clear();
        for batch in &mut loaded.batches {
            batch.clear();
        }
        for mark in marks.iter() {
            let Some((slot, set, _)) = pictures_of(loaded, mark) else { continue };
            let tile = match mark.slip {
                true => set.slip_tile,
                false => set.spin_tile,
            };
            let alpha = (mark.strength() * set.opacity * mark.fade() * 255.0) as u8;
            push_strip(&mut loaded.batches[slot], mark, tile, alpha);
        }
        for (slot, batch) in loaded.batches.iter_mut().enumerate() {
            let (Some(handle), false) = (loaded.textures[slot], batch.is_empty()) else { continue };
            let vertices = std::mem::take(batch);
            loaded.handed.push((slot, handle, layer.textured.len()));
            layer.textured.push(TexturedEffect { texture: handle, blend: BlendMode::AlphaBlend, vertices });
        }
    }

    /// Take the vertex buffers back from `layer`.
    pub(super) fn reclaim(&mut self, layer: &mut EffectLayer) {
        let Some(loaded) = self.loaded.as_mut() else { return };
        for (slot, handle, at) in loaded.handed.drain(..) {
            let Some(batch) = layer.textured.get_mut(at).filter(|batch| batch.texture == handle) else { continue };
            loaded.batches[slot] = std::mem::take(&mut batch.vertices);
        }
    }

    /// How many marks the variant draws with a picture, and how many it leaves to the game's strip (for the console).
    pub(super) fn counts(&self, marks: &Marks) -> (usize, usize) {
        let drawn = marks.iter().filter(|mark| self.covers(mark)).count();
        (drawn, marks.len() - drawn)
    }
}

/// The picture of `mark` in `loaded`: its slot, its set and the set's index.
fn pictures_of<'a>(loaded: &'a Loaded, mark: &Mark) -> Option<(usize, &'a MarkSet, usize)> {
    let profile = &loaded.profile;
    let index = profile
        .sets
        .iter()
        .position(|set| set.surface == mark.surface() && set.wet == mark.wet)
        .or_else(|| profile.sets.iter().position(|set| set.surface == mark.surface() && !set.wet))?;
    let pictures = &loaded.sets[index];
    let slot = match mark.slip {
        true => pictures.slip,
        false => pictures.spin,
    }?;
    Some((slot, &profile.sets[index], index))
}

/// Append the triangles of `mark`, cut at every repeat of a picture `tile` metres long: the columns of the picture run along the
/// track and its rows across the tyre.
fn push_strip(out: &mut Vec<EffectVertex>, mark: &Mark, tile: f32, alpha: u8) {
    let corners = mark.lifted(LIFT);
    let [start, end] = mark.along();
    let (from, to) = (start / tile, end / tile);
    let span = (to - from).max(f32::EPSILON);
    let color = [255, 255, 255, alpha];
    let at = |t: f32| (corners[0].lerp(corners[3], t), corners[1].lerp(corners[2], t));
    let mut u = from;
    while u < to - 1e-6 {
        let next = (u.floor() + 1.0).min(to);
        let ((l0, r0), (l1, r1)) = (at((u - from) / span), at((next - from) / span));
        push_quad(out, [l0, r0, r1, l1], [u - u.floor(), next - u.floor()], color);
        u = next;
    }
}

/// One quad: start left, start right, end right, end left, with u running from `u[0]` to `u[1]` and v across.
fn push_quad(out: &mut Vec<EffectVertex>, c: [Vec3; 4], u: [f32; 2], color: [u8; 4]) {
    let uv = [[u[0], 0.0], [u[0], 1.0], [u[1], 1.0], [u[1], 0.0]];
    for i in [0, 1, 2, 0, 2, 3] {
        out.push(EffectVertex { position: c[i].to_array(), color, uv: uv[i], detail: [0.0; 2] });
    }
}

#[cfg(test)]
#[path = "marks_variant_tests.rs"]
mod tests;
