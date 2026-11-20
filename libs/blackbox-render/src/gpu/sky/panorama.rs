//! The panoramic sky: the pictures a sky variant gives the enhanced sky (`docs/modding/sky-variants.md`) and the group each
//! material binds while they are drawn.

use super::{Material, Sky, material_group_parts};
use crate::Renderer;

/// The views of the panoramic sky's pictures.
pub(super) struct Panorama {
    pub(super) pictures: blackbox_gfx::SkyPanorama,
    pub(super) domes: [wgpu::TextureView; 2],
    pub(super) moon: wgpu::TextureView,
    pub(super) has_moon: bool,
}

impl Renderer {
    /// Set the pictures of the panoramic sky, or `None` for the procedural clouds alone. The same pictures again do nothing.
    pub fn set_sky_panorama(&mut self, pictures: Option<blackbox_gfx::SkyPanorama>) {
        let Some(sky) = &self.sky else { return };
        if sky.panorama.as_ref().map(|p| p.pictures) == pictures {
            return;
        }
        self.flush_captures();
        let Some(sky) = &mut self.sky else { return };
        sky.panorama = pictures.and_then(|pictures| {
            let a = self.texture_views.get(&pictures.a.raw())?.clone();
            let b = self.texture_views.get(&pictures.b.raw())?.clone();
            let moon = pictures.moon.and_then(|moon| self.texture_views.get(&moon.raw()).cloned());
            let has_moon = moon.is_some();
            Some(Panorama { pictures, domes: [a, b], moon: moon.unwrap_or_else(|| sky.blank.clone()), has_moon })
        });
        sky.panorama_epoch = sky.panorama_epoch.wrapping_add(1);
        let groups: Vec<_> =
            sky.materials.iter_indexed().map(|(_, m)| sky.panorama_group(&self.device, &m.uniform)).collect();
        for (material, group) in sky.materials.iter_mut().zip(groups) {
            material.pano_group = group;
        }
        sky.params_dirty = true;
    }
}

impl Sky {
    /// The panorama as the shader draws it now: only with the procedural sky on and pictures set.
    pub(super) fn panorama_state(&self) -> Option<blackbox_gfx::SkyPanoramaState> {
        self.panorama.as_ref()?;
        self.state.enhanced.then_some(self.state.panorama).flatten()
    }

    /// The part of the cloud map's key that the panorama decides: which pictures, the blend and the turns.
    pub(super) fn panorama_key(&self) -> [f32; 4] {
        match self.panorama_state() {
            Some(p) => [self.panorama_epoch as f32, p.blend, p.rotation[0], p.rotation[1]],
            None => [-1.0, 0.0, 0.0, 0.0],
        }
    }

    /// The material's group with the panorama's domes and moon in place of the authored pictures.
    pub(super) fn panorama_group(&self, device: &wgpu::Device, uniform: &wgpu::Buffer) -> Option<wgpu::BindGroup> {
        let panorama = self.panorama.as_ref()?;
        let moon = match panorama.has_moon {
            true => &panorama.moon,
            false => &self.blank,
        };
        Some(material_group_parts(
            device,
            &self.material_layout,
            &self.sampler,
            [&self.noise, &self.detail, &self.weather],
            &self.noise_sampler,
            &panorama.domes,
            uniform,
            moon,
        ))
    }

    /// The group the material's pass binds: the panorama's while it is drawn.
    pub(super) fn group<'a>(&self, material: &'a Material) -> &'a wgpu::BindGroup {
        match (&material.pano_group, self.panorama_state()) {
            (Some(group), Some(_)) => group,
            _ => &material.bind_group,
        }
    }
}
