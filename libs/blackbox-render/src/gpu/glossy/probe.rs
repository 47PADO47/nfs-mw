//! Current local probe depth and position; static environments explicitly disable correction.
use glam::Vec3;
use wgpu::util::DeviceExt;

use crate::gpu::queued::write_ordered;

pub(super) struct Probe {
    pub depth: wgpu::TextureView,
    pub control: wgpu::Buffer,
    pub sky: wgpu::Buffer,
    pub active: bool,
    /// The background colours of the current environment, before the night dims them (`sync_night`).
    sky_base: [[f32; 4]; 6],
    /// The night blend `sky` was last written with.
    sky_night: f32,
}

/// How far a full night dims the background colours. The environment faces hold a daytime sky, which a road
/// reflection's ray falls back to past the end of its trace.
const NIGHT_SKY: f32 = 0.12;

impl Probe {
    pub fn new(device: &wgpu::Device, size: u32, faces: [&[u8]; 6]) -> Self {
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
        let sky_base = sky_colours(size, faces);
        let sky = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("road background colours"),
            contents: bytemuck::cast_slice(&sky_base),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        Self { depth, control, sky, active: false, sky_base, sky_night: 0.0 }
    }

    /// Use `colours` as the daytime background, dimmed for the last night blend.
    pub fn set_sky(&mut self, queue: &wgpu::Queue, colours: [[f32; 4]; 6]) {
        self.sky_base = colours;
        self.write_sky(queue);
    }

    /// Dim the background colours for `night` (0 day, 1 full night); writes only when it changed.
    pub fn sync_night(&mut self, queue: &wgpu::Queue, night: f32) {
        if (night - self.sky_night).abs() < 1e-3 {
            return;
        }
        self.sky_night = night;
        self.write_sky(queue);
    }

    fn write_sky(&self, queue: &wgpu::Queue) {
        let scale = 1.0 + (NIGHT_SKY - 1.0) * self.sky_night;
        let colours = self.sky_base.map(|[r, g, b, a]| [r * scale, g * scale, b * scale, a]);
        queue.write_buffer(&self.sky, 0, bytemuck::cast_slice(&colours));
    }
}

impl super::Glossy {
    /// Dim the road reflections' background colours for `night` (`Probe::sync_night`).
    pub(in crate::gpu) fn sync_sky_night(&mut self, queue: &wgpu::Queue, night: f32) {
        self.probe.sync_night(queue, night);
    }
}

pub(super) fn sky_colours(size: u32, faces: [&[u8]; 6]) -> [[f32; 4]; 6] {
    let center = ((size / 2 * size + size / 2) * 4) as usize;
    faces.map(|face| std::array::from_fn(|channel| f32::from(face[center + channel]) / 255.0))
}

impl super::Glossy {
    pub fn set_probe(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: Option<&mut wgpu::CommandEncoder>,
        depth: &wgpu::TextureView,
        origin: Vec3,
        enabled: bool,
    ) {
        // `encoder` is given while captures are queued ahead of the frame: they keep the origin they were recorded
        // with, and the frame's views (all recorded after this) read the new one.
        // Positive: car correction enabled. Negative: captured depth remains valid for road fallback checks.
        let flag = match enabled {
            true => 1.0,
            false => -1.0,
        };
        let control = origin.extend(flag).to_array();
        write_ordered(device, queue, encoder, &self.probe.control, 0, bytemuck::cast_slice(&control));
        self.probe.active = true;
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
        let enabled = match (self.probe.active, options.local_reflection_correction) {
            (false, _) => 0.0_f32,
            (true, false) => -1.0,
            (true, true) => 1.0,
        };
        queue.write_buffer(&self.probe.control, 12, bytemuck::bytes_of(&enabled));
    }
}

impl super::Glossy {
    pub(in crate::gpu) fn window_environment(&self) -> &wgpu::TextureView {
        // Only the roughness-filtered capture can contribute local shapes on shop glass.
        // Otherwise use the distant environment. Never bind the sharp street capture here.
        if self.capture_bound
            && let Some(filter) = &self.filtered
        {
            return &filter.view;
        }
        &self.static_environment
    }

    pub(in crate::gpu) fn road_environment(&self) -> ([wgpu::TextureView; 4], [wgpu::Buffer; 2]) {
        (
            [
                self.environment.clone(),
                self.static_environment.clone(),
                self.filtered.as_ref().map_or(&self.environment, |f| &f.view).clone(),
                self.probe.depth.clone(),
            ],
            [self.probe.control.clone(), self.probe.sky.clone()],
        )
    }
}
