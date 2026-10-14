//! AO changes indirect illumination, with direct light, highlights and reflections preserved.
use super::tests::scene_shading;
use crate::gpu::test_support;
use blackbox_gfx::{
    AoQuality, AoSettings, DirectionalLight, GlossyMaterial, GraphicsSettings, LightingRig, RenderBackend, Shading,
    SurfaceLight,
};
use glam::Vec3;

#[test]
#[ignore = "needs a GPU"]
fn reused_prepass_depth_matches_separate_depth_for_world_and_glossy_surfaces() {
    let _serial = test_support::serial();
    let size = (319, 237);
    let Some(mut r) = test_support::headless(size) else { return };
    let material = r.create_glossy_material(&GlossyMaterial::default());
    for shading in [Shading::Lit, Shading::Prelit, Shading::Glossy(material)] {
        let (frame, instances) = scene_shading(&mut r, size, true, shading);
        for quality in [AoQuality::Low, AoQuality::Medium, AoQuality::High] {
            r.apply_graphics(&ao(quality));
            r.set_prepass_depth_reuse(true);
            let shared = r.capture(size.0, size.1, &frame, &instances).unwrap();
            r.set_prepass_depth_reuse(false);
            assert_eq!(
                shared,
                r.capture(size.0, size.1, &frame, &instances).unwrap(),
                "depth reuse changed {shading:?} with {quality}"
            );
        }
    }
}

#[test]
#[ignore = "needs a GPU"]
fn ao_reuses_bindings_but_rebinds_replaced_same_size_inputs() {
    let _serial = test_support::serial();
    let size = (319, 237);
    let Some(mut r) = test_support::headless(size) else { return };
    let (frame, instances) = scene_shading(&mut r, size, true, Shading::Lit);
    r.apply_graphics(&ao(AoQuality::Medium));
    r.render(&frame, &instances).unwrap();
    let groups = r.ao.as_ref().unwrap().inputs.as_ref().unwrap().groups.clone();
    r.render(&frame, &instances).unwrap();
    assert_eq!(groups, r.ao.as_ref().unwrap().inputs.as_ref().unwrap().groups);
    let before = r.capture(size.0, size.1, &frame, &instances).unwrap();
    r.surfaces.as_mut().unwrap().targets = crate::gpu::surface::targets::Targets::new(&r.device, size);
    let after = r.capture(size.0, size.1, &frame, &instances).unwrap();
    assert_ne!(groups, r.ao.as_ref().unwrap().inputs.as_ref().unwrap().groups);
    assert_eq!(before, after, "a replacement depth/normal image must not leave stale bindings");
}

#[test]
#[ignore = "needs a GPU"]
fn shared_prepass_resizes_and_switches_both_visibility_masks_without_history() {
    let _serial = test_support::serial();
    let Some(mut r) = test_support::headless((319, 237)) else { return };
    let (mut frame, instances) = scene_shading(&mut r, (319, 237), true, Shading::Lit);
    for size in [(319, 237), (511, 255), (127, 93)] {
        r.resize(size.0, size.1);
        frame.projection = blackbox_gfx::Projection::PerspectiveInfiniteReverse {
            fov_y: 1.0,
            aspect: size.0 as f32 / size.1 as f32,
            near: 0.1,
        };
        for contact in [false, true, false] {
            r.set_render_experiments(blackbox_gfx::RenderExperiments {
                sun_shadows: true,
                contact_shadows: contact,
                shadow_resolution: 512,
                ..Default::default()
            });
            r.apply_graphics(&GraphicsSettings {
                ao: AoSettings { quality: AoQuality::Medium, ..Default::default() },
                post: blackbox_gfx::PostSettings {
                    antialiasing: blackbox_gfx::Antialiasing::Smaa,
                    ..Default::default()
                },
                ..Default::default()
            });
            let image = r.capture(size.0, size.1, &frame, &instances).unwrap();
            assert_eq!(image, r.capture(size.0, size.1, &frame, &instances).unwrap());
            assert_eq!(r.surfaces.as_ref().unwrap().targets.size, size);
            assert_eq!(r.shared.shadows.ambient.texture().width(), size.0);
        }
        r.apply_graphics(&ao(AoQuality::Off));
        r.set_render_experiments(Default::default());
        r.capture(size.0, size.1, &frame, &instances).unwrap();
        assert!(r.surfaces.is_none() && r.ao.is_none());
        assert_eq!(r.shared.shadows.ambient.texture().width(), 1);
    }
}

fn ao(quality: AoQuality) -> GraphicsSettings {
    GraphicsSettings { ao: AoSettings { quality, ..Default::default() }, ..Default::default() }
}

#[test]
#[ignore = "needs a GPU"]
fn ao_attenuates_ambient_but_preserves_direct_light_and_glossy_reflections() {
    let _serial = test_support::serial();
    let size = (320, 240);
    let Some(mut r) = test_support::headless(size) else { return };
    let (mut frame, instances) = scene_shading(&mut r, size, true, Shading::Lit);
    frame.surface_light = Some(SurfaceLight { ambient: [0.0; 3], diffuse: [0.6; 3] });
    let direct = r.capture(size.0, size.1, &frame, &instances).unwrap();
    r.apply_graphics(&ao(AoQuality::High));
    assert_eq!(direct, r.capture(size.0, size.1, &frame, &instances).unwrap(), "AO leaves direct-only surfaces intact");
    frame.surface_light = Some(SurfaceLight { ambient: [0.6; 3], diffuse: [0.0; 3] });
    let ambient_on = r.capture(size.0, size.1, &frame, &instances).unwrap();
    r.apply_graphics(&ao(AoQuality::Off));
    let ambient_off = r.capture(size.0, size.1, &frame, &instances).unwrap();
    assert!(
        ambient_off.iter().zip(&ambient_on).filter(|(a, b)| u16::from(**a) > u16::from(**b) + 2).count() > 100,
        "ambient contacts darken"
    );

    r.set_lighting_rig(&LightingRig { lights: [DirectionalLight::new(Vec3::Z, Vec3::ZERO); 3], ambient: Vec3::ZERO });
    let face = vec![80u8; 8 * 8 * 4];
    r.set_environment_faces(8, [&face; 6]);
    let material = r.create_glossy_material(&GlossyMaterial {
        diffuse_min: [0.0, 0.0, 0.0, 1.0],
        specular_min: [1.0; 3],
        envmap_min: 1.0,
        ..Default::default()
    });
    let (frame, instances) = scene_shading(&mut r, size, true, Shading::Glossy(material));
    let reflection = r.capture(size.0, size.1, &frame, &instances).unwrap();
    r.apply_graphics(&ao(AoQuality::High));
    assert_eq!(
        reflection,
        r.capture(size.0, size.1, &frame, &instances).unwrap(),
        "AO leaves reflection and glint terms intact"
    );
}
