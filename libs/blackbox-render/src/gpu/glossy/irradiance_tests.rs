//! End-to-end diffuse probe tests: constant energy, directionality, replacement and restoration.
use super::env::face_direction;
use crate::{Renderer, gpu::test_support};
use blackbox_gfx::{
    BlendMode, DirectionalLight, FrameParams, GlossyMaterial, Instance, LightingRig, Projection, ReflectionQuality,
    ReflectionScene, RenderBackend, RenderExperiments, Shading,
};
use blackbox_gfx_testkit::mesh::MeshBuilder;
use glam::{Mat4, Vec3};

const SIZE: (u32, u32) = (96, 96);

fn fixture(r: &mut Renderer) -> (FrameParams, Vec<Instance>) {
    let material = r.create_glossy_material(&GlossyMaterial {
        diffuse_min: [1.0, 1.0, 1.0, 0.0],
        specular_min: [0.0; 3],
        envmap_min: 0.0,
        ..Default::default()
    });
    r.set_lighting_rig(&LightingRig {
        lights: [DirectionalLight::new(Vec3::Z, Vec3::ZERO); 3],
        ambient: Vec3::ZERO,
        ..Default::default()
    });
    r.set_render_experiments(RenderExperiments {
        car_skylight: true,
        sky_strength: 1.0,
        ground_strength: 1.0,
        ..Default::default()
    });
    let mut mesh = MeshBuilder::new();
    mesh.state(None, BlendMode::Opaque, Shading::Glossy(material));
    mesh.flat_quad(
        [
            Vec3::new(-10.0, -10.0, 0.0),
            Vec3::new(10.0, -10.0, 0.0),
            Vec3::new(10.0, 10.0, 0.0),
            Vec3::new(-10.0, 10.0, 0.0),
        ],
        [128, 128, 128, 255],
    );
    let mesh = mesh.finish().upload(r, "diffuse probe receiver");
    let eye = Vec3::new(0.0, -4.0, 3.0);
    (
        FrameParams {
            view: glam::camera::rh::view::look_at_mat4(eye, Vec3::ZERO, Vec3::Z),
            projection: Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: 1.0, near: 0.1 },
            camera_position: eye,
            light_dir: Vec3::NEG_Z,
            clear_color: [0.0; 3],
            fog: None,
            scattering: None,
            surface_light: None,
            camera_cut: false,
            shadow_tuning: None,
        },
        vec![Instance::new(mesh, Mat4::IDENTITY)],
    )
}

fn center(r: &mut Renderer, frame: &FrameParams, instances: &[Instance]) -> [u8; 3] {
    let image = r.capture(SIZE.0, SIZE.1, frame, instances).unwrap();
    let at = (48 * 96 + 48) * 4;
    [image[at], image[at + 1], image[at + 2]]
}

#[test]
#[ignore = "needs a GPU"]
fn diffuse_probe_preserves_constant_energy_and_follows_cube_replacement() {
    let _serial = test_support::serial();
    let Some(mut r) = test_support::headless(SIZE) else { return };
    let (frame, instances) = fixture(&mut r);
    for rgb in [[64, 96, 128], [112, 40, 24], [0, 0, 0]] {
        let face: Vec<u8> = (0..64).flat_map(|_| [rgb[0], rgb[1], rgb[2], 255]).collect();
        r.set_environment_faces(8, [&face; 6]);
        let actual = center(&mut r, &frame, &instances);
        for i in 0..3 {
            assert!(actual[i].abs_diff(rgb[i]) <= 2, "constant environment {rgb:?} returned {actual:?}");
        }
        assert_eq!(actual, center(&mut r, &FrameParams { camera_cut: true, ..frame }, &instances));
    }
}

#[test]
#[ignore = "needs a GPU"]
fn sky_ground_controls_and_local_capture_restore_refresh_the_probe() {
    let _serial = test_support::serial();
    let Some(mut r) = test_support::headless(SIZE) else { return };
    let (mut frame, instances) = fixture(&mut r);
    let faces: [Vec<u8>; 6] = std::array::from_fn(|face| {
        (0..64)
            .flat_map(|p| {
                let n = face_direction(face, (p % 8) as f32 / 8.0 + 0.0625, (p / 8) as f32 / 8.0 + 0.0625);
                if n.z > 0.0 {
                    return [0, 0, 160, 255];
                }
                [160, 0, 0, 255]
            })
            .collect()
    });
    r.set_environment_faces(8, faces.each_ref().map(Vec::as_slice));
    let sky = center(&mut r, &frame, &instances);
    assert!(sky[2] > 150 && sky[0] < 10, "up-facing surface receives sky: {sky:?}");
    let options = r.render_experiments();
    r.set_render_experiments(RenderExperiments { sky_strength: 0.0, ..options });
    assert!(center(&mut r, &frame, &instances).iter().all(|&v| v < 10), "ground does not illuminate an upward normal");
    r.set_render_experiments(options);
    frame.clear_color = [0.2, 0.4, 0.1];
    r.set_reflection_scene(Some(ReflectionScene {
        origin: Vec3::ZERO,
        frame,
        faces: [&[]; 6],
        refresh: blackbox_gfx::reflection::ALL_FACES,
        quality: ReflectionQuality::Low,
    }));
    let local = center(&mut r, &frame, &instances);
    assert!(local[1] > local[2] && local[1] > local[0], "local capture updates diffuse probe: {local:?}");
    r.set_reflection_scene(None);
    assert_eq!(sky, center(&mut r, &frame, &instances), "reflection-off restores static diffuse light");
    r.set_render_experiments(RenderExperiments { car_skylight: false, ..options });
    assert_eq!([0, 0, 0], center(&mut r, &frame, &instances), "off restores the original light rig");
}
