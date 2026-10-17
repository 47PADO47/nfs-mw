//! Game-supplied display-space colour grading, before the HUD.

use super::filter::{Blend, Draw, Filter, Params, filter_source};
use super::treatment_composite::Composite;
use super::{PassContext, PassIo, PostPass, Scratch};
use blackbox_gfx::{ColourTreatment, PostSettings, RgbaImage};

pub(super) fn filter_wgsl() -> String {
    filter_source(include_str!("../../shaders/treatment_filter.wgsl"))
}

/// The scene profile persists across settings changes; its texture view owns the upload.
pub(super) struct Profile {
    pub settings: ColourTreatment,
    pub gain: wgpu::TextureView,
    /// Adapter-selected renderable/filterable format, including the non-HDR fallback.
    pub luma_format: wgpu::TextureFormat,
}

pub(super) fn gain_texture(device: &wgpu::Device, queue: &wgpu::Queue, image: Option<&RgbaImage>) -> wgpu::TextureView {
    let (width, height, bytes) = image
        .filter(|i| i.width > 0 && i.height > 0 && i.rgba.len() as u64 == u64::from(i.width) * u64::from(i.height) * 4)
        .map_or((1, 1, &[255, 0, 255, 255][..]), |i| (i.width, i.height, i.rgba.as_slice()));
    let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("screen gain map"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
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
        bytes,
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(width * 4), rows_per_image: Some(height) },
        size,
    );
    texture.create_view(&Default::default())
}

struct Targets {
    size: (u32, u32),
    luma: [Scratch; 2],
}

pub(super) struct TreatmentPass {
    filter: Filter,
    params: Params,
    composite: Composite,
    gain: wgpu::TextureView,
    luma_format: wgpu::TextureFormat,
    targets: Option<Targets>,
}

impl TreatmentPass {
    pub(super) fn new(device: &wgpu::Device, settings: &PostSettings, profile: &Profile) -> Self {
        let mut params = Params::default();
        params.b[..3].copy_from_slice(&profile.settings.luminance);
        Self {
            filter: Filter::new(device, "scene treatment blur", &filter_wgsl()),
            params,
            composite: Composite::new(device, settings, Some(&profile.settings)),
            gain: profile.gain.clone(),
            luma_format: profile.luma_format,
            targets: None,
        }
    }

    fn blur(
        filter: &mut Filter,
        ctx: &PassContext<'_>,
        source: &wgpu::TextureView,
        pair: &[Scratch; 2],
        entry: &'static str,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let mut draw = |entry, src, target| {
            filter.draw(
                ctx.device,
                encoder,
                &Draw { entry, src, src2: src, target, format: pair[0].format, blend: Blend::Replace },
            )
        };
        draw(entry, source, &pair[0].view);
        for _ in 0..2 {
            draw("fs_horizontal", &pair[0].view, &pair[1].view);
            draw("fs_vertical", &pair[1].view, &pair[0].view);
        }
    }
}

impl PostPass for TreatmentPass {
    fn set_display(&mut self, mapping: Option<super::display::Mapping>) {
        self.composite.set_hdr(mapping.is_some());
    }
    fn name(&self) -> &'static str {
        "scene treatment"
    }

    fn encode(&mut self, ctx: &PassContext<'_>, io: &PassIo<'_>, encoder: &mut wgpu::CommandEncoder) {
        if self.targets.as_ref().is_none_or(|t| t.size != io.input_size) {
            let quarter = ((io.input_size.0 / 4).max(1), (io.input_size.1 / 4).max(1));
            let pair = || std::array::from_fn(|_| Scratch::new(ctx.device, quarter, self.luma_format));
            self.targets = Some(Targets { size: io.input_size, luma: pair() });
        }
        let Some(targets) = &self.targets else { return };
        self.filter.set_params(ctx.queue, &self.params);
        Self::blur(&mut self.filter, ctx, io.input, &targets.luma, "fs_luma", encoder);
        self.composite.draw(ctx, io, [&targets.luma[0].view, &self.gain], encoder);
    }
}
