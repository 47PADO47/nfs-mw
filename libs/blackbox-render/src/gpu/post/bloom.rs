//! The bloom pass (`shaders/bloom.wgsl`): bright areas go down a chain of half-size images and back
//! up, and the result is added to the scene.

use super::filter::{Blend, Draw, Filter, Params, bloom_wgsl};
use super::{PassContext, PassIo, PostPass};
use crate::PostSettings;

/// The most half-size images in the chain.
const MAX_LEVELS: usize = 6;
/// The soft knee starts this share of the threshold below it.
const KNEE: f32 = 0.5;
/// Scales the user's intensity: the up-sampled levels add up, so 1.0 would be much too strong.
const INTENSITY_SCALE: f32 = 0.25;

/// The sizes of the half-size images for a scene image of `size`: each is half the previous one
/// (at least one pixel), starting at half the scene, ending at [`MAX_LEVELS`] images or when an
/// axis gets down to four pixels.
pub(super) fn mip_sizes(size: (u32, u32)) -> Vec<(u32, u32)> {
    let mut sizes = Vec::new();
    let mut current = (size.0.max(1), size.1.max(1));
    while sizes.len() < MAX_LEVELS {
        let next = ((current.0 / 2).max(1), (current.1 / 2).max(1));
        if next == current {
            break;
        }
        sizes.push(next);
        current = next;
        if current.0.min(current.1) <= 4 {
            break;
        }
    }
    sizes
}

/// The parameter block: `a` = (threshold, knee, intensity).
pub(super) fn params(settings: &PostSettings) -> Params {
    let threshold = settings.bloom_threshold;
    Params { a: [threshold, threshold * KNEE, settings.bloom_intensity * INTENSITY_SCALE, 0.0], b: [0.0; 4] }
}

struct Level {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
}

/// The half-size images, for one scene size and format.
struct Chain {
    scene_size: (u32, u32),
    format: wgpu::TextureFormat,
    levels: Vec<Level>,
}

impl Chain {
    fn new(device: &wgpu::Device, scene_size: (u32, u32), format: wgpu::TextureFormat) -> Self {
        let levels = mip_sizes(scene_size)
            .into_iter()
            .map(|(width, height)| {
                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("bloom level"),
                    size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                });
                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                Level { _texture: texture, view }
            })
            .collect();
        Self { scene_size, format, levels }
    }
}

pub(super) struct BloomPass {
    filter: Filter,
    params: Params,
    chain: Option<Chain>,
}

impl BloomPass {
    pub(super) fn new(device: &wgpu::Device, settings: &PostSettings) -> Self {
        Self { filter: Filter::new(device, "bloom", &bloom_wgsl()), params: params(settings), chain: None }
    }
}

impl PostPass for BloomPass {
    fn name(&self) -> &'static str {
        "bloom"
    }

    fn encode(&mut self, ctx: &PassContext<'_>, io: &PassIo<'_>, encoder: &mut wgpu::CommandEncoder) {
        let format = ctx.scene.color_format;
        let stale = self.chain.as_ref().is_none_or(|c| c.scene_size != io.input_size || c.format != format);
        if stale {
            self.chain = Some(Chain::new(ctx.device, io.input_size, format));
        }
        let Self { filter, params, chain } = self;
        let Some(chain) = chain.as_ref() else { return };
        filter.set_params(ctx.queue, params);
        let levels = &chain.levels;
        let mut draw = |entry, src, target: &wgpu::TextureView, blend| {
            let draw = Draw { entry, src, src2: src, target, format, blend };
            filter.draw(ctx.device, encoder, &draw);
        };
        match levels.first() {
            Some(first) => draw("fs_prefilter", io.input, &first.view, Blend::Replace),
            None => return,
        }
        for pair in levels.windows(2) {
            draw("fs_down", &pair[0].view, &pair[1].view, Blend::Replace);
        }
        for pair in levels.windows(2).rev() {
            draw("fs_up", &pair[1].view, &pair[0].view, Blend::Add);
        }
        let composite = Draw {
            entry: "fs_composite",
            src: io.input,
            src2: &levels[0].view,
            target: io.output,
            format: io.output_format,
            blend: Blend::Replace,
        };
        filter.draw(ctx.device, encoder, &composite);
    }
}
