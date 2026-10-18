//! Spatial GTAO from shared depth/normals. An R8 mask attenuates ambient lighting before fog.

#[cfg(test)]
mod benchmark;
mod bindings;
#[cfg(test)]
mod contact_tests;
mod depth;
#[cfg(test)]
mod depth_tests;
mod frame;
#[cfg(test)]
mod integration_tests;
#[cfg(test)]
mod method_tests;
mod pipelines;
#[cfg(test)]
mod pyramid_tests;
#[cfg(test)]
mod resolution_tests;
#[cfg(test)]
mod surface_tests;
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
                    usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
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
    fallback_depth: wgpu::TextureView,
    pub mask: wgpu::TextureView,
    pub size: (u32, u32),
}

pub(super) struct Draw<'a> {
    pub target: &'a wgpu::TextureView,
    pub depth: &'a wgpu::TextureView,
    pub size: (u32, u32),
    pub normals: &'a wgpu::TextureView,
    pub frame: &'a FrameParams,
    pub settings: AoSettings,
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
        let (divisor, slices, steps) = match settings.method {
            AoMethod::XeGtao => match settings.quality {
                blackbox_gfx::AoQuality::Off => (1, 0, 0),
                blackbox_gfx::AoQuality::Low => (2, 2, 3),
                blackbox_gfx::AoQuality::Medium => (2, 3, 4),
                blackbox_gfx::AoQuality::High => (1, 4, 4),
            },
            _ => settings.quality.sampling(),
        };
        let size = (draw.size.0.div_ceil(divisor).max(1), draw.size.1.div_ceil(divisor).max(1));
        if self.images.as_ref().is_none_or(|i| i.size != size) {
            self.images = Some(Images::new(device, size));
        }
        let images = self.images.as_ref().expect("AO images exist");
        let tan_y = (fov_y * 0.5).tan();
        let [fog_start, fog_end] = draw.frame.fog_range();
        let params = Params {
            sizes: [draw.size.0 as f32, draw.size.1 as f32, size.0 as f32, size.1 as f32],
            projection: [tan_y * aspect, tan_y, near, f32::from(u8::from(settings.depth_mips))],
            effect: [settings.radius, settings.intensity, 40.0, 100.0],
            sampling: [divisor, slices, steps, u32::from(settings.compare)],
            fog: [fog_start, fog_end, f32::from(u8::from(settings.method == AoMethod::XeGtao)), 0.0],
            view: draw.frame.view.to_cols_array(),
        };
        queue.write_buffer(&self.params, 0, bytemuck::bytes_of(&params));
        let depth_size = match settings.depth_mips {
            true => draw.size,
            false => (1, 1),
        };
        let xe = settings.method == AoMethod::XeGtao;
        if self.depth.size != depth_size || self.depth.xe != xe {
            self.depth = depth::DepthPyramid::with_method(device, depth_size, xe);
        }
        if settings.depth_mips {
            self.depth.encode(device, encoder, &self.pipelines, &self.params, draw.depth, &self.fallback_depth);
        }
        let views = [draw.depth, draw.normals, &images.raw, &images.filtered, &self.depth.view];
        if self.inputs.as_ref().is_none_or(|inputs| !inputs.matches(views)) {
            self.inputs = Some(bindings::Inputs::new(device, &self.pipelines, &self.params, views));
        }
        let inputs = &self.inputs.as_ref().expect("AO inputs exist").groups;
        let composite = &self.pipelines.resolve;
        let generate = match settings.method {
            AoMethod::Gtao => &self.pipelines.generate,
            AoMethod::Ssao => &self.pipelines.ssao,
            AoMethod::XeGtao => &self.pipelines.xegtao,
        };
        let filter = match settings.method {
            AoMethod::XeGtao => &self.pipelines.xe_filter,
            _ => &self.pipelines.filter,
        };
        let passes: &[(&str, &wgpu::ComputePipeline, &wgpu::BindGroup)] = match settings.method {
            AoMethod::XeGtao => &[
                ("AO estimate", generate, &inputs[0]),
                ("AO spatial denoise 1", &self.pipelines.xe_intermediate, &inputs[1]),
                ("AO spatial denoise 2", &self.pipelines.xe_intermediate, &inputs[3]),
                ("AO spatial denoise 3", &self.pipelines.xe_intermediate, &inputs[1]),
                ("AO spatial denoise 4", &self.pipelines.xe_intermediate, &inputs[3]),
                ("AO spatial denoise final", filter, &inputs[1]),
            ],
            _ => &[("AO estimate", generate, &inputs[0]), ("AO spatial denoise", filter, &inputs[1])],
        };
        for &(label, pipeline, inputs) in passes {
            let mut pass =
                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some(label), timestamp_writes: None });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, inputs, &[]);
            pass.dispatch_workgroups(size.0.div_ceil(8), size.1.div_ceil(8), 1);
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("AO ambient mask"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: draw.target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::WHITE), store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(composite);
        pass.set_bind_group(0, &inputs[2], &[]);
        pass.draw(0..3, 0..1);
    }
}

pub(crate) fn sources() -> [(&'static str, String); 5] {
    let common = include_str!("../../shaders/ao_common.wgsl");
    [
        ("AO horizons", include_str!("../../shaders/ao_horizon.wgsl")),
        ("AO filter", include_str!("../../shaders/ao_filter.wgsl")),
        ("AO composite", include_str!("../../shaders/ao_composite.wgsl")),
        ("AO depth prefilter", include_str!("../../shaders/ao_depth.wgsl")),
        ("XeGTAO edge denoise", include_str!("../../shaders/ao_xe_filter.wgsl")),
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
