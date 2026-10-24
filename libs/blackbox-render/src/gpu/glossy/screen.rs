//! Current-frame world colour, captured before glossy objects and UI are drawn.
use wgpu::util::DeviceExt;

/// The views the glossy scene group binds and that can be replaced: it is rebuilt only when one of them is.
#[derive(PartialEq)]
pub(in crate::gpu) struct SceneKey {
    pub environment: wgpu::TextureView,
    pub faces: wgpu::TextureView,
    pub filtered: wgpu::TextureView,
    pub linear_filtered: wgpu::TextureView,
    pub color: wgpu::TextureView,
    pub depth: wgpu::TextureView,
    pub probe: wgpu::TextureView,
}

pub(in crate::gpu) struct Screen {
    pub control: wgpu::Buffer,
    pub color: wgpu::TextureView,
    pub depth: wgpu::TextureView,
    depth_copy: Option<wgpu::RenderPipeline>,
    /// The glossy scene group of the last [`Self::scene_group`], with the views it was built from.
    scene: Option<(SceneKey, wgpu::BindGroup)>,
    /// The depth snapshot's input group with the scene depth view it reads, kept while that view stays the same.
    inputs: Option<(wgpu::TextureView, wgpu::BindGroup)>,
}
impl Screen {
    pub fn new(device: &wgpu::Device) -> Self {
        let control = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("screen reflection control"),
            contents: bytemuck::cast_slice(&[0.0f32; 4]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        Self {
            control,
            color: image(device, (1, 1), wgpu::TextureFormat::Rgba16Float),
            depth: super::super::resources::create_depth(device, 1, 1),
            depth_copy: None,
            scene: None,
            inputs: None,
        }
    }
    /// The glossy scene group for `key`: the one built last time when the views are the same, else `build`'s.
    /// `build` gets the screen so it can bind its current colour and depth.
    pub fn scene_group(&mut self, key: SceneKey, build: impl FnOnce(&Screen) -> wgpu::BindGroup) -> wgpu::BindGroup {
        if let Some((cached, group)) = &self.scene
            && *cached == key
        {
            return group.clone();
        }
        let group = build(self);
        self.scene = Some((key, group.clone()));
        group
    }
    pub fn disable(&self, queue: &wgpu::Queue) {
        queue.write_buffer(&self.control, 0, bytemuck::cast_slice(&[0.0f32; 4]));
    }
    /// Switch the reflection on with the ray `control` block: enabled flag, near plane, range and fade.
    pub fn enable(&self, queue: &wgpu::Queue, control: [f32; 4]) {
        queue.write_buffer(&self.control, 0, bytemuck::cast_slice(&control));
    }
    /// Copy the world colour and the scenery depth drawn so far. The caller sets the ray control with
    /// [`Self::enable`].
    pub fn capture(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        source: &wgpu::TextureView,
        depth: &wgpu::TextureView,
    ) {
        let texture = source.texture();
        let size = (texture.width(), texture.height());
        if self.color.texture().size() != texture.size() || self.color.texture().format() != texture.format() {
            self.color = image(device, size, texture.format());
        }
        encoder.copy_texture_to_texture(texture.as_image_copy(), self.color.texture().as_image_copy(), texture.size());
        if self.depth.texture().size() != depth.texture().size() {
            self.depth = super::super::resources::create_depth(device, size.0, size.1);
        }
        let pipeline = self.depth_copy.get_or_insert_with(|| depth_pipeline(device));
        let inputs = match self.inputs.take() {
            Some(kept) if kept.0 == *depth => kept,
            _ => {
                let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("scenery-only reflection depth"),
                    layout: &pipeline.get_bind_group_layout(0),
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(depth),
                    }],
                });
                (depth.clone(), group)
            }
        };
        let (_, inputs) = self.inputs.insert(inputs);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("scenery reflection depth snapshot"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth,
                depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(0.0), store: wgpu::StoreOp::Store }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &*inputs, &[]);
        pass.draw(0..3, 0..1);
    }
}
fn image(device: &wgpu::Device, size: (u32, u32), format: wgpu::TextureFormat) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("world colour for selective reflections"),
            size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&Default::default())
}

fn depth_pipeline(device: &wgpu::Device) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("scenery depth snapshot"),
        source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/reflection_depth.wgsl").into()),
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("scenery depth snapshot"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[],
        }),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Always),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        primitive: Default::default(),
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    })
}
