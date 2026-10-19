//! AO-only history; independent of DLSS/FSR and never includes UI or generated frames.
use super::Draw;
use wgpu::util::DeviceExt;

pub(super) struct History {
    size: (u32, u32),
    images: [wgpu::TextureView; 2],
    index: usize,
    pipeline: wgpu::ComputePipeline,
    valid: bool,
    settings: blackbox_gfx::AoSettings,
}
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Reprojection {
    previous_from_current: [f32; 16],
    control: [f32; 4],
}

impl History {
    pub fn new(device: &wgpu::Device, size: (u32, u32), settings: blackbox_gfx::AoSettings) -> Self {
        let images = [0, 1].map(|_| super::Images::new(device, size).filtered);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("temporal AO reprojection"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{}\n{}",
                    include_str!("../../shaders/ao_common.wgsl"),
                    include_str!("../../shaders/ao_temporal.wgsl")
                )
                .into(),
            ),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("temporal AO"),
            layout: None,
            module: &shader,
            entry_point: Some("accumulate"),
            compilation_options: Default::default(),
            cache: None,
        });
        Self { size, images, index: 0, pipeline, valid: false, settings }
    }
    pub fn matches(&self, size: (u32, u32)) -> bool {
        self.size == size
    }
    pub fn encode(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        params: &wgpu::Buffer,
        current: &wgpu::TextureView,
        draw: &Draw<'_>,
    ) -> wgpu::TextureView {
        let motion = draw.motion;
        let valid =
            self.valid && self.settings == draw.settings && !draw.frame.camera_cut && motion.is_some_and(|m| !m.reset);
        self.settings = draw.settings;
        let transform = motion.map_or(glam::Mat4::IDENTITY, |m| m.previous_from_current);
        let jitter = motion.map_or(glam::Vec2::ZERO, |m| m.jitter_delta);
        let control = Reprojection {
            previous_from_current: transform.to_cols_array(),
            control: [u8::from(valid) as f32, 0.85, jitter.x, jitter.y],
        };
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("AO history rejection"),
            contents: bytemuck::bytes_of(&control),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let next = 1 - self.index;
        let motion_view = motion.map_or(current, |m| &m.view);
        let texture =
            |binding, view| wgpu::BindGroupEntry { binding, resource: wgpu::BindingResource::TextureView(view) };
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("AO history inputs"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: params.as_entire_binding() },
                texture(2, current),
                texture(6, &self.images[self.index]),
                texture(7, motion_view),
                texture(8, &self.images[next]),
                wgpu::BindGroupEntry { binding: 9, resource: uniform.as_entire_binding() },
            ],
        });
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("AO temporal accumulation"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(self.size.0.div_ceil(8), self.size.1.div_ceil(8), 1);
        drop(pass);
        self.index = next;
        self.valid = true;
        self.images[next].clone()
    }
}
