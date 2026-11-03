//! Depth clipping and lifetime of local-light haze volumes.
use blackbox_gfx::{
    BlendMode, EffectLayer, FrameParams, Instance, LightHaze, Projection, RenderBackend, RenderExperiments, Shading,
};
use blackbox_gfx_testkit::mesh::{MeshBuilder, prelit};
use glam::{Mat4, Quat, Vec3};

#[test]
#[ignore = "needs a GPU"]
fn haze_is_depth_clipped_and_can_be_disabled() {
    let _gpu = super::test_support::serial();
    let mut renderer = super::test_support::headless((128, 128)).expect("GPU adapter");
    let mut layer = EffectLayer::default();
    layer.light_haze.push(LightHaze {
        to_world: Mat4::from_scale_rotation_translation(
            Vec3::new(2.0, 2.0, 2.0),
            Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            Vec3::X * 4.0,
        ),
        color: [1.0, 0.8, 0.5],
        intensity: 1.0,
    });
    let frame = FrameParams {
        view: glam::camera::rh::view::look_at_mat4(Vec3::ZERO, Vec3::X, Vec3::Z),
        projection: Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: 1.0, near: 0.1 },
        camera_position: Vec3::ZERO,
        light_dir: Vec3::X,
        clear_color: [0.0; 3],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,
        shadow_tuning: None,
    };
    let mut wall = MeshBuilder::new();
    wall.state(None, BlendMode::Opaque, Shading::Prelit);
    wall.cuboid(Vec3::new(-0.01, -2.0, -2.0), Vec3::new(0.01, 2.0, 2.0), prelit([0.0; 3], 1.0));
    let wall = wall.finish().upload(&mut renderer, "haze occluder");
    let mut shot = |layer: &EffectLayer, x: Option<f32>| {
        let instances: Vec<_> =
            x.map(|x| Instance::new(wall, Mat4::from_translation(Vec3::X * x))).into_iter().collect();
        renderer.set_effects(layer);
        renderer.render(&frame, &instances).unwrap();
        renderer.read_output().unwrap().unwrap()
    };
    let energy =
        |image: &blackbox_gfx::RgbaImage| image.rgba.as_chunks::<4>().0.iter().map(|p| u64::from(p[0])).sum::<u64>();
    let full = energy(&shot(&layer, None));
    let partial = energy(&shot(&layer, Some(4.0)));
    let hidden = energy(&shot(&layer, Some(2.5)));
    assert!(full > 100, "visible haze must contribute light");
    assert!(partial > 0 && partial < full, "a surface inside the volume must clip it");
    assert_eq!(hidden, 0, "an opaque foreground surface must hide the haze");
    layer.light_haze[0].to_world *= Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0));
    let reflected = energy(&shot(&layer, None));
    assert!(full.abs_diff(reflected) < full / 20, "mirrored fixtures must retain their haze");
    layer.light_haze.clear();
    assert_eq!(energy(&shot(&layer, None)), 0, "clearing removes the effect");
}

#[test]
#[ignore = "needs a GPU"]
fn shared_light_fog_clips_to_depth_and_disables() {
    let _gpu = super::test_support::serial();
    let mut renderer = super::test_support::headless((128, 128)).expect("GPU adapter");
    renderer.set_local_lights(&[blackbox_gfx::LocalLight {
        position: [5.0, 0.0, 3.0],
        range: 12.0,
        direction: [0.0, 0.0, -1.0],
        outer_cos: 0.2,
        inner_cos: 0.7,
        color: [5.0, 4.0, 3.0],
        upper_plane: None,
        casts_shadow: false,
        shadow_exclude: None,
    }]);
    let frame = FrameParams {
        view: glam::camera::rh::view::look_at_mat4(Vec3::ZERO, Vec3::X, Vec3::Z),
        projection: Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: 1.0, near: 0.1 },
        camera_position: Vec3::ZERO,
        light_dir: Vec3::X,
        clear_color: [0.0; 3],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,
        shadow_tuning: None,
    };
    let mut wall = MeshBuilder::new();
    wall.state(None, BlendMode::Opaque, Shading::Prelit);
    wall.cuboid(Vec3::new(-0.01, -8.0, -8.0), Vec3::new(0.01, 8.0, 8.0), prelit([0.0; 3], 1.0));
    let wall = wall.finish().upload(&mut renderer, "fog occluder");
    let mut shot = |strength: f32, wall_at: Option<f32>| {
        renderer.set_effects(&EffectLayer { light_fog: [strength, 1.0], ..Default::default() });
        let instances: Vec<_> =
            wall_at.map(|x| Instance::new(wall, Mat4::from_translation(Vec3::X * x))).into_iter().collect();
        renderer.render(&frame, &instances).unwrap();
        renderer.read_output().unwrap().unwrap().rgba.as_chunks::<4>().0.iter().map(|p| u64::from(p[0])).sum::<u64>()
    };
    let full = shot(2.0, None);
    let clipped = shot(2.0, Some(2.0));
    assert!(full > 100, "illuminated shared fog must contribute light");
    assert!(clipped < full, "foreground geometry must clip the fog");
    assert_eq!(shot(0.0, None), 0, "zero strength disables the fog");
}

#[test]
#[ignore = "needs a GPU"]
fn an_unseen_car_light_does_not_change_the_sky() {
    let _gpu = super::test_support::serial();
    let mut renderer = super::test_support::headless((128, 128)).expect("GPU adapter");
    let frame = FrameParams {
        view: glam::camera::rh::view::look_at_mat4(Vec3::ZERO, Vec3::X, Vec3::Z),
        projection: Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: 1.0, near: 0.1 },
        camera_position: Vec3::ZERO,
        light_dir: Vec3::X,
        clear_color: [0.3, 0.4, 0.5],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,
        shadow_tuning: None,
    };
    let layer = EffectLayer { light_fog: [0.7, 0.5], ..Default::default() };
    renderer.set_render_experiments(RenderExperiments { local_light_fog_distance: 415.0, ..Default::default() });
    renderer.set_local_lights(&[]);
    renderer.set_effects(&layer);
    renderer.render(&frame, &[]).unwrap();
    let unlit = renderer.read_output().unwrap().unwrap().pixel(64, 64).unwrap();
    renderer.set_local_lights(&[blackbox_gfx::LocalLight {
        position: [-20.0, 0.0, 0.0],
        range: 5.0,
        direction: [-1.0, 0.0, 0.0],
        outer_cos: 0.15,
        inner_cos: 0.72,
        color: [0.65, 0.016, 0.008],
        upper_plane: None,
        casts_shadow: false,
        shadow_exclude: None,
    }]);
    renderer.set_effects(&layer);
    renderer.render(&frame, &[]).unwrap();
    let lit = renderer.read_output().unwrap().unwrap().pixel(64, 64).unwrap();
    assert!(unlit[..3].iter().zip(&lit).all(|(a, b)| a.abs_diff(*b) <= 1), "sky changed: {unlit:?} -> {lit:?}");
}

#[test]
#[ignore = "needs a GPU"]
fn shared_fog_remains_stable_when_camera_approaches_a_light() {
    let _gpu = super::test_support::serial();
    let mut renderer = super::test_support::headless((128, 128)).expect("GPU adapter");
    renderer.set_local_lights(&[blackbox_gfx::LocalLight {
        position: [35.0, 0.0, 4.0],
        range: 20.0,
        direction: [0.0, 0.0, -1.0],
        outer_cos: 0.2,
        inner_cos: 0.7,
        color: [5.0, 4.0, 3.0],
        upper_plane: None,
        casts_shadow: false,
        shadow_exclude: None,
    }]);
    renderer.set_effects(&EffectLayer { light_fog: [2.0, 1.0], ..Default::default() });
    let mut frame = FrameParams {
        view: Mat4::IDENTITY,
        projection: Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: 1.0, near: 0.1 },
        camera_position: Vec3::ZERO,
        light_dir: Vec3::X,
        clear_color: [0.0; 3],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,
        shadow_tuning: None,
    };
    let mut samples = Vec::new();
    for x in [0.0, 1.5, 3.0, 4.5, 6.0, 7.5, 9.0] {
        frame.camera_position = Vec3::new(x, 0.0, 0.0);
        frame.view =
            glam::camera::rh::view::look_at_mat4(frame.camera_position, frame.camera_position + Vec3::X, Vec3::Z);
        renderer.render(&frame, &[]).unwrap();
        let image = renderer.read_output().unwrap().unwrap();
        samples.push(image.pixel(64, 64).unwrap()[0]);
    }
    let low = *samples.iter().min().unwrap();
    let high = *samples.iter().max().unwrap();
    assert!(low > 1, "test ray must see illuminated fog: {samples:?}");
    // Shortening the path through the medium slightly reduces extinction; that is a smooth change.
    assert!(high - low <= 8, "camera movement must not move sampling bands through a fixed light: {samples:?}");
    assert!(
        samples.windows(2).all(|pair| pair[0].abs_diff(pair[1]) <= 2),
        "fog must not jump between frames: {samples:?}"
    );
}

#[test]
#[ignore = "needs a GPU"]
fn fog_distance_control_reveals_distant_light() {
    let _gpu = super::test_support::serial();
    let mut renderer = super::test_support::headless((64, 64)).expect("GPU adapter");
    renderer.set_local_lights(&[blackbox_gfx::LocalLight {
        position: [300.0, 0.0, 4.0],
        range: 30.0,
        direction: [0.0, 0.0, -1.0],
        outer_cos: 0.2,
        inner_cos: 0.7,
        color: [10.0; 3],
        upper_plane: None,
        casts_shadow: false,
        shadow_exclude: None,
    }]);
    let frame = FrameParams {
        view: glam::camera::rh::view::look_at_mat4(Vec3::ZERO, Vec3::X, Vec3::Z),
        projection: Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: 1.0, near: 0.1 },
        camera_position: Vec3::ZERO,
        light_dir: Vec3::X,
        clear_color: [0.0; 3],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,
        shadow_tuning: None,
    };
    let mut capture = |distance: f32| {
        renderer.set_render_experiments(RenderExperiments { local_light_fog_distance: distance, ..Default::default() });
        renderer.set_effects(&EffectLayer { light_fog: [2.0, 1.0], ..Default::default() });
        renderer.render(&frame, &[]).unwrap();
        renderer.read_output().unwrap().unwrap().rgba.as_chunks::<4>().0.iter().map(|p| u64::from(p[0])).sum::<u64>()
    };
    assert_eq!(capture(200.0), 0, "short range excludes the distant light");
    assert!(capture(500.0) > 20, "extended range includes the distant light");
}

#[test]
#[ignore = "needs a GPU"]
fn fog_budget_can_reach_lights_beyond_thirty_two() {
    let _gpu = super::test_support::serial();
    let mut renderer = super::test_support::headless((64, 64)).expect("GPU adapter");
    let lights: Vec<_> = (0..64)
        .map(|i| blackbox_gfx::LocalLight {
            position: [35.0, if i < 32 { 100.0 } else { 0.0 }, 4.0],
            range: 20.0,
            direction: [0.0, 0.0, -1.0],
            outer_cos: 0.2,
            inner_cos: 0.7,
            color: [0.3; 3],
            upper_plane: None,
            casts_shadow: false,
            shadow_exclude: None,
        })
        .collect();
    renderer.set_local_lights(&lights);
    let frame = FrameParams {
        view: glam::camera::rh::view::look_at_mat4(Vec3::ZERO, Vec3::X, Vec3::Z),
        projection: Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: 1.0, near: 0.1 },
        camera_position: Vec3::ZERO,
        light_dir: Vec3::X,
        clear_color: [0.0; 3],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,
        shadow_tuning: None,
    };
    let mut capture = |count| {
        renderer.set_effects(&EffectLayer::default());
        renderer.set_render_experiments(RenderExperiments { local_light_fog_count: count, ..Default::default() });
        renderer.set_effects(&EffectLayer { light_fog: [2.0, 1.0], ..Default::default() });
        renderer.render(&frame, &[]).unwrap();
        renderer.read_output().unwrap().unwrap().rgba.as_chunks::<4>().0.iter().map(|p| u64::from(p[0])).sum::<u64>()
    };
    assert_eq!(capture(32), 0, "the first 32 lights miss the view");
    assert!(capture(64) > 20, "lights beyond 32 should illuminate the fog");
}

#[test]
#[ignore = "needs a GPU"]
fn covered_regions_reduce_rain_fog_without_disabling_the_lamp() {
    let _gpu = super::test_support::serial();
    let mut renderer = super::test_support::headless((128, 128)).expect("GPU adapter");
    renderer.set_local_lights(&[blackbox_gfx::LocalLight {
        position: [5.0, 0.0, 3.0],
        range: 12.0,
        direction: [0.0, 0.0, -1.0],
        outer_cos: 0.2,
        inner_cos: 0.7,
        color: [5.0, 4.0, 3.0],
        upper_plane: None,
        casts_shadow: false,
        shadow_exclude: None,
    }]);
    let frame = FrameParams {
        view: glam::camera::rh::view::look_at_mat4(Vec3::ZERO, Vec3::X, Vec3::Z),
        projection: Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: 1.0, near: 0.1 },
        camera_position: Vec3::ZERO,
        light_dir: Vec3::X,
        clear_color: [0.0; 3],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,
        shadow_tuning: None,
    };
    let mut layer = EffectLayer { light_fog: [2.0, 1.0], ..Default::default() };
    let mut shot = |layer: &EffectLayer| {
        renderer.set_effects(layer);
        renderer.render(&frame, &[]).unwrap();
        renderer.read_output().unwrap().unwrap().rgba.as_chunks::<4>().0.iter().map(|p| u64::from(p[0])).sum::<u64>()
    };
    let outside = shot(&layer);
    layer.fog_shelters.push(blackbox_gfx::FogShelter {
        bounds: [-100.0, -100.0, 100.0, 100.0],
        ceiling: 10.0,
        polygon: vec![[-100.0, -100.0], [100.0, -100.0], [100.0, 100.0], [-100.0, 100.0]],
    });
    let inside = shot(&layer);
    assert!(outside > inside && inside > 0, "covered fog {inside}, outdoor fog {outside}");
}

#[test]
#[ignore = "needs a GPU"]
fn fog_history_rejects_a_light_that_switches_off() {
    let _gpu = super::test_support::serial();
    let mut renderer = super::test_support::headless((128, 128)).expect("GPU adapter");
    let mut light = blackbox_gfx::LocalLight {
        position: [5.0, 0.0, 3.0],
        range: 12.0,
        direction: [0.0, 0.0, -1.0],
        outer_cos: 0.2,
        inner_cos: 0.7,
        color: [5.0, 4.0, 3.0],
        upper_plane: None,
        casts_shadow: false,
        shadow_exclude: None,
    };
    let frame = FrameParams {
        view: glam::camera::rh::view::look_at_mat4(Vec3::ZERO, Vec3::X, Vec3::Z),
        projection: Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: 1.0, near: 0.1 },
        camera_position: Vec3::ZERO,
        light_dir: Vec3::X,
        clear_color: [0.0; 3],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,
        shadow_tuning: None,
    };
    let layer = EffectLayer { light_fog: [2.0, 1.0], ..Default::default() };
    renderer.set_local_lights(&[light]);
    renderer.set_effects(&layer);
    renderer.render(&frame, &[]).unwrap();
    let on =
        renderer.read_output().unwrap().unwrap().rgba.as_chunks::<4>().0.iter().map(|p| u64::from(p[0])).sum::<u64>();
    light.color = [0.0; 3];
    renderer.set_local_lights(&[light]);
    renderer.set_effects(&layer);
    renderer.render(&frame, &[]).unwrap();
    let off =
        renderer.read_output().unwrap().unwrap().rgba.as_chunks::<4>().0.iter().map(|p| u64::from(p[0])).sum::<u64>();
    assert!(on > 100 && off < on / 20, "switched-off light must not leave a fog trail: {on} -> {off}");
}

#[test]
#[ignore = "needs a GPU"]
fn local_shadow_map_blocks_fog_behind_an_occluder() {
    let _gpu = super::test_support::serial();
    let mut renderer = super::test_support::headless((128, 128)).expect("GPU adapter");
    renderer.set_local_lights(&[blackbox_gfx::LocalLight {
        position: [6.0, 0.0, 5.0],
        range: 12.0,
        direction: [0.0, 0.0, -1.0],
        outer_cos: 0.2,
        inner_cos: 0.7,
        color: [5.0, 4.0, 3.0],
        upper_plane: None,
        casts_shadow: true,
        shadow_exclude: None,
    }]);
    let frame = FrameParams {
        view: glam::camera::rh::view::look_at_mat4(Vec3::ZERO, Vec3::X, Vec3::Z),
        projection: Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: 1.0, near: 0.1 },
        camera_position: Vec3::ZERO,
        light_dir: Vec3::X,
        clear_color: [0.0; 3],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: true,
        shadow_tuning: None,
    };
    let mut blocker = MeshBuilder::new();
    blocker.state(None, BlendMode::Opaque, Shading::Prelit);
    blocker.cuboid(Vec3::new(-2.0, -2.0, -0.1), Vec3::new(2.0, 2.0, 0.1), prelit([0.0; 3], 1.0));
    let mesh = blocker.finish().upload(&mut renderer, "fog shadow blocker");
    let instance = Instance::new(mesh, Mat4::from_translation(Vec3::new(6.0, 0.0, 2.5)));
    let mut shot = |count| {
        renderer.set_render_experiments(RenderExperiments {
            local_shadow_count: count,
            local_shadow_distance: 80.0,
            local_shadow_resolution: 512,
            ..Default::default()
        });
        renderer.set_effects(&EffectLayer { light_fog: [2.0, 1.0], ..Default::default() });
        renderer.render(&frame, &[instance]).unwrap();
        renderer.read_output().unwrap().unwrap().rgba.as_chunks::<4>().0.iter().map(|p| u64::from(p[0])).sum::<u64>()
    };
    let unshadowed = shot(0);
    let shadowed = shot(1);
    assert!(
        unshadowed > 100 && shadowed < unshadowed * 9 / 10,
        "occluder should reduce light in the fog: {unshadowed} -> {shadowed}"
    );
}

#[test]
#[ignore = "needs a GPU"]
fn distant_lamp_fog_changes_smoothly_during_forward_motion() {
    let _gpu = super::test_support::serial();
    let mut renderer = super::test_support::headless((128, 128)).expect("GPU adapter");
    renderer.set_render_experiments(RenderExperiments {
        local_light_fog_distance: 460.0,
        local_light_fog_quality: 2,
        local_shadow_count: 0,
        ..Default::default()
    });
    renderer.set_local_lights(&[blackbox_gfx::LocalLight {
        position: [240.0, 0.0, 6.0],
        // Wide enough to span several fog tiles of this small view (the grid follows the view size).
        range: 40.0,
        direction: [0.0, 0.0, -1.0],
        outer_cos: 0.2,
        inner_cos: 0.7,
        color: [30.0, 25.0, 20.0],
        upper_plane: None,
        casts_shadow: false,
        shadow_exclude: None,
    }]);
    renderer.set_effects(&EffectLayer { light_fog: [2.0, 1.0], ..Default::default() });
    let mut frame = FrameParams {
        view: Mat4::IDENTITY,
        projection: Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: 1.0, near: 0.1 },
        camera_position: Vec3::ZERO,
        light_dir: Vec3::X,
        clear_color: [0.0; 3],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,
        shadow_tuning: None,
    };
    let mut samples = Vec::new();
    for i in 0..80 {
        frame.camera_position = Vec3::new(i as f32 * 0.75, 0.0, 0.0);
        frame.view =
            glam::camera::rh::view::look_at_mat4(frame.camera_position, frame.camera_position + Vec3::X, Vec3::Z);
        renderer.render(&frame, &[]).unwrap();
        let image = renderer.read_output().unwrap().unwrap();
        samples.push(image.pixel(64, 64).unwrap()[0]);
    }
    eprintln!("distant lamp moving-camera samples: {samples:?}");
    assert!(samples.iter().copied().max().unwrap() > 10, "fog must be visible");
    assert!(
        samples.windows(3).all(|v| (i32::from(v[0]) - 2 * i32::from(v[1]) + i32::from(v[2])).abs() <= 3),
        "fixed lamp fog should not flicker between adjacent frames: {samples:?}"
    );

    let mut changing_weather = Vec::new();
    for i in 0..80 {
        frame.camera_position = Vec3::new(i as f32 * 0.75, 0.0, 0.0);
        frame.view =
            glam::camera::rh::view::look_at_mat4(frame.camera_position, frame.camera_position + Vec3::X, Vec3::Z);
        frame.camera_cut = i == 0;
        renderer.set_effects(&EffectLayer { light_fog: [2.0, 0.90 + i as f32 * 0.0005], ..Default::default() });
        renderer.render(&frame, &[]).unwrap();
        let image = renderer.read_output().unwrap().unwrap();
        changing_weather.push(image.pixel(64, 64).unwrap()[0]);
    }
    assert!(
        changing_weather.windows(3).all(|v| (i32::from(v[0]) - 2 * i32::from(v[1]) + i32::from(v[2])).abs() <= 3),
        "gradually changing weather must not reset the fog every frame: {changing_weather:?}"
    );
}
