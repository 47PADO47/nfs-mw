// Ported from the smaa crate 0.21.0 (src/lib.rs, the Resources type: the area and search tables and the sampler):
// Copyright (c) 2018 Jonathan Behrens
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::area::{AREA_SIZE, area_texture_cached};
use crate::search::{SEARCH_SIZE, search_texture};

/// The precomputed lookup tables and the linear clamp sampler. Uploaded once per [`crate::SmaaTarget`].
pub(crate) struct Resources {
    /// The area table, RG8, indexed by edge pattern and distance.
    pub(crate) area: wgpu::TextureView,
    /// The search table, R8, packed: the walked distances.
    pub(crate) search: wgpu::TextureView,
    pub(crate) sampler: wgpu::Sampler,
}

impl Resources {
    pub(crate) fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let area = upload_table(
            device,
            queue,
            "smaa.texture.area",
            AREA_SIZE,
            wgpu::TextureFormat::Rg8Unorm,
            2,
            area_texture_cached(),
        );
        let search_bytes = search_texture();
        let search = upload_table(
            device,
            queue,
            "smaa.texture.search",
            SEARCH_SIZE,
            wgpu::TextureFormat::R8Unorm,
            1,
            &search_bytes,
        );
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("smaa.sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        Self { area, search, sampler }
    }
}

/// Create a single-mip texture of `size` in `format` and write `data` (layer-major rows, `bytes_per_texel` each) into it.
fn upload_table(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    (width, height): (u32, u32),
    format: wgpu::TextureFormat,
    bytes_per_texel: u32,
    data: &[u8],
) -> wgpu::TextureView {
    assert_eq!(data.len(), (width * height * bytes_per_texel) as usize, "{label}: the table does not match its size");
    let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * bytes_per_texel),
            rows_per_image: Some(height),
        },
        size,
    );
    texture.create_view(&wgpu::TextureViewDescriptor { label: Some(label), ..Default::default() })
}
