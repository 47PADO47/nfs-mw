//! The UI layer pass: the pipeline, the UI textures and the pass that draws a [`UiLayer`] over a target.
//!
//! The UI draws at the target's own resolution, last, with `LoadOp::Load`, so it is never post-processed
//! or upscaled. Colours and textures are used as stored (gamma space), which is what UI libraries expect
//! when the target is a plain UNORM image.

use std::collections::HashMap;

use blackbox_gfx::{UiLayer, UiTextureComposition, UiTextureId, UiTexturePatch, UiVertex};

mod layer_ref;
mod mask;
mod picture;

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

/// One draw: the texture, the scissor `(x, y, width, height)` in pixels, the indices and the base vertex.
type Draw = (UiTextureId, (u32, u32, u32, u32), std::ops::Range<u32>, i32);

/// Draws [`UiLayer`]s: textured, clipped, premultiplied-alpha triangles in points.
///
/// Typical use from any render loop:
/// 1. [`Self::update_texture`] / [`Self::free_texture`] as the UI's textures change;
/// 2. [`Self::set_layer`] with the layer to draw;
/// 3. [`Self::encode`] once per frame, after everything else has been drawn into the target.
///
/// The vertices and indices are uploaded only when the layer differs from the one already on the GPU.
pub struct UiPass {
    shader: wgpu::ShaderModule,
    pipeline_layout: wgpu::PipelineLayout,
    /// Per target format, built the first time the UI is drawn into it.
    pipelines: HashMap<(wgpu::TextureFormat, bool, bool), wgpu::RenderPipeline>,
    hdr_white: Option<f32>,
    globals: wgpu::Buffer,
    globals_bind_group: wgpu::BindGroup,
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    textures: HashMap<u64, UiTexture>,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    layer: UiLayer,
    /// The vertex and index buffers do not hold the current `layer` yet.
    stale: bool,
    /// Reused by every upload, so a steady layer allocates nothing.
    vertex_staging: Vec<UiVertex>,
    index_staging: Vec<u32>,
    /// Reused by every encode.
    draws: Vec<Draw>,
    /// Used by the pipelines built per format.
    cache: Option<wgpu::PipelineCache>,
    mask_pass: Option<mask::MaskPass>,
    picture_pass: Option<picture::PicturePass>,
    pending_compositions: HashMap<UiTextureId, UiTextureComposition>,
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
    pub fn new(device: &wgpu::Device, cache: Option<&wgpu::PipelineCache>) -> Self {
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
            hdr_white: None,
            globals,
            globals_bind_group,
            texture_layout,
            sampler,
            textures: HashMap::new(),
            vertices: buffer(device, wgpu::BufferUsages::VERTEX, 0),
            indices: buffer(device, wgpu::BufferUsages::INDEX, 0),
            layer: UiLayer::default(),
            stale: false,
            vertex_staging: Vec::new(),
            index_staging: Vec::new(),
            draws: Vec::new(),
            cache: cache.cloned(),
            mask_pass: None,
            picture_pass: None,
            pending_compositions: HashMap::new(),
        }
    }

    /// Build the pipeline for targets of `format` now (it is otherwise built by the first draw into it).
    pub fn use_format(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat) {
        self.use_format_coverage(device, format, false);
    }

    fn use_format_coverage(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat, coverage: bool) {
        let key = (format, self.hdr_white.is_some(), coverage);
        if self.pipelines.contains_key(&key) {
            return;
        }
        let entry_point = match (self.hdr_white.is_some(), crate::world::is_srgb(format)) {
            (true, _) => "fs_main_hdr",
            (false, true) => "fs_main_srgb",
            (false, false) => "fs_main",
        };
        let entry_point = match (coverage, entry_point) {
            (true, "fs_main_hdr") => "fs_coverage_hdr",
            (true, "fs_main_srgb") => "fs_coverage_srgb",
            (true, _) => "fs_coverage",
            _ => entry_point,
        };
        let mut targets = vec![Some(wgpu::ColorTargetState {
            format,
            blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
        })];
        if coverage {
            targets.push(Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::R16Float,
                blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::RED,
            }));
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
                entry_point: Some(entry_point),
                compilation_options: Default::default(),
                targets: &targets,
            }),
            multiview_mask: None,
            cache: self.cache.as_ref(),
        });
        self.pipelines.insert(key, pipeline);
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
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::STORAGE_BINDING,
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
        self.pending_compositions.remove(&id);
    }

    /// The UI to draw on every following [`Self::encode`], until replaced. An empty layer draws nothing.
    /// A layer equal to the current one changes nothing, so the next encode does not upload it again.
    pub fn set_layer(&mut self, mut layer: UiLayer) {
        for command in layer.compositions.drain(..) {
            self.pending_compositions.insert(command.output, command);
        }
        if layer == self.layer {
            return;
        }
        self.layer = layer;
        self.stale = true;
    }

    /// UI reference white in absolute nits for a linear scRGB target; None retains SDR.
    pub fn set_hdr_white(&mut self, nits: Option<f32>) {
        self.hdr_white = nits.filter(|v| v.is_finite()).map(|v| v.clamp(16.0, 10000.0));
    }
    pub fn hdr_white(&self) -> Option<f32> {
        self.hdr_white
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
        self.encode_targets(device, queue, encoder, (target, None), format, size);
    }

    /// Draw colour and an independent R16F UI coverage mask in one pass. Clear both targets first.
    /// Coverage includes additive elements before overlaps, HDR conversion and white calibration.
    pub fn encode_with_coverage(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        coverage: &wgpu::TextureView,
        size: (u32, u32),
    ) {
        self.encode_targets(device, queue, encoder, (target, Some(coverage)), target.texture().format(), size);
    }

    fn encode_targets(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        targets: (&wgpu::TextureView, Option<&wgpu::TextureView>),
        format: wgpu::TextureFormat,
        size: (u32, u32),
    ) {
        self.compose_textures(device, encoder);
        let (target, coverage) = targets;
        let ppp = self.layer.pixels_per_point.max(0.1);
        let (vertex_count, index_count) =
            self.layer.meshes.iter().fold((0, 0), |(v, i), m| (v + m.vertices.len(), i + m.indices.len()));
        if index_count == 0 {
            return;
        }
        self.use_format_coverage(device, format, coverage.is_some());
        let vertex_bytes = (vertex_count * std::mem::size_of::<UiVertex>()) as u64;
        let index_bytes = (index_count * 4) as u64;
        if vertex_bytes > self.vertices.size() {
            self.vertices = buffer(device, wgpu::BufferUsages::VERTEX, vertex_bytes);
            self.stale = true;
        }
        if index_bytes > self.indices.size() {
            self.indices = buffer(device, wgpu::BufferUsages::INDEX, index_bytes);
            self.stale = true;
        }
        if self.stale {
            self.upload(queue);
            self.stale = false;
        }
        self.collect_draws(ppp, size);
        let screen = [size.0 as f32 / ppp, size.1 as f32 / ppp, self.hdr_white.unwrap_or(80.0) / 80.0, 0.0];
        queue.write_buffer(&self.globals, 0, bytemuck::cast_slice(&screen));

        let attachments: Vec<_> = std::iter::once(target)
            .chain(coverage)
            .map(|view| {
                Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                })
            })
            .collect();
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ui"),
            color_attachments: &attachments,
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipelines[&(format, self.hdr_white.is_some(), coverage.is_some())]);
        pass.set_bind_group(0, &self.globals_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
        for (texture, (x, y, w, h), range, base_vertex) in &self.draws {
            let Some(t) = self.textures.get(&texture.raw()) else { continue };
            pass.set_scissor_rect(*x, *y, *w, *h);
            pass.set_bind_group(1, &t.bind_group, &[]);
            pass.draw_indexed(range.clone(), *base_vertex, 0..1);
        }
    }

    /// Copy every mesh of the layer, in order, into the vertex and index buffers.
    fn upload(&mut self, queue: &wgpu::Queue) {
        self.vertex_staging.clear();
        self.index_staging.clear();
        for mesh in &self.layer.meshes {
            self.vertex_staging.extend_from_slice(&mesh.vertices);
            self.index_staging.extend_from_slice(&mesh.indices);
        }
        queue.write_buffer(&self.vertices, 0, bytemuck::cast_slice(&self.vertex_staging));
        queue.write_buffer(&self.indices, 0, bytemuck::cast_slice(&self.index_staging));
    }

    /// The draws for a target of `size` pixels: every mesh with a clip that is left on the target, with its
    /// indices and base vertex in the buffers. The buffers hold all the meshes, in layer order, so they do not
    /// depend on the target and need no upload when only the size changes.
    fn collect_draws(&mut self, ppp: f32, size: (u32, u32)) {
        self.draws.clear();
        let (mut first_vertex, mut first_index) = (0usize, 0usize);
        for mesh in &self.layer.meshes {
            let indices = first_index as u32..(first_index + mesh.indices.len()) as u32;
            let base_vertex = first_vertex as i32;
            first_vertex += mesh.vertices.len();
            first_index += mesh.indices.len();
            let Some(rect) = scissor(mesh.clip, ppp, size) else { continue };
            self.draws.push((mesh.texture, rect, indices, base_vertex));
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
