//! The textures, pipelines and passes the test scene is drawn with.

use glam::UVec2;

pub(super) const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
pub(super) const MOTION_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rg16Float;
pub(super) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
pub(super) const REFERENCE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba32Float;
/// Subsamples per axis and output pixel of the reference.
pub(super) const REFERENCE_SAMPLES: f32 = 8.0;

pub(super) struct Targets {
    pub color: wgpu::Texture,
    pub motion: wgpu::Texture,
    /// The motion vectors at the output resolution, when the configuration asks for them.
    pub motion_display: wgpu::Texture,
    pub depth: wgpu::Texture,
}

pub(super) fn texture(
    device: &wgpu::Device,
    size: UVec2,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("fsr3 test"),
        size: wgpu::Extent3d { width: size.x, height: size.y, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    })
}

pub(super) fn view(texture: &wgpu::Texture) -> wgpu::TextureView {
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

/// The textures that depend on the render and output size.
pub(super) fn create_images(
    device: &wgpu::Device,
    render: UVec2,
    display: UVec2,
    output_format: wgpu::TextureFormat,
) -> (Targets, wgpu::Texture, wgpu::Texture) {
    let render_usage = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
    let targets = Targets {
        color: texture(device, render, COLOR_FORMAT, render_usage | wgpu::TextureUsages::COPY_SRC),
        motion: texture(device, render, MOTION_FORMAT, render_usage),
        motion_display: texture(device, display, MOTION_FORMAT, render_usage),
        depth: texture(device, render, DEPTH_FORMAT, render_usage),
    };
    let output =
        texture(device, display, output_format, wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC);
    let reference = texture(
        device,
        display,
        REFERENCE_FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    (targets, output, reference)
}

pub(super) fn pipeline(
    device: &wgpu::Device,
    module: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    entry: &str,
    targets: &[Option<wgpu::ColorTargetState>],
    depth: bool,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(entry),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: depth.then(|| wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Always),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module,
            entry_point: Some(entry),
            compilation_options: Default::default(),
            targets,
        }),
        multiview_mask: None,
        cache: None,
    })
}

pub(super) fn target(format: wgpu::TextureFormat) -> Option<wgpu::ColorTargetState> {
    Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })
}

pub(super) fn color_attachment(view: &wgpu::TextureView) -> Option<wgpu::RenderPassColorAttachment<'_>> {
    Some(wgpu::RenderPassColorAttachment {
        view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
    })
}
