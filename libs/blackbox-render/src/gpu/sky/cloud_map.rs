//! Current-frame cloud lighting shared by all nearby views. No temporal history.

use super::super::resources::Globals;
use super::cloud_schedule::{Key, Rebuild, Schedule};

const SIZE: (u32, u32) = (1536, 384);

pub(super) struct CloudMap {
    pub layout: wgpu::BindGroupLayout,
    pub group: wgpu::BindGroup,
    pub(super) view: wgpu::TextureView,
    pipeline: Option<wgpu::RenderPipeline>,
    schedule: Schedule,
    origin: [f32; 3],
    pub updates: u64,
}

impl CloudMap {
    pub fn new(device: &wgpu::Device) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shared cloud lighting"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shared linear cloud radiance and transmittance"),
            size: wgpu::Extent3d { width: SIZE.0, height: SIZE.1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("cloud map seam wrapping"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shared cloud lighting"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        });
        Self { layout, group, view, pipeline: None, schedule: Schedule::default(), origin: [0.0; 3], updates: 0 }
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        shader: &wgpu::ShaderModule,
        bindings: [(&wgpu::BindGroupLayout, &wgpu::BindGroup); 2],
        globals: &Globals,
        state: blackbox_gfx::SkyState,
    ) {
        let key = Key {
            scroll: state.scroll * state.sky_wind_speed,
            coverage: state.cloud_coverage,
            sun: globals.to_sun,
            light: globals.light_dir[..3].try_into().unwrap(),
            warmth: state.sky_sunset_warmth,
        };
        let origin: [f32; 3] = globals.camera_pos[..3].try_into().unwrap();
        // Capture eyes differ by metres while clouds start kilometres away. Keep the
        // first nearby eye; large moves/teleports rebuild immediately, even at frozen time.
        // Wind and slow sun drift refresh one band of rows per call (see `cloud_schedule`).
        let distance_squared: f32 = origin.iter().zip(self.origin).map(|(a, b)| (a - b).powi(2)).sum();
        let Some(rebuild) = self.schedule.plan(key, distance_squared >= 32.0 * 32.0) else {
            return;
        };
        let _trace = blackbox_trace::scope("Renderer CPU: cloud map rebuilt");
        let pipeline = self.pipeline.get_or_insert_with(|| {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("cloud lighting generation"),
                bind_group_layouts: &[Some(bindings[0].0), Some(bindings[1].0)],
                immediate_size: 0,
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("cloud lighting generation"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: shader,
                    entry_point: Some("cache_vertex"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: shader,
                    entry_point: Some("cache_fragment"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        });
        let load = match rebuild {
            Rebuild::Full => wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            Rebuild::Band(_) => wgpu::LoadOp::Load,
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("shared cloud lighting"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.view,
                depth_slice: None,
                resolve_target: None,
                // A band keeps the other rows: each is a complete march from an earlier call, never a blend.
                ops: wgpu::Operations { load, store: wgpu::StoreOp::Store },
            })],
            ..Default::default()
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, bindings[0].1, &[]);
        pass.set_bind_group(1, bindings[1].1, &[]);
        let (y, height) = rebuild.rows(SIZE.1);
        pass.set_scissor_rect(0, y, SIZE.0, height);
        pass.draw(0..3, 0..1);
        if rebuild == Rebuild::Full {
            self.origin = origin;
        }
        self.updates += 1;
    }
}
