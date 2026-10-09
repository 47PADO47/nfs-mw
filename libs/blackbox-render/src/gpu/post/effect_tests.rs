//! Checks of the post effects: CPU tests (shader validation, parameter blocks, planning) and GPU
//! tests on a real backend (ignored without a GPU).

use super::bloom::{self, mip_sizes};
use super::filter::{BLOOM_WGSL, FXAA_WGSL, Params, TONEMAP_WGSL};
use super::tonemap::aces;
use super::{PassContext, PassIo, PostChain, PostPass, fxaa};
use crate::gpu::targets::{FrameTargets, HDR_FORMAT};
use crate::{Antialiasing, PostSettings, Tonemap};

const OUTPUT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

fn validate(name: &str, source: &str) -> wgpu::naga::Module {
    let module =
        wgpu::naga::front::wgsl::parse_str(source).unwrap_or_else(|e| panic!("{name}: {}", e.emit_to_string(source)));
    let mut validator = wgpu::naga::valid::Validator::new(
        wgpu::naga::valid::ValidationFlags::all(),
        wgpu::naga::valid::Capabilities::empty(),
    );
    validator.validate(&module).unwrap_or_else(|e| panic!("{name}: {}", e.emit_to_string(source)));
    module
}

#[test]
fn the_effect_shaders_validate_and_expose_their_entry_points() {
    for (name, source, entries) in [
        ("tonemap", TONEMAP_WGSL, &["vs_main", "fs_main"][..]),
        ("fxaa", FXAA_WGSL, &["vs_main", "fs_main"][..]),
        ("bloom", BLOOM_WGSL, &["vs_main", "fs_prefilter", "fs_down", "fs_up", "fs_composite"][..]),
    ] {
        let module = validate(name, source);
        for entry in entries {
            assert!(module.entry_points.iter().any(|e| e.name == *entry), "{name} lacks {entry}");
        }
        assert_eq!(module.entry_points.len(), entries.len(), "{name} has unexpected entry points");
    }
}

#[test]
fn every_effect_shader_binds_the_shared_filter_layout() {
    use wgpu::naga::{AddressSpace, ResourceBinding};
    for (name, source) in [("tonemap", TONEMAP_WGSL), ("fxaa", FXAA_WGSL), ("bloom", BLOOM_WGSL)] {
        let module = validate(name, source);
        let mut bindings: Vec<(u32, u32)> = module
            .global_variables
            .iter()
            .filter_map(|(_, g)| g.binding.as_ref().map(|ResourceBinding { group, binding }| (*group, *binding)))
            .collect();
        bindings.sort_unstable();
        assert_eq!(bindings, [(0, 0), (0, 1), (0, 2), (0, 3)], "{name}");
        let uniform = module.global_variables.iter().find(|(_, g)| g.space == AddressSpace::Uniform).unwrap().1;
        let size = module.types[uniform.ty].inner.size(module.to_ctx());
        assert_eq!(size as usize, std::mem::size_of::<Params>(), "{name}: the uniform block is two vec4");
    }
}

#[test]
fn the_parameter_block_is_two_vec4() {
    assert_eq!(std::mem::size_of::<Params>(), 32);
    assert_eq!(std::mem::align_of::<Params>(), 4);
    let bytes = bytemuck::bytes_of(&Params { a: [1.0, 2.0, 3.0, 4.0], b: [5.0, 6.0, 7.0, 8.0] });
    let floats: &[f32] = bytemuck::cast_slice(bytes);
    assert_eq!(floats, [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
}

#[test]
fn parameters_follow_the_settings() {
    let settings =
        PostSettings { exposure: 2.0, bloom_intensity: 1.0, bloom_threshold: 0.8, ..PostSettings::default() };
    assert_eq!(super::tonemap::params(&settings).a[0], 2.0);
    let bloom = bloom::params(&settings);
    assert_eq!(bloom.a[0], 0.8);
    assert!((bloom.a[1] - 0.4).abs() < 1e-6, "the knee is half the threshold");
    assert!(bloom.a[2] > 0.0 && bloom.a[2] < 1.0, "the intensity is scaled for the summed levels");
    let aa = fxaa::params();
    assert!(aa.a[0] > 0.0 && aa.a[1] > 0.0 && aa.a[2] > 0.0);
}

#[test]
fn the_filmic_curve_is_monotonic_and_bounded() {
    assert_eq!(aces(0.0), 0.0);
    assert!((aces(1.0) - 0.804).abs() < 0.001, "1.0 maps to about 0.80");
    assert_eq!(aces(1000.0), 1.0);
    assert_eq!(aces(-3.0), 0.0);
    let mut last = 0.0;
    for i in 0..400 {
        let y = aces(i as f32 * 0.025);
        assert!(y >= last && y <= 1.0);
        last = y;
    }
}

#[test]
fn the_bloom_chain_halves_down_to_a_few_pixels() {
    assert_eq!(mip_sizes((1280, 720)), [(640, 360), (320, 180), (160, 90), (80, 45), (40, 22), (20, 11)]);
    assert_eq!(mip_sizes((64, 64)), [(32, 32), (16, 16), (8, 8), (4, 4)]);
    assert_eq!(mip_sizes((1, 1)), [], "nothing to halve");
    assert_eq!(mip_sizes((2, 1)), [(1, 1)]);
    assert_eq!(mip_sizes((0, 0)), []);
    for size in [(7, 5), (3840, 2160), (100, 1)] {
        let sizes = mip_sizes(size);
        assert!(sizes.len() <= 6 && sizes.iter().all(|&(w, h)| w >= 1 && h >= 1), "{size:?}");
        assert_eq!(sizes[0], ((size.0 / 2).max(1), (size.1 / 2).max(1)));
    }
}

#[test]
#[ignore = "needs a Vulkan GPU"]
fn post_effects_vulkan() {
    check(wgpu::Backends::VULKAN);
}

#[cfg(target_os = "windows")]
#[test]
#[ignore = "needs a Direct3D 12 GPU"]
fn post_effects_dx12() {
    check(wgpu::Backends::DX12);
}

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

/// A pass that records its place in the chain, to check that inserted passes stay behind the effects.
struct Marker;

impl PostPass for Marker {
    fn name(&self) -> &'static str {
        "marker"
    }

    fn encode(&mut self, _: &PassContext<'_>, _: &PassIo<'_>, _: &mut wgpu::CommandEncoder) {}
}

impl Gpu {
    fn new(backend: wgpu::Backends) -> Self {
        let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
        desc.backends = backend;
        let instance = wgpu::Instance::new(desc);
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
            apply_limit_buckets: false,
        }))
        .expect("test GPU adapter");
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
        Self { device, queue }
    }

    /// Draw `body` (the statements of a fragment shader returning the colour at `pos`, in pixels)
    /// into `scene`'s colour image.
    fn paint(&self, scene: &FrameTargets, body: &str, encoder: &mut wgpu::CommandEncoder) {
        let source = format!(
            "@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {{
                 let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
                 return vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
             }}
             @fragment fn fs(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {{ {body} }}"
        );
        let module = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("test pattern"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("test pattern"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });
        let pipeline = self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("test pattern"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: HDR_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("test pattern"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &scene.color_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&pipeline);
        pass.draw(0..3, 0..1);
    }

    /// Paint a scene image of `size` with `body`, run `chain` into an output of the same size and read it back.
    fn run(&self, chain: &mut PostChain, size: (u32, u32), body: &str) -> Vec<[u8; 4]> {
        let device = &self.device;
        let scene = FrameTargets::new(device, HDR_FORMAT, size);
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("effect test output"),
            size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: OUTPUT_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        self.paint(&scene, body, &mut encoder);
        let ctx = PassContext { device, queue: &self.queue, scene: &scene, output_size: size };
        chain.encode(&ctx, &mut encoder, (&view, OUTPUT_FORMAT));
        let row = (size.0 * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("effect test readback"),
            size: u64::from(row * size.1),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(size.1),
                },
            },
            target.size(),
        );
        self.queue.submit([encoder.finish()]);
        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let mapped = slice.get_mapped_range().unwrap();
        let mut pixels = Vec::new();
        for y in 0..size.1 as usize {
            let line = &mapped[y * row as usize..][..size.0 as usize * 4];
            pixels.extend(line.as_chunks::<4>().0.iter().copied());
        }
        pixels
    }

    fn chain_with(&self, settings: PostSettings) -> PostChain {
        let mut chain = PostChain::new(&self.device);
        chain.set_effects(&self.device, settings);
        chain
    }
}

fn byte(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn near(pixel: [u8; 4], expected: [u8; 4], tolerance: u8) -> bool {
    pixel.iter().zip(expected).all(|(&a, b)| a.abs_diff(b) <= tolerance)
}

/// Fraction of pixels whose red channel is neither black nor white.
fn grey_pixels(pixels: &[[u8; 4]]) -> usize {
    pixels.iter().filter(|p| p[0] > 12 && p[0] < 243).count()
}

fn check(backend: wgpu::Backends) {
    let gpu = Gpu::new(backend);
    let size = (64, 64);
    let flat = "return vec4<f32>(1.5, 0.5, 0.25, 0.3);";

    // The default settings add no pass: only the resolve pass runs, and it clamps.
    let mut chain = gpu.chain_with(PostSettings::default());
    assert_eq!(chain.names(), ["resolve"]);
    let plain = gpu.run(&mut chain, size, flat);
    assert!(plain.iter().all(|&p| near(p, [255, 128, 64, 255], 1)), "{:?}", plain[0]);

    // Tone mapping: exposure, then the curve, per channel.
    for exposure in [1.0, 2.0, 0.5] {
        let settings = PostSettings { tonemap: Tonemap::Aces, exposure, ..PostSettings::default() };
        let mut chain = gpu.chain_with(settings);
        assert_eq!(chain.names(), ["tonemap", "resolve"]);
        let expected = [byte(aces(1.5 * exposure)), byte(aces(0.5 * exposure)), byte(aces(0.25 * exposure)), 255];
        let mapped = gpu.run(&mut chain, size, flat);
        assert!(mapped.iter().all(|&p| near(p, expected, 2)), "exposure {exposure}: {:?} vs {expected:?}", mapped[0]);
    }

    // Bloom: a bright square spills light onto its black surroundings; below the threshold nothing changes.
    let square = "let on = abs(pos.x - 32.0) < 6.0 && abs(pos.y - 32.0) < 6.0;
                  return select(vec4<f32>(0.0, 0.0, 0.0, 1.0), vec4<f32>(4.0, 4.0, 4.0, 1.0), on);";
    let settings = PostSettings { bloom_intensity: 1.0, ..PostSettings::default() };
    let mut chain = gpu.chain_with(settings);
    assert_eq!(chain.names(), ["bloom", "resolve"]);
    let image = gpu.run(&mut chain, size, square);
    let at = |x: usize, y: usize| image[y * 64 + x];
    assert_eq!(at(32, 32)[0], 255, "the square stays white");
    assert!(at(42, 32)[0] > 8, "light spills next to the square: {:?}", at(42, 32));
    assert!(at(42, 32)[0] > at(52, 32)[0], "and fades with distance: {:?} {:?}", at(42, 32), at(52, 32));
    assert!(at(2, 2)[0] < at(42, 32)[0], "the far corner gets the least");
    let dim = "return vec4<f32>(0.4, 0.4, 0.4, 1.0);";
    let unlit = gpu.run(&mut chain, size, dim);
    assert!(unlit.iter().all(|&p| near(p, [102, 102, 102, 255], 3)), "below the threshold: {:?}", unlit[0]);
    let off = gpu.run(&mut gpu.chain_with(PostSettings { bloom_intensity: 0.0, ..settings }), size, square);
    assert!(at(42, 32)[0] > off[32 * 64 + 42][0], "bloom off leaves the surroundings black");

    // FXAA: a slanted black/white edge gets intermediate greys; flat areas are untouched.
    let slanted = "let on = pos.x + pos.y * 0.37 < 30.0;
                   return select(vec4<f32>(0.0, 0.0, 0.0, 1.0), vec4<f32>(1.0, 1.0, 1.0, 1.0), on);";
    let aliased = gpu.run(&mut gpu.chain_with(PostSettings::default()), size, slanted);
    let settings = PostSettings { antialiasing: Antialiasing::Fxaa, ..PostSettings::default() };
    let mut chain = gpu.chain_with(settings);
    assert_eq!(chain.names(), ["fxaa", "resolve"]);
    let smoothed = gpu.run(&mut chain, size, slanted);
    assert_eq!(grey_pixels(&aliased), 0, "the source has hard edges only");
    assert!(grey_pixels(&smoothed) >= 40, "FXAA smooths the edge: {}", grey_pixels(&smoothed));
    assert_eq!(smoothed[2 * 64 + 2], [255, 255, 255, 255], "far from the edge stays white");
    assert_eq!(smoothed[60 * 64 + 60], [0, 0, 0, 255], "far from the edge stays black");
    let still = gpu.run(&mut chain, size, flat);
    assert!(still.iter().all(|&p| near(p, [255, 128, 64, 255], 1)), "a flat image is unchanged: {:?}", still[0]);

    // The whole stack, in order; inserted passes stay behind the effects, and clearing restores the plain chain.
    let all = PostSettings {
        tonemap: Tonemap::Aces,
        bloom_intensity: 0.5,
        antialiasing: Antialiasing::Fxaa,
        ..PostSettings::default()
    };
    let mut chain = gpu.chain_with(all);
    assert_eq!(chain.names(), ["bloom", "tonemap", "fxaa", "resolve"]);
    chain.insert(0, Box::new(Marker));
    assert_eq!(chain.names(), ["bloom", "tonemap", "fxaa", "marker", "resolve"]);
    let stacked = gpu.run(&mut chain, size, square);
    assert_eq!(stacked.len(), 64 * 64);
    assert!(chain.set_effects(&gpu.device, PostSettings { tonemap: Tonemap::Off, ..all }));
    assert_eq!(chain.names(), ["bloom", "fxaa", "marker", "resolve"]);
    assert!(!chain.set_effects(&gpu.device, PostSettings { tonemap: Tonemap::Off, ..all }), "same settings, no change");
    assert!(chain.set_effects(&gpu.device, PostSettings::default()));
    assert_eq!(chain.names(), ["marker", "resolve"]);
}
