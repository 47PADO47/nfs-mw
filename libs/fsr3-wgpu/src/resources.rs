//! The GPU resources the passes share, sized for one render and output size.

use glam::UVec2;
use wgpu::{
    Buffer, BufferDescriptor, BufferUsages, Device, Extent3d, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureView, TextureViewDescriptor, util::DeviceExt,
};

/// Exposure 1 and a log luma that tells the luma pass to take the frame's own value (see the shader).
const FRAME_INFO_RESET: [f32; 4] = [1.0, 1.0e4, 0.0, 0.0];
/// The pixels one workgroup of the luma reduction covers along an axis (8 threads of 2 pixels).
const REDUCE_TILE: u32 = 16;

/// Two textures that swap roles every frame.
pub(crate) struct Pair {
    views: [TextureView; 2],
}

impl Pair {
    /// The texture written this frame (`frame` parity 0 writes the first).
    pub fn current(&self, frame: u64) -> &TextureView {
        &self.views[(frame & 1) as usize]
    }

    /// The texture written by the previous frame.
    pub fn previous(&self, frame: u64) -> &TextureView {
        &self.views[((frame + 1) & 1) as usize]
    }
}

/// Everything sized by the render and output resolution.
pub(crate) struct Resources {
    pub render: UVec2,
    pub output: UVec2,
    pub dilated_motion_vectors: TextureView,
    pub dilated_depth: TextureView,
    pub farthest_depth: TextureView,
    pub farthest_depth_mip1: TextureView,
    pub luma_instability: TextureView,
    pub shading_change: TextureView,
    pub reactive_masks: TextureView,
    pub new_locks: TextureView,
    /// The three levels of the shading change pyramid and their sizes.
    pub shading_pyramid: [TextureView; 3],
    pub shading_pyramid_sizes: [UVec2; 3],
    pub luma: Pair,
    pub accumulation: Pair,
    pub luma_history: Pair,
    pub history: Pair,
    /// The previous frame's nearest depth per render pixel (atomic keys).
    pub reconstructed_depth: Buffer,
    /// One partial sum per workgroup of the luma reduction.
    pub partials: Buffer,
    /// `[exposure, smoothed log luma, average luma, 0]`.
    pub frame_info: Buffer,
    pub frame_info_reset: Buffer,
    /// A zero texture standing in for the optional inputs that are not given.
    pub placeholder: TextureView,
    pub bytes: u64,
}

struct Allocator<'a> {
    device: &'a Device,
    bytes: u64,
}

impl Allocator<'_> {
    fn texture(&mut self, label: &str, size: UVec2, format: TextureFormat) -> TextureView {
        let texel = match format {
            TextureFormat::R32Float => 4,
            TextureFormat::Rg32Float | TextureFormat::Rgba16Float => 8,
            _ => 16,
        };
        self.bytes += u64::from(size.x) * u64::from(size.y) * texel;
        let texture = self.device.create_texture(&TextureDescriptor {
            label: Some(label),
            size: Extent3d { width: size.x, height: size.y, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING | TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        texture.create_view(&TextureViewDescriptor::default())
    }

    fn pair(&mut self, label: &str, size: UVec2, format: TextureFormat) -> Pair {
        Pair { views: [self.texture(label, size, format), self.texture(label, size, format)] }
    }

    fn buffer(&mut self, label: &str, size: u64, usage: BufferUsages) -> Buffer {
        self.bytes += size;
        self.device.create_buffer(&BufferDescriptor { label: Some(label), size, usage, mapped_at_creation: false })
    }
}

fn div_ceil(size: UVec2, divisor: u32) -> UVec2 {
    UVec2::new(size.x.div_ceil(divisor), size.y.div_ceil(divisor))
}

fn half(size: UVec2) -> UVec2 {
    UVec2::new((size.x / 2).max(1), (size.y / 2).max(1))
}

impl Resources {
    pub fn new(device: &Device, render: UVec2, output: UVec2) -> Self {
        let mut a = Allocator { device, bytes: 0 };
        let r32 = TextureFormat::R32Float;
        let rgba16 = TextureFormat::Rgba16Float;
        let half_size = half(render);
        let level1 = half(half_size);
        let level2 = half(level1);

        let partial_groups = div_ceil(render, REDUCE_TILE);
        let pixels = u64::from(render.x) * u64::from(render.y);
        let storage = BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC;
        let frame_info_reset = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("fsr3 frame info reset"),
            contents: bytemuck::bytes_of(&FRAME_INFO_RESET),
            usage: BufferUsages::COPY_SRC,
        });

        let mut resources = Self {
            render,
            output,
            dilated_motion_vectors: a.texture("fsr3 dilated motion vectors", render, TextureFormat::Rg32Float),
            dilated_depth: a.texture("fsr3 dilated depth", render, r32),
            farthest_depth: a.texture("fsr3 farthest depth", render, r32),
            farthest_depth_mip1: a.texture("fsr3 farthest depth mip 1", half_size, r32),
            luma_instability: a.texture("fsr3 luma instability", render, r32),
            shading_change: a.texture("fsr3 shading change", half_size, r32),
            reactive_masks: a.texture("fsr3 reactive masks", render, rgba16),
            new_locks: a.texture("fsr3 new locks", output, r32),
            shading_pyramid: [
                a.texture("fsr3 shading change level 0", half_size, TextureFormat::Rg32Float),
                a.texture("fsr3 shading change level 1", level1, TextureFormat::Rg32Float),
                a.texture("fsr3 shading change level 2", level2, TextureFormat::Rg32Float),
            ],
            shading_pyramid_sizes: [half_size, level1, level2],
            luma: a.pair("fsr3 luma", render, r32),
            accumulation: a.pair("fsr3 accumulation", render, r32),
            luma_history: a.pair("fsr3 luma history", render, rgba16),
            history: a.pair("fsr3 history", output, rgba16),
            reconstructed_depth: a.buffer("fsr3 reconstructed depth", pixels * 4, storage),
            partials: a.buffer(
                "fsr3 luma partials",
                u64::from(partial_groups.x) * u64::from(partial_groups.y) * 16,
                BufferUsages::STORAGE,
            ),
            frame_info: a.buffer("fsr3 frame info", 16, storage),
            frame_info_reset,
            placeholder: a.texture("fsr3 placeholder", UVec2::ONE, r32),
            bytes: 0,
        };
        resources.bytes = a.bytes;
        resources
    }

    /// The workgroups of the luma reduction along each axis.
    pub fn luma_reduce_groups(&self) -> UVec2 {
        div_ceil(self.render, REDUCE_TILE)
    }

    /// Whether these resources fit the given sizes.
    pub fn fits(&self, render: UVec2, output: UVec2) -> bool {
        self.render == render && self.output == output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_sizes_never_collapse() {
        assert_eq!(half(UVec2::new(1920, 1080)), UVec2::new(960, 540));
        assert_eq!(half(UVec2::new(3, 2)), UVec2::new(1, 1));
        assert_eq!(half(UVec2::new(1, 1)), UVec2::new(1, 1));
    }

    #[test]
    fn pair_swaps_every_frame() {
        let toggled = |frame: u64| (frame & 1, (frame + 1) & 1);
        assert_eq!(toggled(0), (0, 1));
        assert_eq!(toggled(1), (1, 0));
        assert_eq!(toggled(2), (0, 1));
    }
}
