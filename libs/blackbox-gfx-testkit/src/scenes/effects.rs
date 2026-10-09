//! The effect layer over a small world: ground surfaces, soft particles, glows, streaks, textured effects.

use blackbox_gfx::{BlendMode, EffectLayer, EffectVertex, Instance, Shading, TexturedEffect};
use glam::{Mat4, Vec3};

use super::{Cx, Parts};
use crate::camera::Camera;
use crate::mesh::{MeshBuilder, prelit};
use crate::texture::{Image, Storage};

const CLEAR: [f32; 3] = [0.20, 0.24, 0.30];

/// A camera-facing quad of half-size `radius` around `centre`.
fn billboard(camera: &Camera, centre: Vec3, radius: f32) -> [Vec3; 4] {
    let (right, up) = camera.right_up();
    let (r, u) = (right * radius, up * radius);
    [centre - r + u, centre + r + u, centre + r - u, centre - r - u]
}

/// A long thin quad from `start` to `end`: u across the width, v along the length.
fn streak(out: &mut Vec<EffectVertex>, start: Vec3, end: Vec3, half_width: f32, color: [u8; 4]) {
    let side = Vec3::X * half_width;
    let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let corners = [start - side, start + side, end + side, end - side];
    for i in [0, 1, 2, 0, 2, 3] {
        out.push(EffectVertex { position: corners[i].to_array(), color, uv: uv[i], detail: [0.0; 2] });
    }
}

pub(super) fn build(cx: &mut Cx) -> Parts {
    let camera = Camera::new(Vec3::new(0.0, -6.0, 2.6), Vec3::new(0.0, 8.0, 1.0));
    let floor = cx.texture("effects floor", &Image::checker(32, 4, [120, 120, 130], [80, 80, 90]), Storage::Rgba8);
    let crate_texture =
        cx.texture("effects crate", &Image::checker(16, 4, [190, 140, 80], [140, 100, 50]), Storage::Bc1);
    let glow = cx.texture("effects glow", &Image::glow(64), Storage::Rgba8);

    let mut world = MeshBuilder::new();
    world.state(Some(floor), BlendMode::Opaque, Shading::Prelit);
    let corners = [
        Vec3::new(-10.0, -4.0, 0.0),
        Vec3::new(10.0, -4.0, 0.0),
        Vec3::new(10.0, 30.0, 0.0),
        Vec3::new(-10.0, 30.0, 0.0),
    ];
    world.quad(corners, [prelit([1.0; 3], 1.0); 4], [10.0, 17.0]);
    world.state(Some(crate_texture), BlendMode::Opaque, Shading::Lit);
    world.cuboid(Vec3::new(1.5, 7.0, 0.0), Vec3::new(3.5, 9.0, 2.0), prelit([1.0; 3], 1.0));
    let world = cx.mesh("effects world", &world.finish());

    let mut layer = EffectLayer::default();
    // Skid-like marks lying on the floor.
    for x in [-2.0_f32, -1.2] {
        let marks = [
            Vec3::new(x, 1.0, 0.02),
            Vec3::new(x + 0.35, 1.0, 0.02),
            Vec3::new(x + 0.35, 14.0, 0.02),
            Vec3::new(x, 14.0, 0.02),
        ];
        EffectLayer::quad(&mut layer.surfaces, marks, [20, 20, 24, 200]);
    }
    // Smoke: one puff clear of everything, one sinking into the crate (the soft fade).
    EffectLayer::particle_quad(
        &mut layer.particles,
        billboard(&camera, Vec3::new(-3.5, 9.0, 1.6), 1.6),
        [190, 190, 195, 190],
        [0.0; 2],
    );
    EffectLayer::particle_quad(
        &mut layer.particles,
        billboard(&camera, Vec3::new(2.5, 7.4, 1.2), 1.4),
        [220, 200, 170, 200],
        [0.0; 2],
    );
    // Lights.
    for (x, rgb) in [(-5.0, [255, 150, 60]), (0.0, [90, 190, 255]), (5.0, [255, 70, 150])] {
        EffectLayer::quad(
            &mut layer.glows,
            billboard(&camera, Vec3::new(x, 12.0, 3.2), 0.9),
            [rgb[0], rgb[1], rgb[2], 230],
        );
    }
    // Sparks.
    for (k, x) in [-1.0_f32, -0.4, 0.2, 0.9].into_iter().enumerate() {
        streak(
            &mut layer.streaks,
            Vec3::new(x, 3.0, 0.5 + k as f32 * 0.3),
            Vec3::new(x + 0.2, 10.0, 0.9 + k as f32 * 0.4),
            0.05,
            [255, 220, 140, 255],
        );
    }
    // Textured effects, one of each blend mode.
    let mut halo = Vec::new();
    EffectLayer::quad(&mut halo, billboard(&camera, Vec3::new(-2.0, 6.0, 2.4), 1.0), [255, 200, 120, 255]);
    layer.textured.push(TexturedEffect { texture: glow, blend: BlendMode::Additive, vertices: halo });
    let mut fog_puff = Vec::new();
    EffectLayer::quad(&mut fog_puff, billboard(&camera, Vec3::new(4.5, 5.0, 1.0), 1.2), [150, 220, 160, 220]);
    layer.textured.push(TexturedEffect { texture: glow, blend: BlendMode::AlphaBlend, vertices: fog_puff });

    let instances = vec![Instance::new(world, Mat4::IDENTITY)];
    let mut parts = Parts::world(camera.frame(cx.aspect, CLEAR, Some((10.0, 40.0))), instances);
    parts.effects = layer;
    parts
}
