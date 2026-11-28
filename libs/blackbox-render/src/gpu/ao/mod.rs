//! Spatial GTAO from shared depth/normals. An R8 mask attenuates ambient lighting before fog.

#[cfg(test)]
mod benchmark;
mod bindings;
#[cfg(test)]
#[cfg(test)]
mod contact_tests;
#[cfg(test)]
mod convergence_tests;
mod depth;
#[cfg(test)]
mod depth_tests;
#[cfg(test)]
mod directional_tests;
mod frame;
#[cfg(test)]
mod integration_tests;
#[cfg(test)]
mod jitter_tests;
#[cfg(test)]
mod method_tests;
#[cfg(test)]
mod occluder_tests;
mod pipelines;
#[cfg(test)]
mod pyramid_tests;
#[cfg(test)]
mod resolution_tests;
#[cfg(test)]
mod sampling_tests;
#[cfg(test)]
mod surface_tests;
mod temporal;
#[cfg(test)]
mod temporal_tests;
#[cfg(test)]
mod tests;

use blackbox_gfx::{AoMethod, AoSettings, FrameParams, Projection};
use pipelines::Pipelines;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    sizes: [f32; 4],
    projection: [f32; 4],
    effect: [f32; 4],
    sampling: [u32; 4],
    fog: [f32; 4],
    view: [f32; 16],
    /// XeGTAO thin occluder compensation, the shading normal guard, the bent-normal flag, the static depth switch.
    tuning: [f32; 4],
}

/// A number from the environment, read once, for tuning and comparisons.
fn env_override(name: &'static str, cell: &'static std::sync::OnceLock<Option<f32>>) -> Option<f32> {
    *cell.get_or_init(|| std::env::var(name).ok().and_then(|v| v.trim().parse().ok()).filter(|v: &f32| v.is_finite()))
}

/// `BLACKBOX_AO_THIN_OCCLUDERS` overrides the thin occluder compensation and `BLACKBOX_AO_NORMAL_GUARD` the normal
/// guard (each 0 to 1; 0 turns it off). The third value is 1 when XeGTAO also searches the static depth.
fn tuning(settings: &AoSettings, static_depth: bool) -> [f32; 4] {
    static THIN: std::sync::OnceLock<Option<f32>> = std::sync::OnceLock::new();
    static GUARD: std::sync::OnceLock<Option<f32>> = std::sync::OnceLock::new();
    let thin = env_override("BLACKBOX_AO_THIN_OCCLUDERS", &THIN).map_or(settings.thin_occluders, |v| v.clamp(0.0, 1.0));
    let guard = env_override("BLACKBOX_AO_NORMAL_GUARD", &GUARD).map_or(settings.normal_guard, |v| v.clamp(0.0, 1.0));
    [
        thin,
        guard,
        f32::from(u8::from(settings.directional())),
        f32::from(u8::from(static_depth && settings.method.is_xe())),
    ]
}

struct Images {
    size: (u32, u32),
    raw: wgpu::TextureView,
    filtered: wgpu::TextureView,
}

impl Images {
    fn new(device: &wgpu::Device, size: (u32, u32)) -> Self {
        let image = |label| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba16Float,
                    usage: wgpu::TextureUsages::STORAGE_BINDING
                        | wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::COPY_SRC,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default())
        };
        Self { size, raw: image("AO raw"), filtered: image("AO filtered") }
    }
}

pub(super) struct AmbientOcclusion {
    pipelines: Pipelines,
    params: wgpu::Buffer,
    images: Option<Images>,
    inputs: Option<bindings::Inputs>,
    depth: depth::DepthPyramid,
    temporal: Option<temporal::History>,
    phase: u32,
    /// Frames drawn under a temporal upscaler, which steps the spatial methods' sample phases.
    spin: u32,
    fallback_depth: wgpu::TextureView,
    pub mask: wgpu::TextureView,
    pub size: (u32, u32),
}

pub(super) struct Draw<'a> {
    pub motion: Option<&'a crate::gpu::temporal::EffectMotion>,
    pub target: &'a wgpu::TextureView,
    pub depth: &'a wgpu::TextureView,
    pub size: (u32, u32),
    pub normals: &'a wgpu::TextureView,
    pub frame: &'a FrameParams,
    pub settings: AoSettings,
    /// The frame is jittered for a temporal upscaler, which accumulates what it is given over frames.
    pub jittered: bool,
    /// The depth of the static scene alone (reverse-Z) and the mip chain of tiles a dynamic model covers, when
    /// dynamic models cover part of `depth`.
    pub static_depth: Option<(&'a wgpu::TextureView, &'a wgpu::TextureView)>,
}

impl AmbientOcclusion {
    pub(super) fn new(device: &wgpu::Device, size: (u32, u32), cache: Option<&wgpu::PipelineCache>) -> Self {
        Self {
            pipelines: Pipelines::new(device, cache),
            params: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("AO params"),
                size: std::mem::size_of::<Params>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            images: None,
            temporal: None,
            phase: 0,
            spin: 0,
            inputs: None,
            depth: depth::DepthPyramid::new(device, (1, 1)),
            fallback_depth: depth::DepthPyramid::new(device, (1, 1)).view,
            mask: super::surface::mask(device, size, "ambient visibility"),
            size,
        }
    }

    pub(super) fn encode(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        draw: Draw<'_>,
    ) {
        let Projection::PerspectiveInfiniteReverse { fov_y, aspect, near } = draw.frame.projection else { return };
        let settings = draw.settings.sanitized();
        if !settings.active() {
            return;
        }
        let (mut divisor, slices, steps) = match settings.method {
            // Spatial output cannot rely on TAA to hide sparse depth-search strata. Temporal
            // AO keeps the cheaper estimator and accumulates that coverage across frames.
            AoMethod::XeGtao => match settings.quality {
                blackbox_gfx::AoQuality::Off => (1, 0, 0),
                blackbox_gfx::AoQuality::Low => (2, 2, 6),
                blackbox_gfx::AoQuality::Medium => (2, 3, 6),
                // Half resolution: the composite pass upsamples with a depth and normal aware bilateral
                // filter, so High keeps its slice and step budget at a quarter of the pixel work. There is no
                // quality above High to keep at full resolution.
                blackbox_gfx::AoQuality::High => (2, 4, 8),
            },
            AoMethod::TemporalXeGtao => match settings.quality {
                blackbox_gfx::AoQuality::Off => (1, 0, 0),
                blackbox_gfx::AoQuality::Low => (2, 2, 3),
                blackbox_gfx::AoQuality::Medium => (2, 3, 4),
                blackbox_gfx::AoQuality::High => (1, 4, 4),
            },
            _ => settings.quality.sampling(),
        };
        if settings.method.temporal() {
            divisor = 2;
            if draw.motion.is_none_or(|motion| motion.reset) || draw.frame.camera_cut {
                self.phase = 0;
            }
        }
        let size = (draw.size.0.div_ceil(divisor).max(1), draw.size.1.div_ceil(divisor).max(1));
        if self.images.as_ref().is_none_or(|i| i.size != size) {
            self.images = Some(Images::new(device, size));
        }
        let images = self.images.as_ref().expect("AO images exist");
        let tan_y = (fov_y * 0.5).tan();
        self.spin = match draw.jittered && !settings.method.temporal() && animated_noise() {
            true => self.spin % 64 + 1,
            false => 0,
        };
        let params = Params {
            sizes: [draw.size.0 as f32, draw.size.1 as f32, size.0 as f32, size.1 as f32],
            projection: [tan_y * aspect, tan_y, near, f32::from(u8::from(settings.depth_mips))],
            effect: [settings.radius, settings.intensity, 40.0, 100.0],
            sampling: [divisor, slices, steps, u32::from(settings.compare)],
            fog: [
                self.spin as f32,
                0.0,
                f32::from(u8::from(settings.method.is_xe())),
                match settings.method.temporal() {
                    true => self.phase as f32,
                    false => 0.0,
                },
            ],
            view: draw.frame.view.to_cols_array(),
            tuning: tuning(&settings, draw.static_depth.is_some()),
        };
        queue.write_buffer(&self.params, 0, bytemuck::bytes_of(&params));
        let depth_size = match settings.depth_mips {
            true => draw.size,
            false => (1, 1),
        };
        let xe = settings.method.is_xe();
        if self.depth.size != depth_size || self.depth.xe != xe {
            self.depth = depth::DepthPyramid::with_method(device, depth_size, xe);
        }
        if settings.depth_mips {
            self.depth.encode(device, encoder, &self.pipelines, &self.params, draw.depth, &self.fallback_depth);
        }
        let views = [
            draw.depth,
            draw.normals,
            &images.raw,
            &images.filtered,
            &self.depth.view,
            draw.static_depth.map_or(&self.fallback_depth, |(depth, _)| depth),
            draw.static_depth.map_or(&self.fallback_depth, |(_, tiles)| tiles),
        ];
        if self.inputs.as_ref().is_none_or(|inputs| !inputs.matches(views)) {
            self.inputs = Some(bindings::Inputs::new(device, &self.pipelines, &self.params, views));
        }
        let inputs = &self.inputs.as_ref().expect("AO inputs exist").groups;
        let composite = &self.pipelines.resolve[usize::from(settings.directional())];
        let generate = match settings.method {
            AoMethod::Gtao => &self.pipelines.generate,
            AoMethod::Ssao => &self.pipelines.ssao,
            AoMethod::XeGtao | AoMethod::TemporalXeGtao => &self.pipelines.xegtao,
        };
        let filter = match settings.method {
            AoMethod::XeGtao | AoMethod::TemporalXeGtao => &self.pipelines.xe_filter,
            _ => &self.pipelines.filter,
        };
        let temporal_passes = [
            ("AO estimate", generate, &inputs[0]),
            ("AO spatial denoise 1", &self.pipelines.xe_intermediate, &inputs[1]),
            ("AO spatial denoise 2", &self.pipelines.xe_intermediate, &inputs[3]),
            ("AO spatial denoise final", filter, &inputs[1]),
        ];
        let passes: &[(&str, &wgpu::ComputePipeline, &wgpu::BindGroup)] = match settings.method {
            AoMethod::XeGtao | AoMethod::TemporalXeGtao => &[
                ("AO estimate", generate, &inputs[0]),
                ("AO spatial denoise 1", &self.pipelines.xe_intermediate, &inputs[1]),
                ("AO spatial denoise 2", &self.pipelines.xe_intermediate, &inputs[3]),
                ("AO spatial denoise 3", &self.pipelines.xe_intermediate, &inputs[1]),
                ("AO spatial denoise 4", &self.pipelines.xe_intermediate, &inputs[3]),
                ("AO spatial denoise final", filter, &inputs[1]),
            ],
            _ => &[("AO estimate", generate, &inputs[0]), ("AO spatial denoise", filter, &inputs[1])],
        };
        let passes = match settings.method.temporal() {
            true => &temporal_passes[..],
            false => passes,
        };
        for &(label, pipeline, inputs) in passes {
            let mut pass =
                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some(label), timestamp_writes: None });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, inputs, &[]);
            pass.dispatch_workgroups(size.0.div_ceil(8), size.1.div_ceil(8), 1);
        }
        let temporal_group = match settings.method.temporal() {
            true => {
                if self.temporal.as_ref().is_none_or(|h| !h.matches(size)) {
                    self.temporal = Some(temporal::History::new(device, size, settings));
                }
                let output =
                    self.temporal.as_mut().unwrap().encode(device, encoder, &self.params, &images.filtered, &draw);
                self.phase = self.phase.wrapping_add(1) % 64;
                Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("temporal AO resolve"),
                    layout: &composite.get_bind_group_layout(0),
                    entries: &[
                        wgpu::BindGroupEntry { binding: 0, resource: self.params.as_entire_binding() },
                        wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(draw.depth) },
                        wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&output) },
                        wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(draw.normals) },
                    ],
                }))
            }
            false => {
                self.temporal = None;
                self.phase = 0;
                None
            }
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("AO ambient mask"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: draw.target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { a: 0.0, ..wgpu::Color::WHITE }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(composite);
        pass.set_bind_group(0, temporal_group.as_ref().unwrap_or(&inputs[2]), &[]);
        pass.draw(0..3, 0..1);
    }
}

/// Whether the spatial methods step their sample phases each frame under a temporal upscaler.
/// `BLACKBOX_AO_ANIMATED_NOISE=0` keeps them fixed, for comparing the two.
fn animated_noise() -> bool {
    static ANIMATED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ANIMATED.get_or_init(|| std::env::var("BLACKBOX_AO_ANIMATED_NOISE").as_deref() != Ok("0"))
}

pub(crate) fn sources() -> [(&'static str, String); 6] {
    let common = include_str!("../../shaders/ao_common.wgsl");
    [
        ("AO horizons", include_str!("../../shaders/ao_horizon.wgsl")),
        ("AO filter", include_str!("../../shaders/ao_filter.wgsl")),
        ("AO composite", include_str!("../../shaders/ao_composite.wgsl")),
        ("AO depth prefilter", include_str!("../../shaders/ao_depth.wgsl")),
        ("XeGTAO edge denoise", include_str!("../../shaders/ao_xe_filter.wgsl")),
        ("AO temporal", include_str!("../../shaders/ao_temporal.wgsl")),
    ]
    .map(|(name, source)| {
        let extra = match name {
            "AO horizons" => {
                concat!(include_str!("../../shaders/ao_ssao.wgsl"), "\n", include_str!("../../shaders/ao_xegtao.wgsl"))
            }
            _ => "",
        };
        (name, format!("{common}\n{source}\n{extra}"))
    })
}
