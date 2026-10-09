//! What is built and allocated when the optional costs are off (GPU checks on a real backend):
//! the pass list, the post chain's images and the scene pipelines.

use super::post::{PassContext, PostChain};
use super::resources::Shared;
use super::targets::{FrameTargets, HDR_FORMAT};
use crate::{Antialiasing, PostSettings, Tonemap};

#[test]
#[ignore = "needs a Vulkan GPU"]
fn lean_configuration_vulkan() {
    check(wgpu::Backends::VULKAN);
}

#[cfg(target_os = "windows")]
#[test]
#[ignore = "needs a Direct3D 12 GPU"]
fn lean_configuration_dx12() {
    check(wgpu::Backends::DX12);
}

fn gpu(backend: wgpu::Backends) -> (wgpu::Device, wgpu::Queue) {
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
    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap()
}

/// Run `chain` once over a fresh scene image of `size` into an output of the same size.
fn run(device: &wgpu::Device, queue: &wgpu::Queue, chain: &mut PostChain, size: (u32, u32)) {
    let scene = FrameTargets::new(device, HDR_FORMAT, size);
    let output = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("lean test output"),
        size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = output.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let ctx = PassContext { device, queue, scene: &scene, output_size: size };
    chain.encode(&ctx, &mut encoder, (&view, wgpu::TextureFormat::Rgba8Unorm));
    queue.submit([encoder.finish()]);
}

fn check(backend: wgpu::Backends) {
    let _gpu = crate::gpu::test_support::serial();
    let (device, queue) = gpu(backend);
    let size = (32, 32);

    // Everything off: the chain is the resolve pass alone and allocates no ping-pong image.
    let mut chain = PostChain::new(&device);
    assert_eq!(chain.names(), ["resolve"]);
    assert!(!chain.has_passes());
    run(&device, &queue, &mut chain, size);
    assert_eq!(chain.scratch_count(), 0, "a single pass reads the scene and writes the output");
    assert!(!chain.set_effects(&device, PostSettings::default()), "the defaults add nothing");
    assert_eq!(chain.names(), ["resolve"]);

    // Every effect: three render-size passes need both ping-pong images.
    let all = PostSettings {
        tonemap: Tonemap::Aces,
        bloom_intensity: 0.5,
        antialiasing: Antialiasing::Fxaa,
        ..PostSettings::default()
    };
    assert!(chain.set_effects(&device, all));
    assert!(chain.has_passes());
    run(&device, &queue, &mut chain, size);
    assert_eq!(chain.scratch_count(), 2);

    // Dropping to one effect frees the second image on the next frame, and dropping them all frees both.
    let fxaa = PostSettings { antialiasing: Antialiasing::Fxaa, ..PostSettings::default() };
    assert!(chain.set_effects(&device, fxaa));
    run(&device, &queue, &mut chain, size);
    assert_eq!(chain.scratch_count(), 1);
    assert!(chain.set_effects(&device, PostSettings::default()));
    run(&device, &queue, &mut chain, size);
    assert_eq!(chain.names(), ["resolve"]);
    assert_eq!(chain.scratch_count(), 0, "the images of removed passes do not linger");

    // The scene pipelines: a format gets its own set when first used, glossy ones only on request.
    let shared = Shared::new(&device);
    let (surface, hdr) = (wgpu::TextureFormat::Rgba8Unorm, HDR_FORMAT);
    let mut pipelines = super::pipelines::Pipelines::new(&device, surface, &shared);
    assert_eq!(pipelines.count(surface), 12, "4 blend modes x lit, pre-lit and sky");
    assert_eq!(pipelines.count(hdr), 0, "the HDR set is not built until HDR is used");
    assert!(!pipelines.has_glossy(), "the glossy shader is not compiled");
    pipelines.use_format(&device, hdr, false);
    assert_eq!((pipelines.count(surface), pipelines.count(hdr)), (12, 12));
    pipelines.use_format(&device, surface, true);
    assert!(pipelines.has_glossy());
    assert_eq!((pipelines.count(surface), pipelines.count(hdr)), (16, 12), "glossy only for the format in use");
}
