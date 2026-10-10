//! The UI layer pass: the pipeline, the UI textures and the pass that draws a [`UiLayer`] over a target.
//!
//! The UI draws at the target's own resolution, last, with `LoadOp::Load`, so it is never post-processed
//! or upscaled. Colours and textures are used as stored (gamma space), which is what UI libraries expect
//! when the target is a plain UNORM image.

use std::collections::HashMap;

use blackbox_gfx::{UiLayer, UiTextureId, UiTexturePatch, UiVertex};

const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 3] =
    wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Unorm8x4];
const TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// Why a [`UiTexturePatch`] was refused. The patch is ignored; the caller decides whether to log it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiTextureError {
    /// The pixel data is not `width * height * 4` bytes, or the size is zero.
    BadSize { id: u64, bytes: usize, size: [u32; 2] },
    /// The region does not fit in the texture, or the texture does not exist.
    RegionDoesNotFit { id: u64 },
}

impl std::fmt::Display for UiTextureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadSize { id, bytes, size } => {
                write!(f, "ui texture {id}: {bytes} bytes for {}x{}", size[0], size[1])
            }
            Self::RegionDoesNotFit { id } => write!(f, "ui texture {id}: region does not fit"),
        }
    }
}

impl std::error::Error for UiTextureError {}

struct UiTexture {
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    size: [u32; 2],
}

/// Draws [`UiLayer`]s: textured, clipped, premultiplied-alpha triangles in points.
///
/// Typical use from any render loop:
/// 1. [`Self::update_texture`] / [`Self::free_texture`] as the UI's textures change;
/// 2. [`Self::set_layer`] with the layer to draw;
/// 3. [`Self::encode`] once per frame, after everything else has been drawn into the target.
pub struct UiPass {
    shader: wgpu::ShaderModule,
    pipeline_layout: wgpu::PipelineLayout,
    /// Per target format, built the first time the UI is drawn into it.
    pipelines: HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,
    globals: wgpu::Buffer,
    globals_bind_group: wgpu::BindGroup,
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    textures: HashMap<u64, UiTexture>,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    layer: UiLayer,
}

fn buffer(device: &wgpu::Device, usage: wgpu::BufferUsages, bytes: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("ui"),
        size: bytes.next_power_of_two().max(4096),
        usage: usage | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

impl UiPass {
    /// A pass with no pipelines yet: they are built per target format by [`Self::use_format`] or the
    /// first [`Self::encode`] into that format.
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ui shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/ui.wgsl").into()),
        });
        let globals_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ui globals"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let texture_layout = crate::world::texture_layout(device);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ui"),
            bind_group_layouts: &[Some(&globals_layout), Some(&texture_layout)],
            immediate_size: 0,
        });
        let globals = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ui globals"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let globals_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ui globals"),
            layout: &globals_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: globals.as_entire_binding() }],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ui"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Self {
            shader,
            pipeline_layout,
            pipelines: HashMap::new(),
            globals,
            globals_bind_group,
            texture_layout,
            sampler,
            textures: HashMap::new(),
            vertices: buffer(device, wgpu::BufferUsages::VERTEX, 0),
            indices: buffer(device, wgpu::BufferUsages::INDEX, 0),
            layer: UiLayer::default(),
        }
    }

    /// Build the pipeline for targets of `format` now (it is otherwise built by the first draw into it).
    pub fn use_format(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat) {
        if self.pipelines.contains_key(&format) {
            return;
        }
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ui"),
            layout: Some(&self.pipeline_layout),
            vertex: wgpu::VertexState {
                module: &self.shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<UiVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &VERTEX_ATTRIBUTES,
                })],
            },
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &self.shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        self.pipelines.insert(format, pipeline);
    }

    /// Create, replace or update a UI texture. A bad patch (wrong byte count, a region outside the
    /// texture, or a region of a texture that does not exist) is refused and changes nothing.
    pub fn update_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        patch: &UiTexturePatch<'_>,
    ) -> Result<(), UiTextureError> {
        let [w, h] = patch.size;
        let id = patch.id.raw();
        if w == 0 || h == 0 || patch.rgba.len() != (w as usize) * (h as usize) * 4 {
            return Err(UiTextureError::BadSize { id, bytes: patch.rgba.len(), size: patch.size });
        }
        let existing = self.textures.get(&id);
        let origin = match (patch.offset, existing) {
            (Some([x, y]), Some(t)) if x + w <= t.size[0] && y + h <= t.size[1] => [x, y],
            (None, Some(t)) if t.size == patch.size => [0, 0],
            (None, _) => {
                self.create_texture(device, patch.id, patch.size);
                [0, 0]
            }
            _ => return Err(UiTextureError::RegionDoesNotFit { id }),
        };
        let Some(t) = self.textures.get(&id) else { return Ok(()) };
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &t.texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x: origin[0], y: origin[1], z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            patch.rgba,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(w * 4), rows_per_image: Some(h) },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        Ok(())
    }

    fn create_texture(&mut self, device: &wgpu::Device, id: UiTextureId, size: [u32; 2]) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ui texture"),
            size: wgpu::Extent3d { width: size[0], height: size[1], depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TEXTURE_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ui texture"),
            layout: &self.texture_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
            ],
        });
        self.textures.insert(id.raw(), UiTexture { texture, bind_group, size });
    }

    /// Free a UI texture. Meshes that still name it are skipped.
    pub fn free_texture(&mut self, id: UiTextureId) {
        self.textures.remove(&id.raw());
    }

    /// The UI to draw on every following [`Self::encode`], until replaced. An empty layer draws nothing.
    pub fn set_layer(&mut self, layer: UiLayer) {
        self.layer = layer;
    }

    /// Draw the layer into `target` (an image of `format`, `size` pixels) on top of what is already there.
    /// Records nothing for an empty layer. One render pass, `LoadOp::Load`, no depth.
    pub fn encode(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        format: wgpu::TextureFormat,
        size: (u32, u32),
    ) {
        let ppp = self.layer.pixels_per_point.max(0.1);
        let (vertex_count, index_count) =
            self.layer.meshes.iter().fold((0, 0), |(v, i), m| (v + m.vertices.len(), i + m.indices.len()));
        if index_count == 0 {
            return;
        }
        self.use_format(device, format);
        let vertex_bytes = (vertex_count * std::mem::size_of::<UiVertex>()) as u64;
        let index_bytes = (index_count * 4) as u64;
        if vertex_bytes > self.vertices.size() {
            self.vertices = buffer(device, wgpu::BufferUsages::VERTEX, vertex_bytes);
        }
        if index_bytes > self.indices.size() {
            self.indices = buffer(device, wgpu::BufferUsages::INDEX, index_bytes);
        }
        let (mut vertices, mut indices) = (Vec::with_capacity(vertex_count), Vec::with_capacity(index_count));
        let mut draws = Vec::with_capacity(self.layer.meshes.len());
        for mesh in &self.layer.meshes {
            let Some(rect) = scissor(mesh.clip, ppp, size) else { continue };
            draws.push((
                mesh.texture,
                rect,
                indices.len() as u32..(indices.len() + mesh.indices.len()) as u32,
                vertices.len() as i32,
            ));
            vertices.extend_from_slice(&mesh.vertices);
            indices.extend_from_slice(&mesh.indices);
        }
        queue.write_buffer(&self.vertices, 0, bytemuck::cast_slice(&vertices));
        queue.write_buffer(&self.indices, 0, bytemuck::cast_slice(&indices));
        let screen = [size.0 as f32 / ppp, size.1 as f32 / ppp, 0.0, 0.0];
        queue.write_buffer(&self.globals, 0, bytemuck::cast_slice(&screen));

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ui"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipelines[&format]);
        pass.set_bind_group(0, &self.globals_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
        for (texture, (x, y, w, h), range, base_vertex) in draws {
            let Some(t) = self.textures.get(&texture.raw()) else { continue };
            pass.set_scissor_rect(x, y, w, h);
            pass.set_bind_group(1, &t.bind_group, &[]);
            pass.draw_indexed(range, base_vertex, 0..1);
        }
    }
}

/// The clip rectangle in pixels as `(x, y, width, height)`, kept inside a `width` × `height` target,
/// or `None` if nothing is left of it.
fn scissor(clip: [f32; 4], pixels_per_point: f32, target: (u32, u32)) -> Option<(u32, u32, u32, u32)> {
    let to_px = |v: f32, max: u32| (v * pixels_per_point).round().clamp(0.0, max as f32) as u32;
    let (x0, y0) = (to_px(clip[0], target.0), to_px(clip[1], target.1));
    let (x1, y1) = (to_px(clip[2], target.0), to_px(clip[3], target.1));
    (x1 > x0 && y1 > y0).then_some((x0, y0, x1 - x0, y1 - y0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scissor_scales_and_clamps() {
        assert_eq!(scissor([0.0, 0.0, 100.0, 50.0], 2.0, (1280, 720)), Some((0, 0, 200, 100)));
        assert_eq!(scissor([-10.0, -10.0, 5000.0, 5000.0], 1.0, (1280, 720)), Some((0, 0, 1280, 720)));
        assert_eq!(scissor([10.0, 10.0, 10.0, 40.0], 1.0, (1280, 720)), None, "zero width");
        assert_eq!(scissor([2000.0, 0.0, 3000.0, 10.0], 1.0, (1280, 720)), None, "off the target");
    }

    #[test]
    fn errors_name_the_texture() {
        let bad = UiTextureError::BadSize { id: 7, bytes: 3, size: [2, 2] };
        assert_eq!(bad.to_string(), "ui texture 7: 3 bytes for 2x2");
        assert_eq!(UiTextureError::RegionDoesNotFit { id: 9 }.to_string(), "ui texture 9: region does not fit");
    }
}
