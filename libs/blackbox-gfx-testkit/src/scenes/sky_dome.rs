//! A sky dome 9.7 km away behind fogged ground and buildings.

use blackbox_gfx::{BlendMode, Instance, Shading};
use glam::{Mat4, Vec3};

use super::{Cx, Parts};
use crate::camera::Camera;
use crate::mesh::{MeshBuilder, prelit};
use crate::texture::{Image, Storage};

const HORIZON: [f32; 3] = [0.85, 0.88, 0.90];
const ZENITH: [f32; 3] = [0.25, 0.45, 0.85];
const HAZE: [f32; 3] = [0.35, 0.38, 0.42];
/// The dome's radius: far enough to need an infinite far plane.
pub(super) const DOME_RADIUS: f32 = 9700.0;

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t)
}

fn sky_colour(direction: Vec3) -> [u8; 4] {
    let up = direction.z;
    let rgb = match up >= 0.0 {
        true => mix(HORIZON, ZENITH, up.sqrt()),
        false => mix(HORIZON, HAZE, (-up * 4.0).min(1.0)),
    };
    prelit(rgb, 1.0)
}

pub(super) fn build(cx: &mut Cx) -> Parts {
    let camera = Camera::new(Vec3::new(0.0, 0.0, 2.0), Vec3::new(0.0, 100.0, 14.0));
    let grass = cx.texture("sky ground", &Image::checker(32, 16, [70, 110, 60], [80, 125, 70]), Storage::Rgba8);

    let mut dome = MeshBuilder::new();
    dome.state(None, BlendMode::Opaque, Shading::Sky);
    dome.sphere(Vec3::ZERO, DOME_RADIUS, 24, 48, sky_colour);
    let dome = cx.mesh("sky dome", &dome.finish());

    let mut land = MeshBuilder::new();
    land.state(Some(grass), BlendMode::Opaque, Shading::Prelit);
    let edge = 9000.0;
    let ground = [
        Vec3::new(-edge, -edge, 0.0),
        Vec3::new(edge, -edge, 0.0),
        Vec3::new(edge, edge, 0.0),
        Vec3::new(-edge, edge, 0.0),
    ];
    land.quad(ground, [prelit([1.0; 3], 1.0); 4], [180.0, 180.0]);
    let blocks: [(f32, f32, f32, f32, f32); 6] = [
        (-30.0, 60.0, 14.0, 30.0, 0.55),
        (20.0, 120.0, 22.0, 50.0, 0.5),
        (-90.0, 300.0, 40.0, 90.0, 0.45),
        (60.0, 700.0, 60.0, 160.0, 0.4),
        (-200.0, 2000.0, 150.0, 500.0, 0.35),
        (400.0, 5000.0, 400.0, 900.0, 0.3),
    ];
    for (x, y, width, height, grey) in blocks {
        let half = width * 0.5;
        land.cuboid(
            Vec3::new(x - half, y - half, 0.0),
            Vec3::new(x + half, y + half, height),
            prelit([grey, grey * 1.05, grey * 1.15], 1.0),
        );
    }
    let land = cx.mesh("sky land", &land.finish());

    // The dome follows the camera, as a sky does.
    let instances = vec![
        Instance { mesh: dome, transform: Mat4::from_translation(camera.eye) },
        Instance { mesh: land, transform: Mat4::IDENTITY },
    ];
    Parts::world(camera.frame(cx.aspect, HORIZON, Some((200.0, 6000.0))), instances)
}
