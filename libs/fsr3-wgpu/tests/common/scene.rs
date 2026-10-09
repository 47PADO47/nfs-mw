//! A procedural scene rendered at the render resolution with jitter, depth and motion vectors, and the
//! same scene supersampled at the output resolution as the reference to compare the upscaler with.

use bytemuck::{Pod, Zeroable};
use fsr3_wgpu::{
    DebugTexture, DepthConvention, Fsr3Config, Fsr3Context, Fsr3Inputs, Fsr3Outputs, MOTION_VECTOR_SCALE_PIXELS, jitter,
};
use glam::{UVec2, Vec2};
use wgpu::util::DeviceExt;

use super::{Gpu, scene_shader::SHADER};

pub const NEAR: f32 = 0.1;
pub const FAR: f32 = 1000.0;
pub const FOV_Y: f32 = 1.0;

const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const MOTION_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rg16Float;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const REFERENCE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba32Float;
/// Subsamples per axis and output pixel of the reference.
const REFERENCE_SAMPLES: f32 = 8.0;

/// What the scene shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Hard edges, 1.5 px lines and a fine checker: much more detail than a lower resolution can hold.
    Static = 0,
    /// A checkerboard that moves with the velocity.
    Checker = 1,
    /// A static background far away and a moving square close to the camera.
    Layers = 2,
    /// The inverse of `Static`, for a scene change.
    Inverted = 3,
    /// Band-limited content: soft edges, a grating and a soft line.
    Smooth = 4,
}

#[derive(Clone, Copy, Debug)]
pub struct SceneParams {
    pub kind: Kind,
    pub time: f32,
    /// Display pixels per frame of the moving content.
    pub velocity: Vec2,
    pub gain: f32,
}

impl SceneParams {
    pub fn new(kind: Kind) -> Self {
        Self { kind, time: 0.0, velocity: Vec2::ZERO, gain: 1.0 }
    }

    pub fn at(self, time: u32) -> Self {
        Self { time: time as f32, ..self }
    }

    pub fn moving(self, velocity: Vec2) -> Self {
        Self { velocity, ..self }
    }

    pub fn gain(self, gain: f32) -> Self {
        Self { gain, ..self }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SceneUniform {
    render_size: [f32; 2],
    display_size: [f32; 2],
    jitter: [f32; 2],
    velocity: [f32; 2],
    time: f32,
    kind: f32,
    near: f32,
    far: f32,
    depth_mode: f32,
    samples: f32,
    gain: f32,
    motion_gain: f32,
    mv_offset: [f32; 2],
    pad: [f32; 2],
}

struct Targets {
    color: wgpu::Texture,
    motion: wgpu::Texture,
    /// The motion vectors at the output resolution, when the configuration asks for them.
    motion_display: wgpu::Texture,
    depth: wgpu::Texture,
}

/// The scene renderer and an upscaler on one device.
pub struct Rig {
    pub gpu: Gpu,
    pub ctx: Fsr3Context,
    pub render: UVec2,
    pub display: UVec2,
    /// How the scene encodes depth (the upscaler is told `ctx.config().depth`).
    pub depth: DepthConvention,
    /// Frames dispatched so far.
    pub frame: u32,
    pub sharpness: Option<f32>,
    pub pre_exposure: f32,
    /// The value of the exposure input texture, if one is given.
    pub exposure: Option<f32>,
    /// A reactive mask filled with this value, if one is given.
    pub reactive: Option<f32>,
    /// A transparency and composition mask filled with this value, if one is given.
    pub transparency: Option<f32>,
    /// Scales the motion vectors the scene writes: 1 is correct, 0 pretends nothing moves.
    pub motion_gain: f32,
    /// Writes jittered motion vectors: the true vectors plus the previous minus the current jitter.
    pub jittered_motion_vectors: bool,
    previous_jitter: Vec2,
    targets: Targets,
    output: wgpu::Texture,
    reference: wgpu::Texture,
    scene_pipeline: wgpu::RenderPipeline,
    motion_pipeline: wgpu::RenderPipeline,
    reference_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform: wgpu::Buffer,
}

fn texture(
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

fn view(texture: &wgpu::Texture) -> wgpu::TextureView {
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

/// The textures that depend on the render and output size.
fn create_images(
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

fn pipeline(
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

fn target(format: wgpu::TextureFormat) -> Option<wgpu::ColorTargetState> {
    Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })
}

fn color_attachment(view: &wgpu::TextureView) -> Option<wgpu::RenderPassColorAttachment<'_>> {
    Some(wgpu::RenderPassColorAttachment {
        view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
    })
}

impl Rig {
    /// A rig whose upscaler has `config`.
    pub fn new(config: Fsr3Config, render: UVec2, display: UVec2) -> Self {
        let gpu = Gpu::new();
        let depth = config.depth;
        let device = &gpu.device;
        let ctx = Fsr3Context::new(device, config.clone()).expect("a context");
        let (targets, output, reference) = create_images(device, render, display, config.output_format);

        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fsr3 test scene"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&bind_layout)],
            immediate_size: 0,
        });
        let scene_pipeline =
            pipeline(device, &module, &layout, "fs_scene", &[target(COLOR_FORMAT), target(MOTION_FORMAT)], true);
        let motion_pipeline = pipeline(device, &module, &layout, "fs_motion", &[target(MOTION_FORMAT)], false);
        let reference_pipeline = pipeline(device, &module, &layout, "fs_reference", &[target(REFERENCE_FORMAT)], false);
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("scene"),
            contents: &[0; size_of::<SceneUniform>()],
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: uniform.as_entire_binding() }],
        });
        Self {
            gpu,
            ctx,
            render,
            display,
            depth,
            frame: 0,
            sharpness: None,
            pre_exposure: 0.0,
            exposure: None,
            reactive: None,
            transparency: None,
            motion_gain: 1.0,
            jittered_motion_vectors: false,
            previous_jitter: Vec2::ZERO,
            targets,
            output,
            reference,
            scene_pipeline,
            motion_pipeline,
            reference_pipeline,
            bind_group,
            uniform,
        }
    }

    /// An `R32Float` texture of `size` filled with `value`.
    fn constant_texture(&self, size: UVec2, value: f32) -> wgpu::TextureView {
        let data = vec![value; (size.x * size.y) as usize];
        let texture = self.gpu.device.create_texture_with_data(
            &self.gpu.queue,
            &wgpu::TextureDescriptor {
                label: Some("constant"),
                size: wgpu::Extent3d { width: size.x, height: size.y, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R32Float,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            bytemuck::cast_slice(&data),
        );
        view(&texture)
    }

    /// Switches to other render and output sizes.
    pub fn resize(&mut self, render: UVec2, display: UVec2) {
        let (targets, output, reference) =
            create_images(&self.gpu.device, render, display, self.ctx.config().output_format);
        (self.targets, self.output, self.reference) = (targets, output, reference);
        (self.render, self.display) = (render, display);
    }

    fn depth_mode(&self) -> f32 {
        match (self.depth.inverted, self.depth.infinite) {
            (false, false) => 0.0,
            (true, true) => 1.0,
            (true, false) => 2.0,
            (false, true) => 3.0,
        }
    }

    fn set_scene(&self, p: &SceneParams, jitter: Vec2, mv_offset: Vec2) {
        let uniform = SceneUniform {
            render_size: self.render.as_vec2().to_array(),
            display_size: self.display.as_vec2().to_array(),
            jitter: jitter.to_array(),
            velocity: p.velocity.to_array(),
            time: p.time,
            kind: p.kind as u32 as f32,
            near: NEAR,
            far: FAR,
            depth_mode: self.depth_mode(),
            samples: REFERENCE_SAMPLES,
            gain: p.gain,
            motion_gain: self.motion_gain,
            mv_offset: mv_offset.to_array(),
            pad: [0.0; 2],
        };
        self.gpu.queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&uniform));
    }

    /// Draws the scene at the render resolution, shifted by `jitter` pixels, and its motion vectors at
    /// the resolution the upscaler is configured for.
    pub fn draw_inputs(&self, encoder: &mut wgpu::CommandEncoder, p: &SceneParams, jitter: Vec2) {
        // Jittered vectors point from the previous jittered position to the current one.
        let offset = if self.jittered_motion_vectors { self.previous_jitter - jitter } else { Vec2::ZERO };
        self.set_scene(p, jitter, offset);
        let color = view(&self.targets.color);
        let motion = view(&self.targets.motion);
        let depth = view(&self.targets.depth);
        let clear_depth = if self.depth.inverted { 0.0 } else { 1.0 };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("fsr3 test scene"),
            color_attachments: &[color_attachment(&color), color_attachment(&motion)],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear_depth),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.scene_pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
        drop(pass);

        if !self.ctx.config().motion_vectors.display_resolution {
            return;
        }
        let display_motion = view(&self.targets.motion_display);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("fsr3 test display motion"),
            color_attachments: &[color_attachment(&display_motion)],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.motion_pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }

    /// Renders the next jittered frame of the scene and upscales it.
    pub fn frame(&mut self, params: SceneParams, reset: bool) {
        let jitter = jitter::offset_for_sizes(self.frame, self.render, self.display);
        self.frame_with_jitter(params, jitter, reset);
    }

    pub fn frame_with_jitter(&mut self, params: SceneParams, jitter: Vec2, reset: bool) {
        let mut encoder = self.gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        self.draw_inputs(&mut encoder, &params, jitter);
        let (color, depth) = (view(&self.targets.color), view(&self.targets.depth));
        let display_vectors = self.ctx.config().motion_vectors.display_resolution;
        let motion = view(if display_vectors { &self.targets.motion_display } else { &self.targets.motion });
        let output = view(&self.output);
        let exposure = self.exposure.map(|value| self.constant_texture(UVec2::ONE, value));
        let reactive = self.reactive.map(|value| self.constant_texture(self.render, value));
        let transparency = self.transparency.map(|value| self.constant_texture(self.render, value));
        let inputs = Fsr3Inputs {
            color: &color,
            depth: &depth,
            motion_vectors: &motion,
            exposure: exposure.as_ref(),
            reactive: reactive.as_ref(),
            transparency_and_composition: transparency.as_ref(),
            render_size: self.render,
            jitter,
            motion_vector_scale: MOTION_VECTOR_SCALE_PIXELS,
            delta_time: 1.0 / 60.0,
            pre_exposure: self.pre_exposure,
            sharpness: self.sharpness,
            camera_near: NEAR,
            camera_far: FAR,
            camera_fov_y: FOV_Y,
            view_space_to_meters: 1.0,
            reset,
        };
        self.ctx
            .dispatch(
                &self.gpu.device,
                &self.gpu.queue,
                &mut encoder,
                &inputs,
                &Fsr3Outputs { output: &output, size: self.display },
            )
            .expect("a dispatch");
        self.gpu.queue.submit([encoder.finish()]);
        self.previous_jitter = jitter;
        self.frame += 1;
    }

    /// The upscaled image of an `Rgba32Float` output.
    pub fn output_image(&self) -> Vec<[f32; 4]> {
        self.gpu.read_rgba32f(&self.output, self.display)
    }

    /// The upscaled image of an `Rgba16Float` output.
    pub fn output_image_f16(&self) -> Vec<[f32; 4]> {
        self.gpu.read_rgba16f(&self.output, self.display)
    }

    /// The upscaled image of an `Rgba8Unorm` output.
    pub fn output_image_u8(&self) -> Vec<[f32; 4]> {
        let bytes = self.gpu.read_bytes(&self.output, self.display, 4);
        bytes.chunks(4).map(|t| [t[0], t[1], t[2], t[3]].map(|v| f32::from(v) / 255.0)).collect()
    }

    /// The jitter-free render-resolution colour of the scene.
    pub fn plain_render(&self, params: &SceneParams) -> Vec<[f32; 4]> {
        let mut encoder = self.gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        self.draw_inputs(&mut encoder, params, Vec2::ZERO);
        self.gpu.queue.submit([encoder.finish()]);
        self.gpu.read_rgba16f(&self.targets.color, self.render)
    }

    /// The scene at `params`, supersampled at the output resolution.
    pub fn reference_image(&self, params: &SceneParams) -> Vec<[f32; 4]> {
        self.set_scene(params, Vec2::ZERO, Vec2::ZERO);
        let target = view(&self.reference);
        let mut encoder = self.gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("fsr3 test reference"),
                color_attachments: &[color_attachment(&target)],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.reference_pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.gpu.queue.submit([encoder.finish()]);
        self.gpu.read_rgba32f(&self.reference, self.display)
    }

    /// Reads an internal image of the upscaler as floats (`R32Float`, `Rg32Float` or `Rgba16Float`):
    /// its size, the channels per pixel and the values.
    pub fn read_debug(&self, which: DebugTexture) -> (UVec2, usize, Vec<f32>) {
        let texture = self.ctx.debug_texture(which).expect("a dispatched context");
        let size = UVec2::new(texture.width(), texture.height());
        let (texel, channels) = match texture.format() {
            wgpu::TextureFormat::R32Float => (4, 1),
            wgpu::TextureFormat::Rg32Float => (8, 2),
            wgpu::TextureFormat::Rgba16Float => (8, 4),
            other => panic!("unexpected debug format {other:?}"),
        };
        let bytes = self.gpu.read_bytes(texture, size, texel);
        let values = if channels == 4 {
            bytemuck::cast_slice::<u8, u16>(&bytes).iter().map(|h| super::decode_f16(*h)).collect()
        } else {
            bytemuck::cast_slice::<u8, f32>(&bytes).to_vec()
        };
        (size, channels, values)
    }
}
