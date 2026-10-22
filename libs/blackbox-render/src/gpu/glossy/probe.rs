//! Current local probe depth and position; static environments explicitly disable correction.
use glam::Vec3;
use wgpu::util::DeviceExt;

pub(super) struct Probe {
    pub depth: wgpu::TextureView,
    pub control: wgpu::Buffer,
    pub active: bool,
}

impl Probe {
    pub fn new(device: &wgpu::Device) -> Self {
        let depth = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("absent probe depth"),
                size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 6 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                ..Default::default()
            });
        let control = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("local probe control"),
            contents: bytemuck::cast_slice(&[0.0_f32; 4]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        Self { depth, control, active: false }
    }
}

impl super::Glossy {
    pub fn set_probe(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        depth: &wgpu::TextureView,
        origin: Vec3,
        enabled: bool,
    ) {
        queue.write_buffer(
            &self.probe.control,
            0,
            bytemuck::cast_slice(&origin.extend(u8::from(enabled) as f32).to_array()),
        );
        if self.probe.depth == *depth {
            return;
        }
        self.probe.depth = depth.clone();
        self.scene = super::scene_group(
            device,
            &self.layouts,
            &self.rig,
            &self.environment,
            &self.faces,
            &self.sampler,
            &self.irradiance.buffer,
            &self.screen,
            self.filtered.as_ref().map_or(&self.environment, |f| &f.view),
            self.filtered.as_ref().map_or(&self.environment, |f| &f.linear_view),
            &self.probe,
        );
    }
}

impl super::Glossy {
    pub(in crate::gpu) fn sync_probe_options(&self, queue: &wgpu::Queue, options: &blackbox_gfx::RenderExperiments) {
        // Update only unused scalar lanes; no replacement rig or recapture is needed.
        queue.write_buffer(&self.rig, 12, bytemuck::bytes_of(&options.sun_angle.to_radians().sin().powi(2)));
        let enabled = f32::from(self.probe.active && options.local_reflection_correction);
        queue.write_buffer(&self.probe.control, 12, bytemuck::bytes_of(&enabled));
    }
}
