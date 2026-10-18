//! Normal-mapped pavement receives contact AO on ambient light, independently of road reflection.
use super::tests::scene_shading;
use crate::{Renderer, gpu::test_support};
use blackbox_gfx::{
    AoMethod, AoQuality, AoSettings, BlendMode, GraphicsSettings, Instance, MeshDesc, NormalMaterialDesc, NormalModel,
    PixelFormat, RenderBackend, Shading, SurfaceLight, TextureDesc,
};
use blackbox_gfx_testkit::mesh::MeshBuilder;
use glam::{Mat4, Vec3};

const SIZE: (u32, u32) = (640, 480);

fn receiver(r: &mut Renderer, model: NormalModel, blend: BlendMode) -> Instance {
    let mut texture = |rgba: [u8; 4]| {
        r.create_texture(&TextureDesc {
            label: "AO pavement fixture",
            width: 1,
            height: 1,
            format: PixelFormat::Rgba8,
            mips: vec![&rgba],
        })
    };
    let diffuse = texture([128, 128, 128, 255]);
    let normal = texture([128, 128, 255, 255]);
    let specular = texture([64; 4]);
    let reflection = texture([48, 64, 80, 255]);
    let material = r
        .create_normal_material(&NormalMaterialDesc {
            model,
            diffuse,
            normal,
            specular,
            reflection: Some(reflection),
            uv_offset: [0.0; 2],
        })
        .unwrap();
    let mut mesh = MeshBuilder::new();
    mesh.state(Some(diffuse), blend, Shading::WorldNormal(material));
    mesh.flat_quad(
        [
            Vec3::new(-10.0, -10.0, 0.0),
            Vec3::new(10.0, -10.0, 0.0),
            Vec3::new(10.0, 10.0, 0.0),
            Vec3::new(-10.0, 10.0, 0.0),
        ],
        [128, 128, 128, 255],
    );
    let mesh = mesh.finish();
    let tangents = vec![[1.0, 0.0, 0.0, 1.0]; mesh.vertices.len()];
    let handle = r.create_mesh(&MeshDesc {
        label: "AO normal-mapped receiver",
        vertices: &mesh.vertices,
        indices: &mesh.indices,
        secondary_uvs: None,
        tangents: Some(&tangents),
        draws: mesh.draws,
    });
    Instance::new(handle, Mat4::IDENTITY)
}

#[test]
#[ignore = "needs a GPU"]
fn pavement_receives_ambient_contacts_without_darkening_sun_specular_or_reflection() {
    let _serial = test_support::serial();
    let Some(mut r) = test_support::headless(SIZE) else { return };
    let (mut frame, _) = scene_shading(&mut r, SIZE, false, Shading::Prelit);
    r.set_render_experiments(blackbox_gfx::RenderExperiments { original_world_materials: true, ..Default::default() });
    let mut pole = MeshBuilder::new();
    pole.state(None, BlendMode::Opaque, Shading::Prelit);
    pole.cuboid(Vec3::new(-0.25, -0.25, 0.0), Vec3::new(0.25, 0.25, 2.0), [128, 128, 128, 255]);
    let pole = Instance::new(pole.finish().upload(&mut r, "AO pole"), Mat4::IDENTITY);
    let matrix = frame.projection.matrix() * frame.view;
    let ground: Vec<usize> = (0..12)
        .flat_map(|y| {
            (0..24).map(move |x| {
                let p = matrix.project_point3(Vec3::new(-0.65 + x as f32 * 0.055, -0.4 - y as f32 * 0.04, 0.0));
                let px = ((p.x + 1.0) * 0.5 * SIZE.0 as f32) as usize;
                let py = ((1.0 - p.y) * 0.5 * SIZE.1 as f32) as usize;
                (py * SIZE.0 as usize + px) * 4
            })
        })
        .collect();
    for model in [NormalModel::Bump, NormalModel::Reflective] {
        for blend in [BlendMode::Opaque, BlendMode::AlphaTest, BlendMode::AlphaBlend] {
            let floor = receiver(&mut r, model, blend);
            let instances = [floor, pole];
            for method in AoMethod::ALL {
                for ambient in [true, false] {
                    let (ambient_light, diffuse) = match ambient {
                        true => ([0.6; 3], [0.0; 3]),
                        false => ([0.0; 3], [0.5; 3]),
                    };
                    frame.surface_light = Some(SurfaceLight {
                        ambient: ambient_light,
                        ambient_alpha: 1.0,
                        diffuse,
                        specular: [0.2; 3],
                        specular_power: 11.0,
                        reflection_scale: 0.8,
                        reflective_specular_scale: 1.0,
                        shadow_strength: 1.0,
                    });
                    r.apply_graphics(&GraphicsSettings::default());
                    let off = r.capture(SIZE.0, SIZE.1, &frame, &instances).unwrap();
                    r.apply_graphics(&GraphicsSettings {
                        ao: AoSettings { method, quality: AoQuality::High, intensity: 1.0, ..Default::default() },
                        ..Default::default()
                    });
                    let on = r.capture(SIZE.0, SIZE.1, &frame, &instances).unwrap();
                    let dark = ground.iter().filter(|&&i| u16::from(off[i]) > u16::from(on[i]) + 2).count();
                    if ambient && blend != BlendMode::AlphaBlend {
                        // The reference kernel is weaker around a thin pole than analytic GTAO.
                        // Both must measurably darken the actual ground, not only the pole.
                        assert!(dark > 5, "{model:?}/{blend:?}/{method}: contact must reach pavement ({dark})");
                        continue;
                    }
                    assert!(
                        ground.iter().all(|&i| off[i..i + 3] == on[i..i + 3]),
                        "{model:?}/{blend:?}/{method}: direct/specular/reflection or transparent floor changed"
                    );
                }
            }
        }
    }
}
