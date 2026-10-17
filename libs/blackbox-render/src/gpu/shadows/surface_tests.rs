//! Road, bump and glazing receivers share dynamic visibility without shading ambient/reflections.
use super::tests::{dark_pixels, scene};
use crate::{Renderer, gpu::test_support};
use blackbox_gfx::{
    BlendMode, Instance, MeshDesc, NormalMaterialDesc, NormalModel, PixelFormat, RenderBackend, RenderExperiments,
    Shading, SurfaceLight, TextureDesc, WindowMaterialDesc,
};
use blackbox_gfx_testkit::mesh::MeshBuilder;
use glam::{Mat4, Vec3};

fn texture(r: &mut Renderer, rgba: [u8; 4]) -> blackbox_gfx::TextureHandle {
    r.create_texture(&TextureDesc {
        label: "shadow receiver fixture",
        width: 1,
        height: 1,
        format: PixelFormat::Rgba8,
        mips: vec![&rgba],
    })
}

fn receiver(r: &mut Renderer, family: usize, alpha: u8) -> Instance {
    let diffuse = texture(r, [96, 96, 96, alpha]);
    let normal = texture(r, [128, 128, 255, 255]);
    let specular = texture(r, [64; 4]);
    let reflection = texture(r, [32, 48, 64, 255]);
    let shading = match family {
        2 => Shading::WorldWindow(r.create_window_material(&WindowMaterialDesc { diffuse, reflection }).unwrap()),
        _ => Shading::WorldNormal(
            r.create_normal_material(&NormalMaterialDesc {
                model: match family {
                    0 => NormalModel::Bump,
                    _ => NormalModel::Reflective,
                },
                diffuse,
                normal,
                specular,
                reflection: Some(reflection),
                uv_offset: [0.0; 2],
            })
            .unwrap(),
        ),
    };
    let mut plane = MeshBuilder::new();
    plane.state(Some(diffuse), BlendMode::Opaque, shading);
    plane.flat_quad(
        [
            Vec3::new(-15.0, -15.0, 0.0),
            Vec3::new(15.0, -15.0, 0.0),
            Vec3::new(15.0, 15.0, 0.0),
            Vec3::new(-15.0, 15.0, 0.0),
        ],
        [128, 128, 128, 255],
    );
    let plane = plane.finish();
    let tangents = vec![[1.0, 0.0, 0.0, 1.0]; plane.vertices.len()];
    let mesh = r.create_mesh(&MeshDesc {
        label: "road and window shadow receiver",
        vertices: &plane.vertices,
        secondary_uvs: None,
        tangents: Some(&tangents),
        indices: &plane.indices,
        draws: plane.draws,
    });
    Instance::new(mesh, Mat4::IDENTITY)
}

#[test]
#[ignore = "needs a GPU"]
fn road_bump_and_glass_receive_moving_shadows_and_preserve_indirect_light() {
    let _gpu = test_support::serial();
    let Some(mut r) = test_support::headless((240, 160)) else { return };
    let (mut frame, _, mut casters) = scene(&mut r);
    frame.surface_light = Some(SurfaceLight {
        shadow_strength: 1.0,
        ambient: [0.12; 3],
        ambient_alpha: 1.0,
        diffuse: [0.5; 3],
        specular: [0.0; 3],
        specular_power: 11.0,
        reflection_scale: 0.8,
        reflective_specular_scale: 1.0,
    });
    for family in 0..3 {
        let visible = [receiver(&mut r, family, 128)];
        casters[0].transform = Mat4::IDENTITY;
        r.set_render_experiments(RenderExperiments { original_world_materials: true, ..Default::default() });
        let unshadowed = r.capture(240, 160, &frame, &visible).unwrap();
        r.set_shadow_casters(Some(&casters));
        for resolution in [512, 2048, 1024] {
            r.set_render_experiments(RenderExperiments {
                original_world_materials: true,
                sun_shadows: true,
                shadow_resolution: resolution,
                ..Default::default()
            });
            let shadow = r.capture(240, 160, &frame, &visible).unwrap();
            assert!(dark_pixels(&unshadowed, &shadow) > 50, "family {family} receives shadows");
            assert_eq!(shadow, r.capture(240, 160, &frame, &visible).unwrap());
        }
        let before = r.capture(240, 160, &frame, &visible).unwrap();
        casters[0].transform = Mat4::from_translation(Vec3::new(2.0, 0.0, 0.0));
        r.set_shadow_casters(Some(&casters));
        assert_ne!(before, r.capture(240, 160, &frame, &visible).unwrap(), "family {family} follows movement");
        let light = frame.surface_light.as_mut().unwrap();
        light.diffuse = [0.0; 3];
        // Glass alpha is also its highlight mask. Remove that direct term when
        // isolating its ambient; reflective roads retain their reflected image.
        let indirect_visible = match family {
            2 => [receiver(&mut r, family, 0)],
            _ => visible,
        };
        r.set_render_experiments(RenderExperiments { original_world_materials: true, ..Default::default() });
        let indirect = r.capture(240, 160, &frame, &indirect_visible).unwrap();
        r.set_render_experiments(RenderExperiments {
            original_world_materials: true,
            sun_shadows: true,
            ..Default::default()
        });
        assert!(
            indirect == r.capture(240, 160, &frame, &indirect_visible).unwrap(),
            "family {family}: ambient/reflection unchanged"
        );
        frame.surface_light.as_mut().unwrap().diffuse = [0.5; 3];
        r.set_render_experiments(RenderExperiments { original_world_materials: true, ..Default::default() });
        assert_eq!(unshadowed, r.capture(240, 160, &frame, &visible).unwrap(), "disabled round trip");
    }
}

#[test]
#[ignore = "needs a GPU"]
fn fitted_projected_map_reaches_surfaces_and_rebinds_independently_from_cascades() {
    let _gpu = test_support::serial();
    let Some(mut r) = test_support::headless((240, 160)) else { return };
    let (mut frame, _, casters) = scene(&mut r);
    frame.surface_light = Some(SurfaceLight {
        shadow_strength: 0.9,
        ambient: [0.12; 3],
        ambient_alpha: 1.0,
        diffuse: [0.5; 3],
        specular: [0.0; 3],
        specular_power: 11.0,
        reflection_scale: 0.0,
        reflective_specular_scale: 1.0,
    });
    let visible = [receiver(&mut r, 1, 128)];
    let baseline = RenderExperiments { original_world_materials: true, ..Default::default() };
    r.set_render_experiments(baseline);
    let off = r.capture(240, 160, &frame, &visible).unwrap();
    r.set_shadow_casters(Some(&casters));
    for projection in
        [Some(super::projected_fit::tests::parameters()), None, Some(super::projected_fit::tests::parameters())]
    {
        for resolution in [1024, 2048] {
            r.set_render_experiments(RenderExperiments {
                sun_shadows: true,
                shadow_projection: projection,
                shadow_resolution: resolution,
                ..baseline
            });
            let shadow = r.capture(240, 160, &frame, &visible).unwrap();
            assert!(dark_pixels(&off, &shadow) > 50, "projected {projection:?}: road needs a visible shadow");
            assert!(shadow == r.capture(240, 160, &frame, &visible).unwrap(), "no frame-dependent history");
            let layers = if projection.is_some() { 1 } else { 4 };
            assert_eq!(r.shared.shadows.layer_count, layers);
        }
    }
    r.set_render_experiments(RenderExperiments {
        sun_shadows: true,
        shadow_projection: Some(super::projected_fit::tests::parameters()),
        ..baseline
    });
    frame.surface_light.as_mut().unwrap().shadow_strength = 0.0;
    assert!(off == r.capture(240, 160, &frame, &visible).unwrap(), "zero contrast does not darken the road");
}

#[test]
#[ignore = "needs a GPU"]
fn original_foliage_omits_receiving_while_custom_cascades_can_shadow_it() {
    let _gpu = test_support::serial();
    let Some(mut r) = test_support::headless((240, 160)) else { return };
    let (frame, _, casters) = scene(&mut r);
    let mut plane = MeshBuilder::new();
    plane.state(None, BlendMode::Opaque, Shading::Tree);
    plane.flat_quad(
        [
            Vec3::new(-15.0, -15.0, 0.0),
            Vec3::new(15.0, -15.0, 0.0),
            Vec3::new(15.0, 15.0, 0.0),
            Vec3::new(-15.0, 15.0, 0.0),
        ],
        [128; 4],
    );
    let mesh = plane.finish().upload(&mut r, "foliage receiver family");
    let visible = [Instance::new(mesh, Mat4::IDENTITY)];
    let baseline = RenderExperiments { original_world_materials: true, ..Default::default() };
    r.set_render_experiments(baseline);
    let off = r.capture(240, 160, &frame, &visible).unwrap();
    r.set_shadow_casters(Some(&casters));
    r.set_render_experiments(RenderExperiments {
        sun_shadows: true,
        shadow_projection: Some(super::projected_fit::tests::parameters()),
        ..baseline
    });
    assert!(off == r.capture(240, 160, &frame, &visible).unwrap(), "original Tree has no shadow receiver");
    r.set_render_experiments(RenderExperiments { sun_shadows: true, ..baseline });
    assert!(dark_pixels(&off, &r.capture(240, 160, &frame, &visible).unwrap()) > 50, "custom remains an enhancement");
}

#[test]
#[ignore = "needs a GPU"]
fn projected_roads_do_not_shadow_their_own_plane_at_city_coordinates() {
    let _gpu = test_support::serial();
    let Some(mut r) = test_support::headless((240, 160)) else { return };
    let (mut frame, _, _) = scene(&mut r);
    let offset = Vec3::new(1945.0, 1850.0, 150.0);
    frame.camera_position += offset;
    frame.view = glam::camera::rh::view::look_at_mat4(frame.camera_position, offset, Vec3::Z);
    let baseline = RenderExperiments { original_world_materials: true, ..Default::default() };
    for slope in [0.0, 0.2] {
        let mut instance = receiver(&mut r, 1, 128);
        instance.transform = Mat4::from_translation(offset) * Mat4::from_rotation_x(slope);
        let visible = [instance];
        r.set_render_experiments(baseline);
        let off = r.capture(240, 160, &frame, &visible).unwrap();
        r.set_shadow_casters(Some(&visible));
        r.set_render_experiments(RenderExperiments {
            sun_shadows: true,
            shadow_projection: Some(super::projected_fit::tests::parameters()),
            ..baseline
        });
        let on = r.capture(240, 160, &frame, &visible).unwrap();
        let dark = dark_pixels(&off, &on);
        assert!(dark < 5, "slope {slope}: {dark} self-shadowed pixels");
    }
}
