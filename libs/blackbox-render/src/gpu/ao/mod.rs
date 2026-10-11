//! Isolated experimental GTAO pass: depth-only sampling, spatial denoise, then multiply into colour.
//! Nothing is allocated until an enabled perspective frame is rendered. No history or extra geometry.

#[cfg(test)]
mod benchmark;
#[cfg(test)]
mod contact_tests;
mod pipelines;
#[cfg(test)]
mod tests;

use blackbox_gfx::{AoSettings, FrameParams, Projection};
use pipelines::Pipelines;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    sizes: [f32; 4],
    projection: [f32; 4],
    effect: [f32; 4],
    sampling: [u32; 4],
    fog: [f32; 4],
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
}

pub(super) struct Draw<'a> {
    pub target: &'a wgpu::TextureView,
    pub depth: &'a wgpu::TextureView,
    pub size: (u32, u32),
    pub format: wgpu::TextureFormat,
    pub frame: &'a FrameParams,
    pub settings: AoSettings,
}

impl AmbientOcclusion {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        Self {
            pipelines: Pipelines::new(device),
            params: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("AO params"),
                size: std::mem::size_of::<Params>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            images: None,
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
        let (divisor, slices, steps) = settings.quality.sampling();
        let size = (draw.size.0.div_ceil(divisor).max(1), draw.size.1.div_ceil(divisor).max(1));
        if self.images.as_ref().is_none_or(|i| i.size != size) {
            self.images = Some(Images::new(device, size));
        }
        let images = self.images.as_ref().expect("AO images exist");
        let tan_y = (fov_y * 0.5).tan();
        let [fog_start, fog_end] = draw.frame.fog_range();
        let params = Params {
            sizes: [draw.size.0 as f32, draw.size.1 as f32, size.0 as f32, size.1 as f32],
            projection: [tan_y * aspect, tan_y, near, 0.0],
            effect: [settings.radius, settings.intensity, 40.0, 100.0],
            sampling: [divisor, slices, steps, u32::from(settings.compare)],
            fog: [fog_start, fog_end, 0.0, 0.0],
        };
        queue.write_buffer(&self.params, 0, bytemuck::bytes_of(&params));
        let uniform = wgpu::BindGroupEntry { binding: 0, resource: self.params.as_entire_binding() };
        let depth = view(1, draw.depth);
        let generate = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("AO generate inputs"),
            layout: &self.pipelines.generate.get_bind_group_layout(0),
            entries: &[uniform.clone(), depth.clone(), view(2, &images.raw)],
        });
        let filter = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("AO filter inputs"),
            layout: &self.pipelines.filter.get_bind_group_layout(0),
            entries: &[uniform.clone(), view(2, &images.raw), view(3, &images.filtered)],
        });
        self.pipelines.use_format(device, draw.format);
        let composite = self.pipelines.composite.as_ref().expect("AO composite pipeline");
        let composite_inputs = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("AO composite inputs"),
            layout: &composite.get_bind_group_layout(0),
            entries: &[uniform, depth, view(2, &images.filtered)],
        });
        for (label, pipeline, inputs) in [
            ("AO horizons", &self.pipelines.generate, &generate),
            ("AO spatial denoise", &self.pipelines.filter, &filter),
        ] {
            let mut pass =
                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some(label), timestamp_writes: None });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, inputs, &[]);
            pass.dispatch_workgroups(size.0.div_ceil(8), size.1.div_ceil(8), 1);
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("AO composite"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: draw.target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(composite);
        pass.set_bind_group(0, &composite_inputs, &[]);
        pass.draw(0..3, 0..1);
    }
}

fn view(binding: u32, view: &wgpu::TextureView) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry { binding, resource: wgpu::BindingResource::TextureView(view) }
}

pub(crate) fn sources() -> [(&'static str, String); 3] {
    let common = include_str!("../../shaders/ao_common.wgsl");
    [
        ("AO horizons", include_str!("../../shaders/ao_horizon.wgsl")),
        ("AO filter", include_str!("../../shaders/ao_filter.wgsl")),
        ("AO composite", include_str!("../../shaders/ao_composite.wgsl")),
    ]
    .map(|(name, source)| (name, format!("{common}\n{source}")))
}
