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
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(desc.label),
            layout: &self.shared.texture_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.shared.sampler) },
            ],
        });
        TextureHandle::from_raw(self.textures.insert(bind_group))
    }

    /// Free a texture. Draws that still reference it fall back to white.
    pub fn destroy_texture(&mut self, handle: TextureHandle) {
        if handle.raw() != 0 {
            self.textures.remove(handle.raw());
            self.redirects.retain(|&from, &mut to| from != handle.raw() && to != handle.raw());
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
                self.redirects.remove(&from.raw());
            }
        }
    }
}
