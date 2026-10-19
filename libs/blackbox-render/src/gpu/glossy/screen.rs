//! Current-frame world colour, captured before glossy objects and UI are drawn.
use wgpu::util::DeviceExt;

pub(in crate::gpu) struct Screen {
    pub control: wgpu::Buffer,
    pub color: wgpu::TextureView,
    pub depth: wgpu::TextureView,
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
        }
    }
    pub fn disable(&self, queue: &wgpu::Queue) {
        queue.write_buffer(&self.control, 0, bytemuck::cast_slice(&[0.0f32; 4]));
    }
    pub fn capture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        source: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        near: f32,
    ) {
        let texture = source.texture();
        let size = (texture.width(), texture.height());
        if self.color.texture().size() != texture.size() || self.color.texture().format() != texture.format() {
            self.color = image(device, size, texture.format());
        }
        encoder.copy_texture_to_texture(texture.as_image_copy(), self.color.texture().as_image_copy(), texture.size());
        self.depth = depth.clone();
        queue.write_buffer(&self.control, 0, bytemuck::cast_slice(&[1.0, near, 50.0f32, 0.0]));
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
