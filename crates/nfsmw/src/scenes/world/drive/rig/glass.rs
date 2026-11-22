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
                .swaps_for(mask.glass)
                .with_broken_head_lights(heads)
                .with_broken_brake_lights(u32::from(mask.brakes) << 3);
            // Compare with brakes ON so a broken side is relevant even if it is OFF at this draw.
            let before = self.swaps().with_lamp_state(true, true, &self.model.textures);
            let after = swaps.clone().with_lamp_state(true, true, &self.model.textures);
            let cracked = self.cracked_window_solids(mask.glass);
            let mut affected = affected_solids(&self.model, &changed_textures(&before, &after));
            affected.extend(&cracked);
            set_lighting(renderer, appearance.shading, to_sun);
            let (parts, meshes) = upload_solids(
                renderer,
                &self.model,
                &swaps,
                &appearance.materials,
                Some((&affected, &cracked)),
                self.background_actor,
            );
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

    pub(super) fn swaps_for(&self, glass: u16) -> TextureSwaps {
        let mut hashes: Vec<_> = (0..10)
            .filter(|zone| glass & (1 << zone) != 0)
            .flat_map(nfsmw_data::car::damage::zone_windows)
            .map(|name| blackbox_hash::bstring_hash(name))
            .collect();
        if self.model.driving_lods.damage.enabled && !hashes.is_empty() {
            hashes.push(blackbox_hash::bstring_hash("REAR_DEFROSTER"));
        }
        self.swaps().with_window_hashes(&hashes)
    }

    /// Window meshes can use the same diffuse texture as opaque trim on another part.
    /// Keep their inferred replacements local to the selected panes at each detail level.
    pub(super) fn cracked_window_solids(&self, glass: u16) -> HashSet<u32> {
        let mut cracked = HashSet::new();
        for window in [0, 1, 4, 5, 6, 7] {
            let side = match window {
                4 | 6 => 1 << 2,
                5 | 7 => 1 << 3,
                _ => 0,
            };
            if glass & ((1 << window) | side) == 0 {
                continue;
            }
            let Some(suffix) = nfsmw_data::car::damage::zone_window_solid(window) else { continue };
            let prefix = suffix.trim_end_matches("_A");
            cracked.extend(self.model.solids.iter().filter_map(|(&hash, solid)| {
                let (name, lod) = solid.name.rsplit_once('_')?;
                (name.ends_with(prefix) && lod.len() == 1).then_some(hash)
            }));
        }
        cracked
    }
}
