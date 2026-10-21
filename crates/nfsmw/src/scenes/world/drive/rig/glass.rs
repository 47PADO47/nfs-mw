//! Per-actor window variants; sharing a cop model must not share its damage.
use super::*;
use std::collections::{BTreeMap, BTreeSet};

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
    pub fn prepare_glass(&self, renderer: &mut dyn RenderBackend, masks: &BTreeSet<u16>, to_sun: Vec3) {
        let mut variants = self.glass.borrow_mut();
        let dead: Vec<_> = variants.keys().filter(|m| !masks.contains(m)).copied().collect();
        for mask in dead {
            variants.remove(&mask).unwrap().release(renderer);
        }
        for &mask in masks.iter().filter(|&&m| m != 0) {
            if variants.contains_key(&mask) {
                continue;
            }
            let swaps = self.swaps_for((0..10).filter(|zone| mask & (1 << zone) != 0));
            let affected = affected_solids(&self.model, &changed_textures(&self.swaps(), &swaps));
            let appearance = self.appearance.borrow();
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
