//! Per-view fog state: a second view never reallocates or borrows the main view's volume and history.
use glam::{Mat4, Vec3};

use super::*;
use crate::test_support::{Gpu, serial};
use crate::{Globals, create_depth};

fn light(x: f32) -> LocalLight {
    LocalLight {
        position: [x, 0.0, 3.0],
        range: 12.0,
        direction: [0.0, 0.0, -1.0],
        outer_cos: 0.2,
        inner_cos: 0.7,
        color: [5.0, 4.0, 3.0],
        upper_plane: None,
        casts_shadow: false,
        shadow_exclude: None,
    }
}

struct Bench {
    gpu: Gpu,
    bindings: WorldBindings,
    fog: LightFog,
}

/// A view's targets: colour and depth of one size.
struct Target {
    color: wgpu::TextureView,
    depth: wgpu::TextureView,
}

impl Bench {
    fn new() -> Self {
        let gpu = Gpu::new(wgpu::Backends::VULKAN);
        let bindings = WorldBindings::new(&gpu.device);
        let mut fog = LightFog::new(&gpu.device, &bindings, None);
        fog.use_format(&gpu.device, wgpu::TextureFormat::Rgba8Unorm);
        let globals = Globals {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            camera_pos: [0.0, 0.0, 0.0, 1.0],
            ..bytemuck::Zeroable::zeroed()
        };
        gpu.queue.write_buffer(&bindings.globals, 0, bytemuck::bytes_of(&globals));
        Self { gpu, bindings, fog }
    }

    fn target(&self, width: u32, height: u32) -> Target {
        let color = self.gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("fog test colour"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        Target { color: color.create_view(&Default::default()), depth: create_depth(&self.gpu.device, width, height) }
    }

    fn upload(&mut self, strength: f32, quality: u32) {
        self.fog.upload([strength, 1.0], &[], &[light(5.0)], 8, 200.0, quality);
    }

    fn draw(&mut self, target: &Target, camera: Vec3, view: FogView) {
        let draw = SoftDraw {
            target: &target.color,
            depth: &target.depth,
            globals: &self.bindings.globals_bind_group,
            inverse_view_proj: Mat4::IDENTITY,
            camera_position: camera,
            camera_cut: false,
            local_fog_shadows: None,
        };
        let device = &self.gpu.device;
        let mut encoder = device.create_command_encoder(&Default::default());
        self.fog.draw(device, &self.gpu.queue, &mut encoder, &draw, view);
        self.gpu.queue.submit([encoder.finish()]);
    }

    fn state(&self, key: FogView) -> Option<&ViewFog> {
        self.fog.views.iter().find(|view| view.key == key)
    }
}

#[test]
#[ignore = "needs a Vulkan GPU"]
fn a_smaller_second_view_keeps_the_main_volume_and_history() {
    let _gpu = serial();
    let mut bench = Bench::new();
    let main = bench.target(1280, 720);
    let mirror = bench.target(320, 120);
    let scope = bench.gpu.device.push_error_scope(wgpu::ErrorFilter::Validation);
    bench.upload(1.0, 2);
    bench.draw(&main, Vec3::ZERO, FogView::Main);
    let volume = bench.state(FogView::Main).and_then(ViewFog::integrated).expect("main volume");
    for step in 1..6 {
        bench.upload(1.0, 2);
        bench.draw(&mirror, Vec3::ZERO, FogView::Unkeyed);
        let state = bench.state(FogView::Main).unwrap();
        assert!(state.previous(&frame_of(&bench, step)).is_some(), "the main view keeps its history");
        bench.draw(&main, Vec3::X * step as f32 * 0.1, FogView::Main);
        let state = bench.state(FogView::Main).unwrap();
        assert_eq!(state.integrated().as_ref(), Some(&volume), "the second view must not reallocate the main volume");
    }
    assert_eq!(bench.fog.views.len(), 2);
    assert!(pollster::block_on(scope.pop()).is_none());
}

fn frame_of(bench: &Bench, step: usize) -> view::Frame {
    view::Frame {
        number: bench.fog.frame,
        camera: Vec3::X * step as f32 * 0.1,
        camera_cut: false,
        options: bench.fog.options,
        distance: bench.fog.distance,
    }
}

#[test]
#[ignore = "needs a Vulkan GPU"]
fn alternating_views_of_one_size_each_find_their_own_state() {
    let _gpu = serial();
    let mut bench = Bench::new();
    let half = bench.target(640, 360);
    let far = Vec3::new(500.0, 0.0, 0.0);
    for step in 0..4 {
        bench.upload(1.0, 2);
        bench.draw(&half, Vec3::Y * step as f32 * 0.2, FogView::Unkeyed);
        bench.upload(1.0, 2);
        bench.draw(&half, far + Vec3::Y * step as f32 * 0.2, FogView::Unkeyed);
    }
    assert_eq!(bench.fog.views.len(), 2, "two players, two states");
    let cameras: Vec<_> = bench.fog.views.iter().filter_map(ViewFog::camera).collect();
    assert!(cameras.iter().any(|c| c.x < 1.0) && cameras.iter().any(|c| c.x > 499.0));
    assert!(bench.fog.views.iter().all(ViewFog::has_history));
    // Explicit slots never share either.
    bench.upload(1.0, 2);
    bench.draw(&half, Vec3::ZERO, FogView::Slot(0));
    bench.draw(&half, Vec3::ZERO, FogView::Slot(1));
    assert_eq!(bench.fog.views.len(), 4);
}

#[test]
#[ignore = "needs a Vulkan GPU"]
fn resize_and_quality_reallocate_only_the_changed_view() {
    let _gpu = serial();
    let mut bench = Bench::new();
    let small = bench.target(640, 360);
    let large = bench.target(1920, 1080);
    bench.upload(1.0, 2);
    bench.draw(&small, Vec3::ZERO, FogView::Main);
    let before = bench.state(FogView::Main).and_then(ViewFog::integrated).unwrap();
    bench.upload(1.0, 2);
    bench.draw(&small, Vec3::ZERO, FogView::Main);
    assert_eq!(bench.state(FogView::Main).and_then(ViewFog::integrated), Some(before.clone()), "steady: kept");
    bench.upload(1.0, 2);
    bench.draw(&large, Vec3::ZERO, FogView::Main);
    let resized = bench.state(FogView::Main).and_then(ViewFog::integrated).unwrap();
    assert_ne!(resized, before, "a resize rebuilds the volume");
    bench.upload(1.0, 1);
    bench.draw(&large, Vec3::ZERO, FogView::Main);
    let low = bench.state(FogView::Main).unwrap();
    assert_ne!(low.integrated().unwrap(), resized, "a quality change rebuilds the volume");
    assert_eq!(bench.fog.views.len(), 1);
}

#[test]
#[ignore = "needs a Vulkan GPU"]
fn disabled_fog_releases_its_memory() {
    let _gpu = serial();
    let mut bench = Bench::new();
    let main = bench.target(1280, 720);
    bench.upload(1.0, 3);
    bench.draw(&main, Vec3::ZERO, FogView::Main);
    assert!(bench.fog.allocated_bytes() > 1 << 20);
    bench.upload(0.0, 3);
    assert_eq!(bench.fog.allocated_bytes(), 0, "zero strength releases at once");
    bench.upload(1.0, 3);
    bench.draw(&main, Vec3::ZERO, FogView::Main);
    assert!(bench.fog.allocated_bytes() > 0);
    bench.fog.upload([1.0, 1.0], &[], &[light(5.0)], 0, 200.0, 3);
    assert_eq!(bench.fog.allocated_bytes(), 0, "a zero light limit releases at once");
    bench.upload(1.0, 3);
    bench.draw(&main, Vec3::ZERO, FogView::Main);
    // Let the light's fade-out run its course: the selection fades by wall-clock time, at most 0.1 s per upload.
    while !bench.fog.lights.is_empty() {
        std::thread::sleep(std::time::Duration::from_millis(100));
        bench.fog.upload([1.0, 1.0], &[], &[], 8, 200.0, 3);
    }
    assert!(bench.fog.allocated_bytes() > 0, "a short gap keeps the view");
    for _ in 0..=RELEASE_AFTER {
        bench.fog.upload([1.0, 1.0], &[], &[], 8, 200.0, 3);
        bench.draw(&main, Vec3::ZERO, FogView::Main);
    }
    assert_eq!(bench.fog.allocated_bytes(), 0, "a view without lit fog is released after a while");
}

#[test]
#[ignore = "needs a Vulkan GPU"]
fn devices_without_storage_buffers_draw_nothing_and_raise_no_error() {
    let _gpu = serial();
    let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
    desc.backends = wgpu::Backends::VULKAN;
    let instance = wgpu::Instance::new(desc);
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).expect("test GPU adapter");
    let (device, _queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
        ..Default::default()
    }))
    .unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let bindings = WorldBindings::new(&device);
    let effects = crate::Effects::new(&device, wgpu::TextureFormat::Rgba8Unorm, &bindings, None);
    assert!(pollster::block_on(scope.pop()).is_none(), "no validation error at startup");
    assert!(!effects.light_fog_supported());
}
