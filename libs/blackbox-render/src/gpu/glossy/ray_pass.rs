//! Shared RT scene bound once per car pass, never per material.
use super::*;
use crate::gpu::{
    cull::{FACES, Face},
    instances::INSTANCE_LAYOUT,
    pipelines::{BLEND_ORDER, VERTEX_ATTRIBUTES},
    resources::DEPTH_FORMAT,
    targets::write_mask,
};
use blackbox_gfx::{BlendMode, FrameParams, Vertex};
use std::collections::HashMap;

pub(in crate::gpu) struct RayPass {
    shader: wgpu::ShaderModule,
    layout: wgpu::PipelineLayout,
    scene_layout: wgpu::BindGroupLayout,
    pub group: wgpu::BindGroup,
    pipelines: HashMap<(wgpu::TextureFormat, u8, Face), wgpu::RenderPipeline>,
}
impl RayPass {
    fn new(
        device: &wgpu::Device,
        shared: &crate::gpu::resources::Shared,
        layouts: &Layouts,
        bindings: &crate::gpu::ray_reflections::Bindings,
    ) -> Self {
        let scene_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("car RT scene"),
            entries: &crate::gpu::ray_reflections::layout_entries(),
        });
        let group = Self::group(device, &scene_layout, bindings, &shared.bindings.globals);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("car RT pipeline"),
            bind_group_layouts: &[
                Some(&scene_layout),
                Some(&shared.bindings.texture_layout),
                Some(&layouts.scene),
                Some(&layouts.material),
            ],
            immediate_size: 0,
        });
        let source = super::RAY_SHADER;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("car RT shader"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "enable wgpu_ray_query;\n{}\n{}\n{}",
                    source,
                    include_str!("../../shaders/shadow_receiver.wgsl").replace("SHADOW_GROUP", "3"),
                    include_str!("../../shaders/ray_reflection.wgsl")
                )
                .into(),
            ),
        });
        Self { shader, layout, scene_layout, group, pipelines: HashMap::new() }
    }
    fn group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        bindings: &crate::gpu::ray_reflections::Bindings,
        globals: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shared car RT scene"),
            layout,
            entries: &bindings.entries(globals),
        })
    }
    pub fn set_scene(
        &mut self,
        device: &wgpu::Device,
        bindings: &crate::gpu::ray_reflections::Bindings,
        globals: &wgpu::Buffer,
    ) {
        self.group = Self::group(device, &self.scene_layout, bindings, globals);
    }
    fn prepare(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat) {
        for mode in BLEND_ORDER {
            for face in FACES {
                let key = (format, mode as u8, face);
                if self.pipelines.contains_key(&key) {
                    continue;
                }
                let (entry, blend, write) = match mode {
                    BlendMode::Opaque => ("fs_opaque", None, true),
                    BlendMode::AlphaTest => ("fs_alpha_test", None, true),
                    BlendMode::AlphaBlend => ("fs_blend", Some(wgpu::BlendState::ALPHA_BLENDING), false),
                    BlendMode::Subtractive => ("fs_blend", Some(crate::gpu::pipelines::SUBTRACTIVE), false),
                    BlendMode::Additive => (
                        "fs_blend",
                        Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::SrcAlpha,
                                dst_factor: wgpu::BlendFactor::One,
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent::OVER,
                        }),
                        false,
                    ),
                };
                let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("car RT material"),
                    layout: Some(&self.layout),
                    vertex: wgpu::VertexState {
                        module: &self.shader,
                        entry_point: Some("vs_main"),
                        compilation_options: Default::default(),
                        buffers: &[
                            Some(wgpu::VertexBufferLayout {
                                array_stride: std::mem::size_of::<Vertex>() as u64,
                                step_mode: wgpu::VertexStepMode::Vertex,
                                attributes: &VERTEX_ATTRIBUTES,
                            }),
                            Some(INSTANCE_LAYOUT),
                        ],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &self.shader,
                        entry_point: Some(entry),
                        compilation_options: Default::default(),
                        targets: &[Some(wgpu::ColorTargetState { format, blend, write_mask: write_mask(format) })],
                    }),
                    primitive: wgpu::PrimitiveState { cull_mode: face.cull_mode(), ..Default::default() },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: DEPTH_FORMAT,
                        depth_write_enabled: Some(write),
                        depth_compare: Some(wgpu::CompareFunction::GreaterEqual),
                        stencil: Default::default(),
                        bias: Default::default(),
                    }),
                    multisample: Default::default(),
                    multiview_mask: None,
                    cache: None,
                });
                self.pipelines.insert(key, pipeline);
            }
        }
    }
    pub fn get(&self, format: wgpu::TextureFormat, mode: BlendMode, face: Face) -> Option<&wgpu::RenderPipeline> {
        self.pipelines.get(&(format, mode as u8, face))
    }
}
impl Renderer {
    pub(in crate::gpu) fn ray_cars_active(&self, frame: &FrameParams) -> bool {
        self.experiments.ray_car_reflections
            && self.ray_reflections.is_some()
            && self.glossy.is_some()
            && self.screen_lighting_active(frame)
    }
    pub(in crate::gpu) fn prepare_ray_cars(&mut self, format: wgpu::TextureFormat) {
        let Some(rt) = &self.ray_reflections else { return };
        let Some(glossy) = &mut self.glossy else { return };
        let ray =
            glossy.ray.get_or_insert_with(|| RayPass::new(&self.device, &self.shared, &glossy.layouts, &rt.bindings));
        ray.prepare(&self.device, format);
    }
}
