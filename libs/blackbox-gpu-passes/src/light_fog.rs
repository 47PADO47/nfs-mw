//! A shared half-resolution local-light fog pass, clipped and upsampled using scene depth.
//!
//! Every view keeps its own volume, image and history ([`FogView`]), so a second view (a split-screen half, an
//! off-screen capture) neither reallocates the main view's buffers nor reprojects another camera's cells.
use std::collections::HashMap;

use blackbox_gfx::LocalLight;

use crate::{SoftDraw, WorldBindings, world::write_mask};

mod grid;
mod params;
mod view;

use params::{Inputs, MAX_LIGHTS, MAX_SHELTERS, Params};
use view::{Frame, ViewFog};

/// Frames without lit fog after which a view's buffers are released.
const RELEASE_AFTER: u64 = 120;
/// Views without a stable identity kept at once.
const UNKEYED_VIEWS: usize = 3;

/// Which view a light-fog draw belongs to; each keeps its own volume, image and temporal history.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FogView {
    /// The main view.
    Main,
    /// A view with a stable caller-chosen identity, such as one split-screen player.
    Slot(u32),
    /// A view without a stable identity (an off-screen capture). It reuses the state of the same size whose last
    /// camera is nearest, so alternating cameras each find their own history.
    Unkeyed,
}

/// The pipelines and layouts, on devices that can run the compute volume.
struct Pipelines {
    shader: wgpu::ShaderModule,
    layout: wgpu::PipelineLayout,
    input_layout: wgpu::BindGroupLayout,
    texture_layout: wgpu::BindGroupLayout,
    integrate: wgpu::RenderPipeline,
    grid: grid::Grid,
    composite: HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,
    format: wgpu::TextureFormat,
}

pub struct LightFog {
    /// `None` on devices without compute shaders or enough storage buffers: nothing is drawn.
    gpu: Option<Pipelines>,
    views: Vec<ViewFog>,
    lights: Vec<LocalLight>,
    source_lights: Vec<LocalLight>,
    shelters: Vec<blackbox_gfx::FogShelter>,
    selection: crate::fog_lights::FogLights,
    last_upload: Option<std::time::Instant>,
    options: [f32; 2],
    distance: f32,
    quality: u32,
    /// Counts uploads: one per frame.
    frame: u64,
    cache: Option<wgpu::PipelineCache>,
}

impl LightFog {
    pub fn new(device: &wgpu::Device, bindings: &WorldBindings, cache: Option<&wgpu::PipelineCache>) -> Self {
        Self {
            gpu: grid::Grid::supported(device).then(|| Pipelines::new(device, bindings, cache)),
            views: Vec::new(),
            lights: Vec::new(),
            source_lights: Vec::new(),
            shelters: Vec::new(),
            selection: Default::default(),
            last_upload: None,
            options: [0.0; 2],
            distance: 200.0,
            quality: 2,
            frame: 0,
            cache: cache.cloned(),
        }
    }

    pub fn use_format(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat) {
        let Some(gpu) = &mut self.gpu else { return };
        gpu.use_format(device, format, self.cache.as_ref());
    }

    pub fn upload(
        &mut self,
        options: [f32; 2],
        shelters: &[blackbox_gfx::FogShelter],
        lights: &[LocalLight],
        limit: usize,
        distance: f32,
        quality: u32,
    ) {
        self.frame += 1;
        self.options = options;
        self.shelters.clear();
        self.shelters.extend(shelters.iter().take(MAX_SHELTERS).cloned());
        self.source_lights.clear();
        self.source_lights.extend(lights.iter().take(16).copied());
        self.distance = distance.clamp(80.0, 600.0);
        self.quality = quality.clamp(1, 3);
        let now = std::time::Instant::now();
        let dt = self.last_upload.replace(now).map_or(0.0, |then| now.duration_since(then).as_secs_f32());
        let enabled = options[0] > 0.0 && limit > 0;
        let limit = match enabled {
            true => limit,
            false => 0,
        };
        self.lights = self.selection.update(lights, limit, dt);
        self.lights.truncate(MAX_LIGHTS);
        if !enabled {
            // Switched off: release every view's buffers now.
            self.views.clear();
            return;
        }
        if self.lights.is_empty() {
            self.views.iter_mut().for_each(ViewFog::forget_history);
        }
        // A view not drawn for a while (no lit fog, or the view went away) releases its buffers.
        let frame = self.frame;
        self.views.retain(|view| frame.saturating_sub(view.last_frame) <= RELEASE_AFTER);
    }

    /// Record the fog of `view` into `draw.target`. Does nothing while the fog is off or has no lights, and on
    /// devices that cannot run it.
    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        draw: &SoftDraw<'_>,
        view: FogView,
    ) {
        let Some(gpu) = &mut self.gpu else { return };
        if self.options[0] <= 0.0 || self.lights.is_empty() {
            return;
        }
        let size = draw.depth.texture().size();
        let size = (size.width, size.height);
        let dimensions = grid::Grid::dimensions(size.0, size.1, self.quality);
        let index = Self::view_index(&mut self.views, device, view, size, draw.camera_position);
        let state = &mut self.views[index];
        state.prepare(device, gpu, size, dimensions);
        let frame = Frame {
            number: self.frame,
            camera: draw.camera_position,
            camera_cut: draw.camera_cut,
            options: self.options,
            distance: self.distance,
        };
        let params = Params::new(&Inputs {
            inverse_view_proj: draw.inverse_view_proj,
            options: self.options,
            distance: self.distance,
            dimensions,
            lights: &self.lights,
            source_lights: &self.source_lights,
            shelters: &self.shelters,
            previous: state.previous(&frame),
        });
        state.encode((device, queue), gpu, encoder, draw, (&params, &frame));
    }

    /// The state `view` draws with: its own, the nearest unkeyed one of the same size, or a new one.
    fn view_index(
        views: &mut Vec<ViewFog>,
        device: &wgpu::Device,
        key: FogView,
        size: (u32, u32),
        camera: glam::Vec3,
    ) -> usize {
        if key != FogView::Unkeyed {
            if let Some(index) = views.iter().position(|view| view.key == key) {
                return index;
            }
            views.push(ViewFog::new(device, key, size));
            return views.len() - 1;
        }
        let nearest = views
            .iter()
            .enumerate()
            .filter(|(_, view)| view.key == key && view.size == size)
            .filter_map(|(i, view)| Some((i, view.camera()?.distance(camera))))
            .filter(|(_, distance)| *distance < 2.0 * view::HISTORY_DISTANCE)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((index, _)) = nearest {
            return index;
        }
        let unkeyed = views.iter().filter(|view| view.key == key).count();
        if unkeyed < UNKEYED_VIEWS {
            views.push(ViewFog::new(device, key, size));
            return views.len() - 1;
        }
        // Reuse the least recently drawn unkeyed state, preferring one of the same size.
        let index = views
            .iter()
            .enumerate()
            .filter(|(_, view)| view.key == key)
            .min_by_key(|(_, view)| (view.size != size, view.last_frame))
            .map(|(i, _)| i)
            .unwrap();
        views[index].forget_history();
        index
    }

    /// Bytes of GPU memory held by every view's volume, image and parameters.
    pub fn allocated_bytes(&self) -> u64 {
        self.views.iter().map(ViewFog::bytes).sum()
    }

    /// Whether the device can draw the fog at all.
    pub fn supported(&self) -> bool {
        self.gpu.is_some()
    }
}

impl Pipelines {
    fn new(device: &wgpu::Device, bindings: &WorldBindings, cache: Option<&wgpu::PipelineCache>) -> Self {
        let input_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("light fog inputs"),
            entries: &[
                fragment_entry(
                    0,
                    wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
                fragment_entry(
                    1,
                    wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                fragment_entry(
                    2,
                    wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
            ],
        });
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("light fog image"),
            entries: &[fragment_entry(
                0,
                wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
            )],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("light fog"),
            bind_group_layouts: &[Some(&bindings.globals_layout), Some(&input_layout), Some(&texture_layout)],
            immediate_size: 0,
        });
        let integrate_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("light fog integration"),
            bind_group_layouts: &[Some(&bindings.globals_layout), Some(&input_layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("light fog"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/light_fog.wgsl").into()),
        });
        let integrate =
            pipeline(device, (&integrate_layout, &shader), wgpu::TextureFormat::Rgba16Float, "fs_fog", false, cache);
        Self {
            shader,
            layout,
            input_layout,
            texture_layout,
            integrate,
            grid: grid::Grid::new(device, bindings, cache),
            composite: HashMap::new(),
            format: crate::HDR_FORMAT,
        }
    }

    fn use_format(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat, cache: Option<&wgpu::PipelineCache>) {
        self.format = format;
        if self.composite.contains_key(&format) {
            return;
        }
        let entry = match crate::world::is_srgb(format) {
            true => "fs_composite_srgb",
            false => "fs_composite",
        };
        let composite = pipeline(device, (&self.layout, &self.shader), format, entry, true, cache);
        self.composite.insert(format, composite);
    }
}

fn fragment_entry(binding: u32, ty: wgpu::BindingType) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry { binding, visibility: wgpu::ShaderStages::FRAGMENT, ty, count: None }
}

fn pipeline(
    device: &wgpu::Device,
    (layout, shader): (&wgpu::PipelineLayout, &wgpu::ShaderModule),
    format: wgpu::TextureFormat,
    entry: &str,
    additive: bool,
    cache: Option<&wgpu::PipelineCache>,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("light fog"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(entry),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: additive.then_some(wgpu::BlendState {
                    // Scene weather already supplies its own fog. This pass adds only the local lights scattered
                    // by the medium, not another global extinction: the composite's alpha (the scene distance the
                    // half-resolution image carries) never reaches the target.
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::One,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::Zero,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    },
                }),
                write_mask: write_mask(format),
            })],
        }),
        multiview_mask: None,
        cache,
    })
}

#[cfg(test)]
#[path = "light_fog_tests.rs"]
mod tests;
