//! RT resources are shared once by the wet-road redraw, never by ordinary materials.
use super::*;

pub(super) struct RayPass {
    shader: wgpu::ShaderModule,
    layout: wgpu::PipelineLayout,
    scene_layout: wgpu::BindGroupLayout,
    pub enabled: bool,
    pub group: Option<wgpu::BindGroup>,
    pub pipelines: HashMap<wgpu::TextureFormat, pipeline::FaceSets>,
}
impl RayPass {
    pub fn new(
        device: &wgpu::Device,
        materials: &wgpu::BindGroupLayout,
        shadows: &wgpu::BindGroupLayout,
        instances: Option<&wgpu::BindGroupLayout>,
    ) -> Option<Self> {
        if !device.features().contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY) {
            return None;
        }
        let scene_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("wet-road RT scene"),
            entries: &super::super::ray_reflections::layout_entries(),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("wet-road RT pipeline"),
            bind_group_layouts: &[Some(&scene_layout), Some(materials), Some(shadows), instances],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("wet-road RT shader"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "enable wgpu_ray_query;\n{}\n{}\n{}\n{}\n{}",
                    instance_view::instance_source(include_str!("../../shaders/normal.wgsl"), instances.map(|_| 3)),
                    include_str!("../../shaders/shadow_receiver.wgsl").replace("SHADOW_GROUP", "2"),
                    include_str!("../../shaders/ray_reflection.wgsl"),
                    include_str!("../../shaders/ray_road.wgsl"),
                    include_str!("../../shaders/road_probe.wgsl")
                )
                .into(),
            ),
        });
        Some(Self { enabled: true, shader, layout, scene_layout, group: None, pipelines: HashMap::new() })
    }
    pub fn set_scene(
        &mut self,
        device: &wgpu::Device,
        bindings: &super::super::ray_reflections::Bindings,
        globals: &wgpu::Buffer,
    ) {
        self.group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shared wet-road RT scene"),
            layout: &self.scene_layout,
            entries: &bindings.entries(globals),
        }));
    }
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        culled: bool,
        store: bool,
        cache: Option<&wgpu::PipelineCache>,
    ) {
        let (shader, layout) = (&self.shader, &self.layout);
        pipeline::fill_faces(self.pipelines.entry(format).or_default(), culled, |mode, face| {
            pipeline::create(device, shader, layout, format, (mode, face), true, store, false, cache)
        });
    }
}
