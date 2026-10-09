//! GPU checks of the FSR 1 passes on a real backend (no surface or assets).

use super::tests::{Gpu, near};
use super::{PassContext, PassIo, PostChain, PostPass, RcasScale, fsr1_passes};

#[test]
#[ignore = "needs a Vulkan GPU"]
fn fsr1_vulkan() {
    check(wgpu::Backends::VULKAN);
}

#[cfg(target_os = "windows")]
#[test]
#[ignore = "needs a Direct3D 12 GPU"]
fn fsr1_dx12() {
    check(wgpu::Backends::DX12);
}

const PATTERN: &str = "
@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let p = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
}

// A vertical edge: dark on the left half of the image, light on the right.
@fragment
fn fs_main(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let v = select(0.75, 0.25, frag.x < 4.0);
    return vec4<f32>(v, v, v, 1.0);
}
";

/// Draws a vertical edge into its output, ignoring its input.
struct Pattern(Option<wgpu::RenderPipeline>);

impl PostPass for Pattern {
    fn name(&self) -> &'static str {
        "pattern"
    }

    fn encode(&mut self, ctx: &PassContext<'_>, io: &PassIo<'_>, encoder: &mut wgpu::CommandEncoder) {
        let device = ctx.device;
        let pipeline = self.0.get_or_insert_with(|| {
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("pattern"),
                source: wgpu::ShaderSource::Wgsl(PATTERN.into()),
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("pattern"),
                layout: None,
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: io.output_format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("pattern"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: io.output,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(pipeline);
        pass.draw(0..3, 0..1);
    }
}

/// An 8x8 edge image upscaled to 16x16 through `chain`, as rows of the red channel.
fn upscale(gpu: &Gpu, chain: &mut PostChain) -> Vec<Vec<u8>> {
    let black = wgpu::Color::BLACK;
    let pixels = gpu.run(chain, (8, 8), black, (16, 16));
    pixels.chunks(16).map(|row| row.iter().map(|p| p[0]).collect()).collect()
}

fn check(backend: wgpu::Backends) {
    let _gpu = crate::gpu::test_support::serial();
    let gpu = Gpu::new(backend);
    let rcas = RcasScale::new();
    let (dark, light) = (64u8, 191u8);

    // EASU alone: flat areas stay exactly flat, the edge stays monotonic and never overshoots.
    let mut chain = PostChain::new(&gpu.device);
    chain.insert(0, Box::new(Pattern(None)));
    for pass in fsr1_passes(&gpu.device, &rcas, false) {
        chain.insert(usize::MAX, pass);
    }
    assert_eq!(chain.names(), ["pattern", "fsr1-easu", "resolve"]);
    let rows = upscale(&gpu, &mut chain);
    let first = &rows[0];
    assert!(rows.iter().all(|r| r == first), "a vertical edge looks the same on every row: {rows:?}");
    assert!(near([first[0], 0, 0, 0], [dark, 0, 0, 0]) && near([first[15], 0, 0, 0], [light, 0, 0, 0]), "{first:?}");
    assert!(first.windows(2).all(|w| w[0] <= w[1]), "no ringing without RCAS: {first:?}");
    assert!(first.iter().all(|&v| (dark - 1..=light + 1).contains(&v)), "{first:?}");
    assert!(first[7] < first[8], "the edge is across the middle: {first:?}");

    // With RCAS the edge is sharper: it overshoots the two levels, and the flat areas are untouched.
    rcas.set_stops(0.0);
    let mut chain = PostChain::new(&gpu.device);
    chain.insert(0, Box::new(Pattern(None)));
    for pass in fsr1_passes(&gpu.device, &rcas, true) {
        chain.insert(usize::MAX, pass);
    }
    assert_eq!(chain.names(), ["pattern", "fsr1-easu", "fsr1-rcas", "resolve"]);
    let sharp = &upscale(&gpu, &mut chain)[0];
    assert!(near([sharp[0], 0, 0, 0], [dark, 0, 0, 0]) && near([sharp[15], 0, 0, 0], [light, 0, 0, 0]), "{sharp:?}");
    assert!(sharp[1..7].iter().any(|&v| v < dark) || sharp[9..15].iter().any(|&v| v > light), "{sharp:?}");
}
