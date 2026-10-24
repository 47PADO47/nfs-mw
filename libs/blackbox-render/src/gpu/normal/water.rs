//! Current-frame road reflection snapshot and selective receiver redraw.
use super::*;
use crate::gpu::glossy::{screen::Screen, screen_draw::resume};

pub(super) struct Water {
    pub screen: Screen,
    pub disabled: wgpu::Buffer,
    pub environment: wgpu::TextureView,
    pub base_environment: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}
impl Water {
    pub fn new(device: &wgpu::Device) -> Self {
        let environment = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("absent road environment"),
                size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 6 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::Cube),
                ..Default::default()
            });
        let disabled = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("base road pass: rays disabled"),
            contents: bytemuck::cast_slice(&[0.0f32; 4]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        Self {
            screen: Screen::new(device),
            disabled,
            base_environment: environment.clone(),
            environment,
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("road reflection clamp"),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            }),
        }
    }
}
impl Normal {
    pub(super) fn rebind_water(&mut self, device: &wgpu::Device) {
        for m in self.materials.iter_mut() {
            m.bind_group = material_group(
                device,
                &self.material_layout,
                &self.sampler,
                &self.lighting,
                &m.views,
                &m.offset,
                &self.water,
                false,
            );
            m.water_group = None;
        }
    }
}
impl Renderer {
    pub(in crate::gpu) fn road_reflections_active(&self, frame: &FrameParams, capture: bool) -> bool {
        !capture
            && self.normal.is_some()
            && self.experiments.original_world_materials
            && self.experiments.enhanced_wet_surfaces
            && self.surface_water.dampness > 0.0
            && frame.surface_light.is_some_and(|l| l.reflection_scale > 0.0)
            && self.screen_lighting_active(frame)
    }
    #[allow(clippy::too_many_arguments)]
    pub(in crate::gpu) fn encode_material_reflections(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        frame: &FrameParams,
        _instances: &[blackbox_gfx::Instance],
        (cars, capture): (bool, bool),
    ) {
        if cars {
            self.encode_screen_glossy(encoder, target, depth, frame);
        }
        self.encode_road_reflections(encoder, target, depth, frame, capture);
    }
    pub(in crate::gpu) fn encode_road_reflections(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        frame: &FrameParams,
        capture: bool,
    ) {
        if !self.road_reflections_active(frame, capture) {
            return;
        }
        let blackbox_gfx::Projection::PerspectiveInfiniteReverse { near, .. } = frame.projection else { return };
        let Some(normal) = &mut self.normal else { return };
        let previous = normal.water.screen.color.clone();
        normal.water.screen.capture(&self.device, encoder, target, depth);
        if normal.water.screen.color != previous {
            normal.rebind_water(&self.device);
        }
        // The base draw binds the static environment, never the cube being captured.
        normal.water.screen.enable(
            &self.queue,
            [1.0, near, self.experiments.road_reflection_range, self.experiments.road_reflection_fade],
        );
        for m in normal.materials.iter_mut().filter(|m| m.uses_projected && m.water_group.is_none()) {
            m.water_group = Some(material_group(
                &self.device,
                &normal.material_layout,
                &normal.sampler,
                &normal.lighting,
                &m.views,
                &m.offset,
                &normal.water,
                true,
            ));
        }
        let format = target.texture().format();
        let store = self.draw_list.store.bind_group();
        let mut pass = resume(encoder, target, depth);
        pass.set_bind_group(0, &self.shared.bindings.globals_bind_group, &[]);
        // Scenery reads u32 slots after the glossy matrix prefix, exactly as the base draw does.
        pass.set_vertex_buffer(1, self.instances.buffer.slice(self.draw_list.store.main_offset()..));
        for (index, mode) in BLEND_ORDER.into_iter().take(2).enumerate() {
            normal.draw(
                &mut pass,
                format,
                (mode, frame.mirrored_view(), true),
                &self.meshes,
                self.draw_list.normal(index),
                &self.shared.shadows.fallback,
                store,
            );
        }
    }
}
