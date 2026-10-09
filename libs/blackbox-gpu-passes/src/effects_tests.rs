//! Synthetic GPU checks for the actual additive effect pipeline (no surface or assets).

use blackbox_gfx::{EffectLayer, EffectVertex, TextureHandle};
use glam::{Mat4, Vec3};

use crate::test_support::{Gpu, serial};
use crate::{Effects, Globals, WorldBindings, create_depth};

const BACKGROUND: [u8; 4] = [16, 24, 32, 255];
#[path = "textured_effect_tests.rs"]
mod textured;

#[test]
#[ignore = "needs a Vulkan GPU"]
fn additive_streaks_vulkan() {
    check(wgpu::Backends::VULKAN);
}

#[cfg(target_os = "windows")]
#[test]
#[ignore = "needs a Direct3D 12 GPU"]
fn additive_streaks_dx12() {
    check(wgpu::Backends::DX12);
}

struct Bench {
    gpu: Gpu,
    bindings: WorldBindings,
    effects: Effects,
    /// Texture bind groups; a [`TextureHandle`] is an index.
    textures: Vec<wgpu::BindGroup>,
}

impl Bench {
    fn new(backend: wgpu::Backends) -> Self {
        let gpu = Gpu::new(backend);
        let bindings = WorldBindings::new(&gpu.device);
        let effects = Effects::new(&gpu.device, wgpu::TextureFormat::Rgba8Unorm, &bindings);
        Self { gpu, bindings, effects, textures: Vec::new() }
    }

    fn add_texture(&mut self, group: wgpu::BindGroup) -> TextureHandle {
        self.textures.push(group);
        TextureHandle::from_raw(self.textures.len() - 1)
    }

    fn sample(&mut self, layer: &EffectLayer, depth: f32, width: u32, fog: [f32; 2]) -> Pixels {
        let Gpu { device, queue } = &self.gpu;
        let globals = Globals {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            camera_pos: [0.0, 0.0, 0.0, 1.0],
            light_dir: [0.0; 4],
            // Bright coloured fog must never add energy to an additive streak.
            fog_color: [0.3, 0.6, 0.9, 1.0],
            fog_range: [fog[0], fog[1], 0.0, 0.0],
        };
        queue.write_buffer(&self.bindings.globals, 0, bytemuck::bytes_of(&globals));
        self.effects.upload(device, queue, layer);
        let target = self.gpu.target(width);
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view = create_depth(device, width, width);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("streak test pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: f64::from(BACKGROUND[0]) / 255.0,
                            g: f64::from(BACKGROUND[1]) / 255.0,
                            b: f64::from(BACKGROUND[2]) / 255.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(depth), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.bindings.globals_bind_group, &[]);
            self.effects.draw(&mut pass);
            self.effects.draw_textured(&mut pass, |t| self.textures.get(t.raw()));
        }
        Pixels { width, rgba: self.gpu.finish(encoder, &target) }
    }
}

struct Pixels {
    width: u32,
    rgba: Vec<u8>,
}

impl Pixels {
    fn at(&self, x: u32, y: u32) -> [u8; 4] {
        self.rgba[((y * self.width + x) * 4) as usize..][..4].try_into().unwrap()
    }

    fn center(&self) -> [u8; 4] {
        self.at(self.width / 2, self.width / 2)
    }
}

fn quad(out: &mut Vec<EffectVertex>, depth: f32, color: [u8; 4]) {
    EffectLayer::quad(
        out,
        [
            Vec3::new(-0.8, -0.8, depth),
            Vec3::new(0.8, -0.8, depth),
            Vec3::new(0.8, 0.8, depth),
            Vec3::new(-0.8, 0.8, depth),
        ],
        color,
    );
}

fn check(backend: wgpu::Backends) {
    let _lock = serial();
    let mut gpu = Bench::new(backend);
    let no_fog = [f32::MAX, f32::MAX];
    let mut layer = EffectLayer::default();
    quad(&mut layer.streaks, 0.8, [80, 0, 0, 128]);
    let front = gpu.sample(&layer, 0.6, 64, no_fog);
    let center = front.center();
    eprintln!("{backend:?}: additive streak center {center:?}");
    assert!((24..=28).contains(&center[0]), "RGB must use source alpha and the narrow tail mask plus the destination");
    assert_eq!(&center[1..], &BACKGROUND[1..], "a red streak must preserve other channels and opaque alpha");
    assert_eq!(front.at(55, 32), BACKGROUND, "the analytic cross-section feathers to transparent");
    assert!(front.at(32, 8)[0] < center[0], "the longitudinal tail fades");
    assert!(front.at(32, 55)[0] > center[0] + 20, "the light is concentrated at the tip");

    layer.clear();
    quad(&mut layer.streaks, 0.2, [80, 0, 0, 128]);
    assert_eq!(gpu.sample(&layer, 0.6, 64, no_fog).center(), BACKGROUND, "opaque reverse-Z depth occludes streaks");

    // Near geometry comes first: a farther additive streak must still draw because streaks do not write depth.
    layer.clear();
    quad(&mut layer.streaks, 0.8, [80, 0, 0, 128]);
    quad(&mut layer.streaks, 0.2, [0, 0, 100, 128]);
    let overlap = gpu.sample(&layer, 0.0, 64, no_fog).center();
    assert!(overlap[2] > BACKGROUND[2] + 4, "additive overlap must include the farther streak");
    let reversed: Vec<_> = layer.streaks[6..].iter().chain(&layer.streaks[..6]).copied().collect();
    layer.streaks = reversed;
    let reverse = gpu.sample(&layer, 0.0, 64, no_fog).center();
    for channel in 0..4 {
        assert!(overlap[channel].abs_diff(reverse[channel]) <= 1, "streak ordering must not change addition");
    }

    layer.clear();
    quad(&mut layer.streaks, 0.8, [80, 0, 0, 128]);
    let resized = gpu.sample(&layer, 0.6, 128, no_fog).center();
    assert!(center[0].abs_diff(resized[0]) <= 1, "replacement target sizes retain the same depth/blend behavior");
    assert_eq!(
        gpu.sample(&layer, 0.0, 64, [0.0, 0.1]).center(),
        BACKGROUND,
        "fully fogged light must not brighten fog"
    );
    let partial_fog = gpu.sample(&layer, 0.0, 64, [0.0, 1.6]).center();
    assert!(partial_fog[0] > BACKGROUND[0] && partial_fog[0] < center[0], "fog must reduce emitted light");

    let mut particles = EffectLayer::default();
    quad(&mut particles.particles, 0.8, [0, 0, 100, 255]);
    let particle = gpu.sample(&particles, 0.0, 64, no_fog).center();
    particles.streaks = layer.streaks.clone();
    let composed = gpu.sample(&particles, 0.0, 64, no_fog).center();
    assert!(
        composed[0].saturating_sub(particle[0]).abs_diff(center[0] - BACKGROUND[0]) <= 1,
        "streaks add after the standard alpha-particle batch"
    );
    particles.clear();
    assert_eq!(
        gpu.sample(&particles, 0.0, 64, no_fog).center(),
        BACKGROUND,
        "clearing must not leave stale GPU streaks"
    );
    quad(&mut particles.glows, 0.8, [80, 0, 0, 128]);
    let glow = gpu.sample(&particles, 0.6, 64, no_fog).center();
    assert!(glow[0] > BACKGROUND[0] + 30, "glow has an additive bright core");
    assert_eq!(&glow[1..], &BACKGROUND[1..]);
    assert_eq!(gpu.sample(&particles, 0.9, 64, no_fog).center(), BACKGROUND, "glows stay behind opaque geometry");
    assert_eq!(gpu.sample(&particles, 0.0, 64, [0.0, 0.1]).center(), BACKGROUND, "glows cannot brighten fog");
    particles.clear();
    assert_eq!(gpu.sample(&particles, 0.0, 64, no_fog).center(), BACKGROUND);
}
