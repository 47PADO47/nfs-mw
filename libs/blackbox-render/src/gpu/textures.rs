//! Texture upload and release.

use super::Renderer;
use crate::{PixelFormat, TextureDesc, TextureHandle};

impl Renderer {
    /// Upload a texture. BC formats need [`Self::supports_bc`] and a block-aligned size.
    pub fn create_texture(&mut self, desc: &TextureDesc<'_>) -> TextureHandle {
        let (format, block, block_bytes) = match desc.format {
            PixelFormat::Bc1 => (wgpu::TextureFormat::Bc1RgbaUnorm, 4, 8),
            PixelFormat::Bc2 => (wgpu::TextureFormat::Bc2RgbaUnorm, 4, 16),
            PixelFormat::Bc3 => (wgpu::TextureFormat::Bc3RgbaUnorm, 4, 16),
            PixelFormat::Rgba8 => (wgpu::TextureFormat::Rgba8Unorm, 1, 4),
        };
        let size = wgpu::Extent3d { width: desc.width, height: desc.height, depth_or_array_layers: 1 };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(desc.label),
            size,
            mip_level_count: desc.mips.len().max(1) as u32,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        for (level, data) in desc.mips.iter().enumerate() {
            let w = (desc.width >> level).max(1);
            let h = (desc.height >> level).max(1);
            let (blocks_w, blocks_h) = (w.div_ceil(block), h.div_ceil(block));
            let needed = (blocks_w * blocks_h * block_bytes) as usize;
            if data.len() < needed {
                log::warn!("{}: mip {level} is {} bytes, expected {needed}", desc.label, data.len());
                break;
            }
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: level as u32,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &data[..needed],
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(blocks_w * block_bytes),
                    rows_per_image: Some(blocks_h),
                },
                size.mip_level_size(level as u32, wgpu::TextureDimension::D2).physical_size(format),
            );
        }
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.diffuse_group(&view, None, None);
        let slot = self.textures.insert(bind_group);
        self.texture_views.insert(slot, view);
        TextureHandle::from_raw(slot)
    }

    /// Free a texture. Draws that still reference it fall back to white.
    pub fn destroy_texture(&mut self, handle: TextureHandle) {
        if handle.raw() != 0 {
            self.planar_receivers.retain(|&receiver| receiver != handle.raw());
            self.textures.remove(handle.raw());
            self.texture_views.remove(&handle.raw());
            self.texture_scrolls.remove(&handle.raw());
            self.redirects.forget(handle.raw());
            if let Some(rt) = &mut self.ray_reflections {
                rt.remove_texture(handle.raw());
            }
        }
    }

    /// Draw `to` wherever `from` is used, until changed (`None` restores `from`).
    /// Animated textures switch frames this way without touching meshes.
    pub fn redirect_texture(&mut self, from: TextureHandle, to: Option<TextureHandle>) {
        match to {
            Some(to) if to != from => {
                self.redirects.insert(from.raw(), to.raw());
            }
            _ => {
                self.redirects.remove(from.raw());
            }
        }
    }

    pub fn set_texture_uv_offset(&mut self, handle: TextureHandle, offset: [f32; 2]) {
        let Some(view) = self.texture_views.get(&handle.raw()) else { return };
        if !self.texture_scrolls.contains_key(&handle.raw()) {
            let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("texture UV offset"),
                size: 16,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let projected = self
                .projected
                .as_ref()
                .filter(|_| self.planar_receivers.contains(&handle.raw()))
                .map(|capture| (capture.output(), capture.receiver_matrix));
            let group = self.diffuse_group(view, projected, Some(&buffer));
            self.texture_scrolls.insert(handle.raw(), buffer);
            if let Some(slot) = self.textures.get_mut(handle.raw()) {
                *slot = group;
            }
        }
        let words = [offset[0], offset[1], 0.0, 0.0];
        self.queue.write_buffer(&self.texture_scrolls[&handle.raw()], 0, bytemuck::cast_slice(&words));
    }
}
