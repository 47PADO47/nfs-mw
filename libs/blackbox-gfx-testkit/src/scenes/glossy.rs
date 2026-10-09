//! Two glossy spheres lit by the lighting rig and reflecting the environment, over a lit floor.

use blackbox_gfx::{BlendMode, Environment, GlossyMaterial, Instance, LightingRig, Shading, SkyGradient};
use glam::{Mat4, Vec3};

use super::{Cx, Parts};
use crate::camera::Camera;
use crate::mesh::{MeshBuilder, prelit};
use crate::texture::{Image, Storage};

const CLEAR: [f32; 3] = [0.40, 0.50, 0.62];

pub(super) fn build(cx: &mut Cx) -> Parts {
    cx.backend.set_lighting_rig(&LightingRig::default());
    cx.backend.set_environment(Environment::Sky(SkyGradient::default()));

    let paint = cx.texture("glossy paint", &Image::ramp(64, [200, 40, 40], [250, 190, 60]), Storage::Bc1);
    let floor = cx.texture("glossy floor", &Image::checker(32, 4, [150, 150, 155], [95, 95, 100]), Storage::Rgba8);
    let satin = cx.glossy(&GlossyMaterial {
        envmap_min: 0.15,
        envmap_range: 0.55,
        envmap_power: 2.0,
        ..GlossyMaterial::default()
    });
    let mirror = cx.glossy(&GlossyMaterial {
        diffuse_min: [0.8, 0.9, 1.0, 1.0],
        specular_min: [0.3; 3],
        specular_range: [0.7; 3],
        specular_power: 24.0,
        envmap_min: 0.35,
        envmap_range: 0.6,
        envmap_power: 1.5,
        ..GlossyMaterial::default()
    });

    let mut scene = MeshBuilder::new();
    scene.state(Some(floor), BlendMode::Opaque, Shading::Lit);
    let floor_corners =
        [Vec3::new(-8.0, -2.0, 0.0), Vec3::new(8.0, -2.0, 0.0), Vec3::new(8.0, 14.0, 0.0), Vec3::new(-8.0, 14.0, 0.0)];
    scene.quad(floor_corners, [prelit([1.0; 3], 1.0); 4], [6.0, 6.0]);
    let floor_mesh = cx.mesh("glossy floor", &scene.finish());

    let mut spheres = MeshBuilder::new();
    spheres.state(Some(paint), BlendMode::Opaque, Shading::Glossy(satin));
    spheres.sphere(Vec3::new(-1.9, 6.0, 1.3), 1.3, 24, 48, |_| prelit([1.0; 3], 1.0));
    spheres.state(Some(paint), BlendMode::Opaque, Shading::Glossy(mirror));
    spheres.sphere(Vec3::new(1.9, 6.0, 1.3), 1.3, 24, 48, |_| prelit([1.0; 3], 1.0));
    let spheres = cx.mesh("glossy spheres", &spheres.finish());

    let instances = vec![
        Instance { mesh: floor_mesh, transform: Mat4::IDENTITY },
        Instance { mesh: spheres, transform: Mat4::IDENTITY },
    ];
    let camera = Camera::new(Vec3::new(0.0, -3.0, 1.8), Vec3::new(0.0, 6.0, 1.2));
    Parts::world(camera.frame(cx.aspect, CLEAR, None), instances)
}
