use super::*;
use crate::gpu::test_support;
use blackbox_gfx::{LocalLight, RenderBackend, RenderExperiments};
use blackbox_gfx_testkit::mesh::{MeshBuilder, prelit};
#[test]
#[ignore = "requires GPU"]
fn local_lamps_illuminate_cast_moving_shadows_and_restore_off() {
    let _gpu = test_support::serial();
    let mut r = test_support::headless((240, 160)).expect("local lamp GPU");
    let (frame, _, mut casters) = super::tests::scene(&mut r);
    let mut m = MeshBuilder::new();
    m.state(None, BlendMode::Opaque, Shading::Prelit);
    m.flat_quad(
        [
            Vec3::new(-15.0, -15.0, 0.0),
            Vec3::new(15.0, -15.0, 0.0),
            Vec3::new(15.0, 15.0, 0.0),
            Vec3::new(-15.0, 15.0, 0.0),
        ],
        prelit([0.12; 3], 1.0),
    );
    let ground = [Instance::new(m.finish().upload(&mut r, "dark local receiver"), Mat4::IDENTITY)];
    let baseline = r.capture(240, 160, &frame, &ground).unwrap();
    let lights = [LocalLight {
        position: [1.0, -1.0, 5.0],
        range: 12.0,
        direction: [-0.2, 0.2, -1.0],
        outer_cos: 0.2,
        inner_cos: 0.7,
        color: [1.8, 1.5, 1.2],
        casts_shadow: true,
        shadow_exclude: None,
        upper_plane: None,
    }];
    r.set_local_lights(&lights);
    r.set_shadow_casters(Some(&[]));
    let lit = r.capture(240, 160, &frame, &ground).unwrap();
    let brighter = baseline
        .as_chunks::<4>()
        .0
        .iter()
        .zip(lit.as_chunks::<4>().0)
        .filter(|(a, b)| b[0] > a[0].saturating_add(8))
        .count();
    assert!(brighter > 100, "lamp must add direct light: {brighter}");
    r.set_shadow_casters(Some(&casters));
    let shadow = r.capture(240, 160, &frame, &ground).unwrap();
    let darker = lit
        .as_chunks::<4>()
        .0
        .iter()
        .zip(shadow.as_chunks::<4>().0)
        .filter(|(a, b)| a[0] > b[0].saturating_add(8))
        .count();
    assert!(darker > 30, "offscreen geometry must block the lamp: {darker}");
    let owner = blackbox_gfx::InstanceKey::new(9, 7);
    casters[0].owner_key = owner;
    r.set_shadow_casters(Some(&casters));
    r.set_local_lights(&[LocalLight { shadow_exclude: Some(owner), ..lights[0] }]);
    assert_eq!(lit, r.capture(240, 160, &frame, &ground).unwrap(), "a lamp must not shadow itself");
    r.set_local_lights(&[LocalLight { shadow_exclude: Some(blackbox_gfx::InstanceKey::new(9, 8)), ..lights[0] }]);
    assert_eq!(shadow, r.capture(240, 160, &frame, &ground).unwrap(), "other objects still cast shadows");
    r.set_local_lights(&lights);
    casters[0].transform = Mat4::from_translation(Vec3::new(3.0, 0.0, 0.0));
    r.set_shadow_casters(Some(&casters));
    let moved = r.capture(240, 160, &frame, &ground).unwrap();
    assert_ne!(shadow, moved, "moving caster moves its shadow");
    r.set_local_lights(&[]);
    assert_eq!(baseline, r.capture(240, 160, &frame, &ground).unwrap(), "off restores original lighting exactly");
    r.set_local_lights(&lights);
    let _ = r.capture(240, 160, &frame, &ground).unwrap();
    r.set_local_lights(&[]);
    assert_eq!(baseline, r.capture(240, 160, &frame, &ground).unwrap(), "re-enable cannot leave stale local lighting");
    r.set_local_lights(&[LocalLight { upper_plane: Some([0.0, 0.0, -1.0]), ..lights[0] }]);
    assert_eq!(
        baseline,
        r.capture(240, 160, &frame, &ground).unwrap(),
        "upper cutoff excludes receivers above its plane"
    );
}

#[test]
#[ignore = "requires GPU"]
fn headlights_light_raised_receivers_without_lighting_high_overhead_surfaces() {
    let _gpu = test_support::serial();
    let mut r = test_support::headless((240, 160)).expect("headlight GPU");
    let (frame, _, _) = super::tests::scene(&mut r);
    r.set_shadow_casters(Some(&[]));
    let light = LocalLight {
        position: [0.0, -5.0, 0.6],
        range: 45.0,
        direction: [0.0, 1.0, 0.0],
        outer_cos: 0.80,
        inner_cos: 0.94,
        color: [3.0; 3],
        casts_shadow: true,
        shadow_exclude: None,
        upper_plane: Some([0.0, 0.0, 1.0]),
    };
    for (bottom, top, should_light) in [(1.3, 1.8, true), (3.0, 3.5, false)] {
        let mut wall = MeshBuilder::new();
        wall.state(None, BlendMode::Opaque, Shading::Prelit);
        wall.flat_quad(
            [
                Vec3::new(-1.0, 0.0, bottom),
                Vec3::new(1.0, 0.0, bottom),
                Vec3::new(1.0, 0.0, top),
                Vec3::new(-1.0, 0.0, top),
            ],
            prelit([0.12; 3], 1.0),
        );
        let receiver = [Instance::new(wall.finish().upload(&mut r, "raised beam receiver"), Mat4::IDENTITY)];
        r.set_local_lights(&[]);
        let baseline = r.capture(240, 160, &frame, &receiver).unwrap();
        r.set_local_lights(&[light]);
        let lit = r.capture(240, 160, &frame, &receiver).unwrap();
        let brighter = baseline
            .as_chunks::<4>()
            .0
            .iter()
            .zip(lit.as_chunks::<4>().0)
            .filter(|(a, b)| b[0] > a[0].saturating_add(8))
            .count();
        if should_light {
            assert!(brighter > 30, "vehicle-height receiver must be illuminated: {brighter}");
            continue;
        }
        assert_eq!(baseline, lit, "high overhead receiver stays outside the beam");
    }
}

#[test]
#[ignore = "requires GPU"]
fn many_lights_keep_illumination_when_shadow_budget_and_resolution_change() {
    let _gpu = test_support::serial();
    let mut r = test_support::headless((240, 160)).expect("budget GPU");
    let (frame, ground, casters) = super::tests::scene(&mut r);
    let light = LocalLight {
        position: [1.0, -1.0, 5.0],
        range: 12.0,
        direction: [-0.2, 0.2, -1.0],
        outer_cos: 0.2,
        inner_cos: 0.7,
        color: [0.004; 3],
        casts_shadow: true,
        shadow_exclude: None,
        upper_plane: Some([0.0, 0.0, 1.0]),
    };
    r.set_shadow_casters(Some(&casters));
    // Two dominant beams and eighteen weak lamps: unshadowed fill must not hide the tested shadow.
    let mut lights = [light; 20];
    lights[0].color = [0.8; 3];
    lights[1].color = [0.8; 3];
    r.set_local_lights(&lights);
    let options = r.render_experiments();
    r.set_render_experiments(RenderExperiments { local_shadow_count: 0, ..options });
    let unshadowed = r.capture(240, 160, &frame, &ground).unwrap();
    for (count, resolution) in [(4, 512), (16, 1024), (2, 2048)] {
        r.set_render_experiments(RenderExperiments {
            local_shadow_count: count,
            local_shadow_resolution: resolution,
            ..options
        });
        let shadowed = r.capture(240, 160, &frame, &ground).unwrap();
        assert!(super::tests::dark_pixels(&unshadowed, &shadowed) > 20, "budget {count}/{resolution} casts shadows");
    }
    r.set_render_experiments(RenderExperiments { local_shadow_count: 0, ..options });
    assert_eq!(
        unshadowed,
        r.capture(240, 160, &frame, &ground).unwrap(),
        "zero shadow budget preserves lights and removes stale maps"
    );
    for light in &mut lights[..19] {
        light.color = [0.0; 3];
    }
    lights[19].color = [1.6; 3];
    r.set_local_lights(&lights);
    let last_only = r.capture(240, 160, &frame, &ground).unwrap();
    r.set_local_lights(&[]);
    assert_ne!(
        last_only,
        r.capture(240, 160, &frame, &ground).unwrap(),
        "more than four lights still add illumination"
    );
}

#[test]
#[ignore = "requires GPU"]
fn night_darkens_the_world_without_suppressing_local_light_and_roundtrips() {
    let _gpu = test_support::serial();
    let mut r = test_support::headless((240, 160)).expect("night GPU");
    let (frame, ground, _) = super::tests::scene(&mut r);
    let day = r.capture(240, 160, &frame, &ground).unwrap();
    let options = r.render_experiments();
    r.set_render_experiments(RenderExperiments { experimental_night: true, ..options });
    let night = r.capture(240, 160, &frame, &ground).unwrap();
    assert!(super::tests::dark_pixels(&day, &night) > 200, "night darkens the world even with no lights");
    r.set_shadow_casters(Some(&[]));
    r.set_local_lights(&[LocalLight {
        position: [1.0, -1.0, 5.0],
        range: 12.0,
        direction: [-0.2, 0.2, -1.0],
        outer_cos: 0.2,
        inner_cos: 0.7,
        color: [1.8; 3],
        casts_shadow: true,
        shadow_exclude: None,
        upper_plane: None,
    }]);
    let lit = r.capture(240, 160, &frame, &ground).unwrap();
    assert!(super::tests::dark_pixels(&lit, &night) > 100, "local light still illuminates night");
    r.set_local_lights(&[]);
    r.set_render_experiments(options);
    assert_eq!(day, r.capture(240, 160, &frame, &ground).unwrap());
}

#[test]
#[ignore = "requires GPU"]
fn shadow_distance_removes_far_shadow_work_without_removing_light() {
    let _gpu = test_support::serial();
    let mut r = test_support::headless((240, 160)).expect("shadow distance GPU");
    let (mut frame, ground, casters) = super::tests::scene(&mut r);
    frame.camera_position = Vec3::new(40.0, -6.0, 4.0);
    r.set_shadow_casters(Some(&casters));
    r.set_local_lights(&[LocalLight {
        position: [1.0, -1.0, 5.0],
        range: 12.0,
        direction: [-0.2, 0.2, -1.0],
        outer_cos: 0.2,
        inner_cos: 0.7,
        color: [1.8; 3],
        casts_shadow: true,
        shadow_exclude: None,
        upper_plane: None,
    }]);
    let options = r.render_experiments();
    r.set_render_experiments(RenderExperiments { local_shadow_count: 0, ..options });
    let unshadowed = r.capture(240, 160, &frame, &ground).unwrap();
    r.set_render_experiments(RenderExperiments { local_shadow_count: 1, local_shadow_distance: 20.0, ..options });
    assert_eq!(unshadowed, r.capture(240, 160, &frame, &ground).unwrap(), "out-of-range light remains unshadowed");
    r.set_render_experiments(RenderExperiments { local_shadow_count: 1, local_shadow_distance: 80.0, ..options });
    let shadowed = r.capture(240, 160, &frame, &ground).unwrap();
    assert!(super::tests::dark_pixels(&unshadowed, &shadowed) > 20, "in-range light casts a shadow");
}

#[test]
#[ignore = "requires GPU"]
fn local_highlight_responds_to_the_strength_control() {
    use blackbox_gfx::{DrawRange, MeshDesc, NormalMaterialDesc, NormalModel, PixelFormat, TextureDesc, Vertex};
    let _gpu = test_support::serial();
    let mut r = test_support::headless((240, 160)).expect("local highlight GPU");
    let (frame, _, _) = super::tests::scene(&mut r);
    let texture = |r: &mut Renderer, pixel: [u8; 4]| {
        r.create_texture(&TextureDesc {
            label: "highlight test",
            width: 1,
            height: 1,
            format: PixelFormat::Rgba8,
            mips: vec![&pixel],
        })
    };
    let diffuse = texture(&mut r, [0, 0, 0, 255]);
    let normal = texture(&mut r, [128, 128, 255, 255]);
    let specular = texture(&mut r, [255, 255, 255, 255]);
    let material = r
        .create_normal_material(&NormalMaterialDesc {
            model: NormalModel::Reflective,
            diffuse,
            normal,
            specular,
            reflection: None,
            uv_offset: [0.0; 2],
        })
        .expect("normal material");
    let vertices = [[-2.0, -2.0, 0.0], [-2.0, 2.0, 0.0], [2.0, 2.0, 0.0], [2.0, -2.0, 0.0]].map(|position| Vertex {
        position,
        normal: [0.0, 0.0, 1.0],
        color_bgra: [128, 128, 128, 255],
        uv: [0.0, 0.0],
    });
    let tangents = [[1.0, 0.0, 0.0, 1.0]; 4];
    let mesh = r.create_mesh(&MeshDesc {
        label: "local highlight receiver",
        vertices: &vertices,
        secondary_uvs: None,
        tangents: Some(&tangents),
        indices: &[0, 1, 2, 0, 2, 3],
        draws: vec![DrawRange {
            first_index: 0,
            index_count: 6,
            base_vertex: 0,
            texture: Some(diffuse),
            blend: BlendMode::Opaque,
            shading: Shading::WorldNormal(material),
            two_sided: true,
        }],
    });
    let receiver = [Instance::new(mesh, Mat4::IDENTITY)];
    r.set_local_lights(&[LocalLight {
        position: [-3.0, 6.0, 4.0],
        range: 20.0,
        direction: [3.0, -6.0, -4.0],
        outer_cos: 0.2,
        inner_cos: 0.7,
        color: [4.0; 3],
        casts_shadow: true,
        shadow_exclude: None,
        upper_plane: None,
    }]);
    let options = RenderExperiments { original_world_materials: true, ..r.render_experiments() };
    r.set_render_experiments(RenderExperiments { local_shadow_count: 0, local_specular_strength: 0.0, ..options });
    let off = r.capture(240, 160, &frame, &receiver).unwrap();
    r.set_render_experiments(RenderExperiments { local_shadow_count: 0, local_specular_strength: 1.0, ..options });
    let on = r.capture(240, 160, &frame, &receiver).unwrap();
    assert!(super::tests::dark_pixels(&on, &off) > 10, "reflective pixels receive a local highlight");
    r.set_render_experiments(RenderExperiments { local_shadow_count: 0, local_specular_strength: 0.0, ..options });
    assert_eq!(
        off,
        r.capture(240, 160, &frame, &receiver).unwrap(),
        "zero strength restores the original material response"
    );
}

#[test]
#[ignore = "requires GPU"]
fn grid_keeps_lights_beyond_the_old_64_light_limit() {
    let _gpu = test_support::serial();
    let mut r = test_support::headless((240, 160)).expect("local light GPU");
    let (frame, ground, _) = super::tests::scene(&mut r);
    r.set_render_experiments(RenderExperiments { local_shadow_count: 0, ..r.render_experiments() });
    let off = r.capture(240, 160, &frame, &ground).unwrap();
    let remote = LocalLight {
        position: [100.0, 100.0, 5.0],
        range: 4.0,
        direction: [0.0, 0.0, -1.0],
        outer_cos: 0.2,
        inner_cos: 0.7,
        color: [1.0; 3],
        casts_shadow: true,
        shadow_exclude: None,
        upper_plane: None,
    };
    let mut lights = vec![remote; 64];
    lights.push(LocalLight {
        position: [1.0, -1.0, 5.0],
        range: 12.0,
        direction: [-0.2, 0.2, -1.0],
        color: [4.0; 3],
        ..remote
    });
    r.set_local_lights(&lights);
    let on = r.capture(240, 160, &frame, &ground).unwrap();
    assert!(super::tests::dark_pixels(&on, &off) > 20, "65th light must reach the receiver");
}

#[test]
#[ignore = "requires GPU"]
fn night_unlit_surfaces_still_respond_to_local_lamps() {
    let _gpu = test_support::serial();
    let mut r = test_support::headless((240, 160)).expect("night surface GPU");
    let (mut frame, _, _) = super::tests::scene(&mut r);
    frame.surface_light = Some(blackbox_gfx::SurfaceLight {
        shadow_strength: 1.0,
        specular: [0.0; 3],
        specular_power: 1.0,
        reflection_scale: 0.0,
        reflective_specular_scale: 1.0,
        ambient: [0.2; 3],
        ambient_alpha: 1.0,
        diffuse: [0.1; 3],
    });
    let mut mesh = MeshBuilder::new();
    mesh.state(None, BlendMode::Opaque, Shading::WorldNoFog);
    mesh.flat_quad(
        [
            Vec3::new(-15.0, -15.0, 0.0),
            Vec3::new(15.0, -15.0, 0.0),
            Vec3::new(15.0, 15.0, 0.0),
            Vec3::new(-15.0, 15.0, 0.0),
        ],
        prelit([0.5; 3], 1.0),
    );
    let receiver = [Instance::new(mesh.finish().upload(&mut r, "night unlit receiver"), Mat4::IDENTITY)];
    r.set_render_experiments(RenderExperiments {
        original_world_materials: true,
        experimental_night: true,
        local_shadow_count: 0,
        ..Default::default()
    });
    let dark = r.capture(240, 160, &frame, &receiver).unwrap();
    r.set_local_lights(&[LocalLight {
        position: [1.0, -1.0, 5.0],
        range: 12.0,
        direction: [-0.2, 0.2, -1.0],
        outer_cos: 0.2,
        inner_cos: 0.7,
        color: [1.8; 3],
        casts_shadow: true,
        shadow_exclude: None,
        upper_plane: None,
    }]);
    let lit = r.capture(240, 160, &frame, &receiver).unwrap();
    assert!(
        super::tests::dark_pixels(&lit, &dark) > 100,
        "local lights must illuminate opaque unlit surfaces at night"
    );
}

#[test]
#[ignore = "requires GPU"]
fn night_blend_changes_smoothly_and_restores_manual_mode() {
    let _gpu = test_support::serial();
    let mut r = test_support::headless((240, 160)).expect("cycle GPU");
    let (frame, ground, _) = super::tests::scene(&mut r);
    r.set_night_blend(Some(0.0));
    let day = r.capture(240, 160, &frame, &ground).unwrap();
    r.set_night_blend(Some(0.5));
    let dusk = r.capture(240, 160, &frame, &ground).unwrap();
    r.set_night_blend(Some(1.0));
    let night = r.capture(240, 160, &frame, &ground).unwrap();
    assert!(super::tests::dark_pixels(&day, &dusk) > 100);
    assert!(super::tests::dark_pixels(&dusk, &night) > 100);
    r.set_night_blend(None);
    assert_eq!(day, r.capture(240, 160, &frame, &ground).unwrap());
}

#[test]
#[ignore = "requires GPU"]
fn explicitly_unshadowed_local_lights_do_not_cast_stale_shadows() {
    let _gpu = test_support::serial();
    let mut r = test_support::headless((240, 160)).expect("local shadow GPU");
    let (frame, ground, casters) = super::tests::scene(&mut r);
    r.set_shadow_casters(Some(&casters));
    let light = LocalLight {
        position: [1.0, -1.0, 5.0],
        range: 12.0,
        direction: [-0.2, 0.2, -1.0],
        outer_cos: 0.2,
        inner_cos: 0.7,
        color: [2.0; 3],
        upper_plane: None,
        casts_shadow: true,
        shadow_exclude: None,
    };
    let opts = RenderExperiments { local_shadow_count: 1, ..Default::default() };
    r.set_render_experiments(opts);
    r.set_local_lights(&[light]);
    let shadowed = r.capture(240, 160, &frame, &ground).unwrap();
    r.set_local_lights(&[LocalLight { casts_shadow: false, ..light }]);
    let unshadowed = r.capture(240, 160, &frame, &ground).unwrap();
    assert!(super::tests::dark_pixels(&unshadowed, &shadowed) > 20);
    r.set_render_experiments(RenderExperiments { local_shadow_count: 0, ..opts });
    assert_eq!(unshadowed, r.capture(240, 160, &frame, &ground).unwrap());
}
