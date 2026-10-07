//! The wgpu implementation of the renderer (Vulkan, Direct3D 12, OpenGL).

use std::sync::Arc;

use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::{
    Backend, BlendMode, DrawRange, FrameParams, MeshDesc, MeshHandle, PixelFormat, RenderError, RendererOptions,
    TextureDesc, TextureHandle, Vertex,
};

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Globals {
    view_proj: [[f32; 4]; 4],
    light_dir: [f32; 4],
}

struct GpuMesh {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    draws: Vec<DrawRange>,
}

struct Pipelines {
    opaque: wgpu::RenderPipeline,
    alpha_test: wgpu::RenderPipeline,
    blend: wgpu::RenderPipeline,
}

pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    adapter_info: wgpu::AdapterInfo,
    supports_bc: bool,
    depth: wgpu::TextureView,
    globals: wgpu::Buffer,
    globals_bind_group: wgpu::BindGroup,
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    pipelines: Pipelines,
    /// Bind groups, indexed by `TextureHandle`. Slot 0 is the white fallback.
    textures: Vec<wgpu::BindGroup>,
    meshes: Vec<GpuMesh>,
}

fn wgpu_backends(backend: Backend) -> Result<wgpu::Backends, RenderError> {
    Ok(match backend {
        Backend::Auto => wgpu::Backends::PRIMARY | wgpu::Backends::GL,
        Backend::Vulkan => wgpu::Backends::VULKAN,
        Backend::Dx12 => wgpu::Backends::DX12,
        Backend::Gl => wgpu::Backends::GL,
        Backend::Dx11 => {
            return Err(RenderError::BackendUnavailable(
                "the Direct3D 11 backend is not implemented yet: wgpu has no D3D11 backend, so it needs its \
                 own renderer (planned, see docs/architecture.md). Use --backend dx12, vulkan or gl."
                    .into(),
            ));
        }
    })
}

impl Renderer {
    /// Create a renderer drawing into `window`.
    ///
    /// `display` is the event loop's display handle (needed by OpenGL on Wayland/X11).
    pub fn new(
        window: Arc<Window>,
        display: winit::event_loop::OwnedDisplayHandle,
        options: RendererOptions,
    ) -> Result<Self, RenderError> {
        let backends = wgpu_backends(options.backend)?;
        let mut desc = wgpu::InstanceDescriptor::new_without_display_handle().with_display_handle(Box::new(display));
        desc.backends = backends;
        let instance = wgpu::Instance::new(desc);
        let surface = instance.create_surface(window.clone()).map_err(|e| RenderError::Surface(e.to_string()))?;

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
            apply_limit_buckets: false,
        }))
        .map_err(|e| RenderError::NoAdapter { backend: options.backend, detail: e.to_string() })?;
        let adapter_info = adapter.get_info();
        log::info!("GPU: {} ({:?}, driver {})", adapter_info.name, adapter_info.backend, adapter_info.driver);

        let supports_bc = adapter.features().contains(wgpu::Features::TEXTURE_COMPRESSION_BC);
        let required_features =
            if supports_bc { wgpu::Features::TEXTURE_COMPRESSION_BC } else { wgpu::Features::empty() };
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("nfsmw device"),
            required_features,
            required_limits: wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
            ..Default::default()
        }))
        .map_err(|e| RenderError::Device(e.to_string()))?;

        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or_else(|| RenderError::Surface("the surface is not supported by this adapter".into()))?;
        // The game's art is authored for a non-sRGB D3D9 pipeline: prefer a plain UNORM target.
        let caps = surface.get_capabilities(&adapter);
        if let Some(f) = caps.formats.iter().copied().find(|f| !f.is_srgb()) {
            config.format = f;
        }
        config.present_mode = if options.vsync { wgpu::PresentMode::AutoVsync } else { wgpu::PresentMode::AutoNoVsync };
        surface.configure(&device, &config);

        let globals = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globals"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let globals_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("globals"),
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
        let globals_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("globals"),
            layout: &globals_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: globals.as_entire_binding() }],
        });
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("texture"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("repeat"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });

        let pipelines = create_pipelines(&device, config.format, &globals_layout, &texture_layout);
        let depth = create_depth(&device, config.width, config.height);

        let mut renderer = Self {
            surface,
            device,
            queue,
            config,
            adapter_info,
            supports_bc,
            depth,
            globals,
            globals_bind_group,
            texture_layout,
            sampler,
            pipelines,
            textures: Vec::new(),
            meshes: Vec::new(),
        };
        let white = [255u8; 4];
        renderer.create_texture(&TextureDesc {
            label: "white",
            width: 1,
            height: 1,
            format: PixelFormat::Rgba8,
            mips: vec![&white],
        });
        Ok(renderer)
    }

    /// "GPU name (backend)" for logs and the window title.
    pub fn adapter_summary(&self) -> String {
        format!("{} ({:?})", self.adapter_info.name, self.adapter_info.backend)
    }

    /// Whether DXT (BC1–3) textures can be uploaded without CPU decoding.
    pub fn supports_bc(&self) -> bool {
        self.supports_bc
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.depth = create_depth(&self.device, width, height);
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.config.width as f32 / self.config.height.max(1) as f32
    }

    /// Upload a texture. BC formats must only be used when [`Self::supports_bc`] is true.
    pub fn create_texture(&mut self, desc: &TextureDesc<'_>) -> TextureHandle {
        let format = match desc.format {
            PixelFormat::Bc1 => wgpu::TextureFormat::Bc1RgbaUnorm,
            PixelFormat::Bc2 => wgpu::TextureFormat::Bc2RgbaUnorm,
            PixelFormat::Bc3 => wgpu::TextureFormat::Bc3RgbaUnorm,
            PixelFormat::Rgba8 => wgpu::TextureFormat::Rgba8Unorm,
        };
        let (block, block_bytes) = match desc.format {
            PixelFormat::Bc1 => (4, 8),
            PixelFormat::Bc2 | PixelFormat::Bc3 => (4, 16),
            PixelFormat::Rgba8 => (1, 4),
        };
        // wgpu requires block-compressed textures to have a block-aligned base size;
        // callers decode anything else to RGBA8 first (see `PixelFormat`).
        let mip_count = desc.mips.len().max(1) as u32;
        let size = wgpu::Extent3d { width: desc.width, height: desc.height, depth_or_array_layers: 1 };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(desc.label),
            size,
            mip_level_count: mip_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        for (level, data) in desc.mips.iter().take(mip_count as usize).enumerate() {
            let w = (desc.width >> level).max(1);
            let h = (desc.height >> level).max(1);
            let blocks_w = w.div_ceil(block);
            let blocks_h = h.div_ceil(block);
            let needed = (blocks_w * blocks_h * block_bytes) as usize;
            if data.len() < needed {
                log::warn!("{}: mip {level} is {} bytes, expected {needed}", desc.label, data.len());
                break;
            }
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: level as u32,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &data[..needed],
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(blocks_w * block_bytes),
                    rows_per_image: Some(blocks_h),
                },
                size.mip_level_size(level as u32, wgpu::TextureDimension::D2).physical_size(format),
            );
        }
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(desc.label),
            layout: &self.texture_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
            ],
        });
        self.textures.push(bind_group);
        TextureHandle(self.textures.len() - 1)
    }

    pub fn create_mesh(&mut self, desc: &MeshDesc<'_>) -> MeshHandle {
        let vertices = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(desc.label),
            contents: bytemuck::cast_slice(desc.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        // Index buffers must be a multiple of 4 bytes.
        let mut indices = desc.indices.to_vec();
        if indices.len() % 2 == 1 {
            indices.push(0);
        }
        let indices = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(desc.label),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        self.meshes.push(GpuMesh { vertices, indices, draws: desc.draws.clone() });
        MeshHandle(self.meshes.len() - 1)
    }

    /// Draw `meshes` and present. Returns `Ok(false)` when the frame was skipped
    /// (window minimised or the surface had to be reconfigured).
    pub fn render(&mut self, frame: &FrameParams, meshes: &[MeshHandle]) -> Result<bool, RenderError> {
        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => return Ok(false),
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return Ok(false);
            }
            #[allow(unreachable_patterns)]
            other => return Err(RenderError::Surface(format!("{other:?}"))),
        };
        let view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frame") });
        self.encode_scene(&mut encoder, &view, &self.depth, frame, meshes);
        self.queue.submit([encoder.finish()]);
        self.queue.present(surface_texture);
        Ok(true)
    }

    /// Render one frame off-screen and return it as tightly packed RGBA8.
    pub fn capture(
        &mut self,
        width: u32,
        height: u32,
        frame: &FrameParams,
        meshes: &[MeshHandle],
    ) -> Result<Vec<u8>, RenderError> {
        let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
        let target = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("capture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let depth = create_depth(&self.device, width, height);
        let row = (width * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("capture readback"),
            size: u64::from(row * height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder =
            self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("capture") });
        self.encode_scene(&mut encoder, &view, &depth, frame, meshes);
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(height),
                },
            },
            size,
        );
        self.queue.submit([encoder.finish()]);
        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        let mapped = slice.get_mapped_range().map_err(|e| RenderError::Device(format!("capture readback: {e:?}")))?;
        let bgra = matches!(self.config.format, wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb);
        let mut out = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height as usize {
            let line = &mapped[y * row as usize..][..width as usize * 4];
            for px in line.as_chunks::<4>().0 {
                if bgra {
                    out.extend_from_slice(&[px[2], px[1], px[0], 255]);
                } else {
                    out.extend_from_slice(&[px[0], px[1], px[2], 255]);
                }
            }
        }
        Ok(out)
    }

    fn encode_scene(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        frame: &FrameParams,
        meshes: &[MeshHandle],
    ) {
        let globals = Globals {
            view_proj: frame.view_proj.to_cols_array_2d(),
            light_dir: frame.light_dir.normalize_or_zero().extend(0.0).to_array(),
        };
        self.queue.write_buffer(&self.globals, 0, bytemuck::bytes_of(&globals));
        let [r, g, b] = frame.clear_color;
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("scene"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a: 1.0 }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_bind_group(0, &self.globals_bind_group, &[]);
        // Opaque and alpha-tested first, blended last.
        for pass_blend in [BlendMode::Opaque, BlendMode::AlphaTest, BlendMode::AlphaBlend] {
            pass.set_pipeline(match pass_blend {
                BlendMode::Opaque => &self.pipelines.opaque,
                BlendMode::AlphaTest => &self.pipelines.alpha_test,
                BlendMode::AlphaBlend => &self.pipelines.blend,
            });
            for handle in meshes {
                let Some(mesh) = self.meshes.get(handle.0) else { continue };
                pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint16);
                for d in mesh.draws.iter().filter(|d| d.blend == pass_blend) {
                    let tex = d.texture.map_or(0, |t| t.0);
                    pass.set_bind_group(1, &self.textures[tex], &[]);
                    pass.draw_indexed(d.first_index..d.first_index + d.index_count, 0, 0..1);
                }
            }
        }
    }
}

fn create_depth(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("depth"),
            size: wgpu::Extent3d { width: width.max(1), height: height.max(1), depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

fn create_pipelines(
    device: &wgpu::Device,
    color_format: wgpu::TextureFormat,
    globals_layout: &wgpu::BindGroupLayout,
    texture_layout: &wgpu::BindGroupLayout,
) -> Pipelines {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("placeholder shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("scene"),
        bind_group_layouts: &[Some(globals_layout), Some(texture_layout)],
        immediate_size: 0,
    });
    let attributes = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Unorm8x4, 3 => Float32x2];
    let vertex_layout = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Vertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &attributes,
    };

    let make = |label: &str, entry: &str, blend: Option<wgpu::BlendState>, depth_write: bool| {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(vertex_layout.clone())],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                // The game's winding/culling per effect is not mapped yet; draw both sides.
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(depth_write),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        })
    };

    Pipelines {
        opaque: make("opaque", "fs_opaque", None, true),
        alpha_test: make("alpha test", "fs_alpha_test", None, true),
        blend: make("alpha blend", "fs_blend", Some(wgpu::BlendState::ALPHA_BLENDING), false),
    }
}
