//! Per-actor window variants; sharing a cop model must not share its damage.
use super::*;
use std::collections::BTreeMap;
#[cfg(test)]
use std::collections::BTreeSet;

/// Only live window/brake combinations need an affected-solid appearance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct SurfaceDamage {
    glass: u16,
    brakes: u8,
    heads: u8,
}

impl SurfaceDamage {
    pub fn new(glass: u16, broken_lights: u32) -> Self {
        Self { glass: glass & 0x3ff, brakes: ((broken_lights >> 3) & 7) as u8, heads: (broken_lights & 3) as u8 }
    }
}

pub(super) struct GlassVariant {
    pub parts: BTreeMap<(usize, usize, bool), Part>,
    meshes: Vec<(u32, MeshHandle)>,
}

impl GlassVariant {
    fn release(self, renderer: &mut dyn RenderBackend) {
        for (_, mesh) in self.meshes {
            renderer.destroy_mesh(mesh);
        }
    }
}

impl CarRig {
    #[cfg(test)]
    pub fn prepare_glass(&self, renderer: &mut dyn RenderBackend, masks: &BTreeSet<u16>, to_sun: Vec3) {
        self.prepare_surfaces(renderer, &masks.iter().map(|&glass| SurfaceDamage::new(glass, 0)).collect(), to_sun);
    }

    #[cfg(test)]
    pub fn prepare_surfaces(&self, renderer: &mut dyn RenderBackend, masks: &BTreeSet<SurfaceDamage>, to_sun: Vec3) {
        let masks: Vec<_> = masks.iter().copied().collect();
        self.prepare_surface_slice(renderer, &masks, to_sun);
    }

    /// [`Self::prepare_surfaces`] for states in a slice, ascending and without repeats (what iterating the set gives),
    /// so a caller that keeps its buffer from frame to frame builds no set.
    pub fn prepare_surface_slice(&self, renderer: &mut dyn RenderBackend, masks: &[SurfaceDamage], to_sun: Vec3) {
        let mut variants = self.glass.borrow_mut();
        let dead: Vec<_> = variants.keys().filter(|m| !masks.contains(m)).copied().collect();
        for mask in dead {
            variants.remove(&mask).unwrap().release(renderer);
        }
        for &mask in masks.iter().filter(|&&m| m != SurfaceDamage::default()) {
            if variants.contains_key(&mask) {
                continue;
            }
            let appearance = self.appearance.borrow();
            let heads = u32::from(mask.heads) * u32::from(appearance.materials.enhanced_lamps());
            let swaps = self
                .swaps_for((0..10).filter(|zone| mask.glass & (1 << zone) != 0))
                .with_broken_head_lights(heads)
                .with_broken_brake_lights(u32::from(mask.brakes) << 3);
            // Compare with brakes ON so a broken side is relevant even if it is OFF at this draw.
            let before = self.swaps().with_lamp_state(true, true, &self.model.textures);
            let after = swaps.clone().with_lamp_state(true, true, &self.model.textures);
            let affected = affected_solids(&self.model, &changed_textures(&before, &after));
            set_lighting(renderer, appearance.shading, to_sun);
            let (parts, meshes) = upload_solids(renderer, &self.model, &swaps, &appearance.materials, Some(&affected));
            variants.insert(
                mask,
                GlassVariant { parts: parts.into_iter().map(|p| ((p.key, p.lod, p.damaged), p)).collect(), meshes },
            );
        }
    }

    pub(super) fn clear_glass(&self, renderer: &mut dyn RenderBackend) {
        for (_, variant) in std::mem::take(&mut *self.glass.borrow_mut()) {
            variant.release(renderer);
        }
    }

    pub(super) fn swaps_for(&self, zones: impl IntoIterator<Item = usize>) -> TextureSwaps {
        let mut hashes = Vec::new();
        for zone in zones {
            hashes.extend(nfsmw_data::car::damage::zone_windows(zone).iter().map(|w| blackbox_hash::bstring_hash(w)));
            let windows: &[usize] = match zone {
                2 => &[4, 6],
                3 => &[5, 7],
                _ => &[],
            };
            for window in std::iter::once(zone).chain(windows.iter().copied()) {
                let Some(suffix) = nfsmw_data::car::damage::zone_window_solid(window) else { continue };
                let prefix = suffix.trim_end_matches("_A");
                for solid in self.model.solids.values().filter(|s| {
                    s.name.rsplit_once('_').is_some_and(|(name, lod)| name.ends_with(prefix) && lod.len() == 1)
                }) {
                    hashes.extend(solid.groups.iter().filter_map(|g| g.diffuse_texture(solid)));
                }
            }
        }
        if self.model.driving_lods.damage.enabled && !hashes.is_empty() {
            hashes.push(blackbox_hash::bstring_hash("REAR_DEFROSTER"));
        }
        self.model.swaps.clone().with_window_hashes(&hashes)
    }
}
