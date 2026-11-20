//! Meter the ungraded scene and apply the correction to the reconstructed image.
use super::filter::{Blend, Draw, Filter, Params, filter_source};
use super::{PassContext, PassIo, PostPass};
use blackbox_gfx::settings::post::AutoExposure;
use std::time::Instant;

pub(super) struct Exposure {
    meter: wgpu::ComputePipeline,
    history: wgpu::Buffer,
    params: wgpu::Buffer,
    binding: Option<(wgpu::TextureView, wgpu::BindGroup)>,
    filter: Filter,
    tuning: AutoExposure,
    last: Option<Instant>,
}

impl Exposure {
    pub(super) fn new(device: &wgpu::Device, tuning: AutoExposure, cache: Option<&wgpu::PipelineCache>) -> Self {
        let label = Some("auto-exposure");
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label,
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/auto_exposure.wgsl").into()),
        });
        let meter = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label,
            layout: None,
            module: &module,
            entry_point: Some("meter"),
            compilation_options: Default::default(),
            cache,
        });
        let buffer = |size, usage| {
            device.create_buffer(&wgpu::BufferDescriptor { label, size, usage, mapped_at_creation: false })
        };
        let history = buffer(4, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC);
        let params = buffer(32, wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST);
        let shader = filter_source(
            "@fragment fn apply(in: VsOut) -> @location(0) vec4<f32> {
            let c = textureLoad(src, vec2<i32>(in.clip.xy), 0); return vec4<f32>(c.rgb * exp2(params.a.x), 1.0); }",
        );
        let filter = Filter::new(device, "auto-exposure", &shader).with_pipeline_cache(cache.cloned());
        Self { meter, history, params, binding: None, filter, tuning, last: None }
    }
    fn bind_scene(&mut self, ctx: &PassContext<'_>) -> wgpu::BindGroup {
        if let Some((view, group)) = &self.binding
            && view == &ctx.scene.color_view
        {
            return group.clone();
        }
        let group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("exposure histogram"),
            layout: &self.meter.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&ctx.scene.color_view),
                },
                wgpu::BindGroupEntry { binding: 1, resource: self.history.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: self.params.as_entire_binding() },
            ],
        });
        self.binding = Some((ctx.scene.color_view.clone(), group.clone()));
        group
    }
}

impl PostPass for Exposure {
    fn name(&self) -> &'static str {
        "auto-exposure"
    }
    fn camera_cut(&mut self) {
        self.last = None;
    }
    fn encode(&mut self, ctx: &PassContext<'_>, io: &PassIo<'_>, encoder: &mut wgpu::CommandEncoder) {
        let now = Instant::now();
        let dt = self.last.replace(now).map_or(-1.0, |last| now.duration_since(last).as_secs_f32());
        let t = self.tuning;
        let params = Params {
            a: [t.neutral_log_luminance[0], t.neutral_log_luminance[1], t.min_ev, t.max_ev],
            b: [t.brighten_rate, t.darken_rate, t.compensation_ev, dt],
        };
        ctx.queue.write_buffer(&self.params, 0, bytemuck::bytes_of(&params));
        let group = self.bind_scene(ctx);
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("exposure histogram"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.meter);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        self.filter.copy_param_prefix(encoder, &self.history, 4);
        self.filter.draw(
            ctx.device,
            encoder,
            &Draw {
                entry: "apply",
                src: io.input,
                src2: io.input,
                target: io.output,
                format: io.output_format,
                blend: Blend::Replace,
            },
        );
    }
}
