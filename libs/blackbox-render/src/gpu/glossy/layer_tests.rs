//! Synthetic material isolation and shadow/linear-probe contracts.
use super::*;
use blackbox_gfx::SurfaceKind;

fn layer(kind: SurfaceKind, roughness: f32, coat: f32) -> GlossyMaterial {
    GlossyMaterial {
        surface: Some(GlossySurface { kind, roughness, coat }),
        envmap_min: 0.0,
        specular_min: [0.0; 3],
        ..material(roughness)
    }
}

fn environment(r: &mut Renderer, value: u8) {
    let face = [value, value, value, 255].repeat(64);
    r.set_environment_faces(8, [&face; 6]);
}

#[test]
#[ignore = "needs a GPU"]
fn shadowed_car_diffuse_retains_color_without_inventing_fill_or_lifting_direct_light() {
    let _serial = test_support::serial();
    let Some(mut r) = test_support::headless(SIZE) else { return };
    environment(&mut r, 0);
    let desc = GlossyMaterial { diffuse_min: [0.4, 0.18, 0.08, 1.0], ..layer(SurfaceKind::Diffuse, 0.65, 0.0) };
    let opted = mesh_draw(&mut r, desc, 32, BlendMode::Opaque);
    let old = mesh_draw(&mut r, GlossyMaterial { surface: None, ..desc }, 32, BlendMode::Opaque);
    let lighting = LightingRig {
        lights: [
            DirectionalLight::new(Vec3::Z, Vec3::ZERO),
            DirectionalLight::new(Vec3::Z, Vec3::splat(0.25)),
            DirectionalLight::new(Vec3::Z, Vec3::ZERO),
        ],
        ambient: Vec3::ZERO,
        ..Default::default()
    };
    r.set_lighting_rig(&lighting);
    let improved = r.capture(96, 96, &frame(), &opted).unwrap();
    let legacy = r.capture(96, 96, &frame(), &old).unwrap();
    assert!(
        center(&improved) > center(&legacy) * 2 + 2,
        "indirect color: {} -> {}",
        center(&legacy),
        center(&improved)
    );
    rig(&mut r, 0.0);
    assert_eq!(center(&r.capture(96, 96, &frame(), &opted).unwrap()), 0, "zero light must stay black");
    rig(&mut r, 0.7);
    assert_eq!(
        r.capture(96, 96, &frame(), &opted).unwrap(),
        r.capture(96, 96, &frame(), &old).unwrap(),
        "direct diffuse is preserved when no indirect light is present"
    );
}

#[test]
#[ignore = "needs a GPU"]
fn carbon_metal_and_rubber_have_distinct_coat_and_reflection_responses() {
    let _serial = test_support::serial();
    let Some(mut r) = test_support::headless(SIZE) else { return };
    rig(&mut r, 0.0);
    environment(&mut r, 200);
    r.set_render_experiments(RenderExperiments { clearcoat: true, ..Default::default() });
    let mut values = Vec::new();
    for (kind, roughness, coat) in
        [(SurfaceKind::Conductor, 0.27, 0.0), (SurfaceKind::Composite, 0.38, 0.85), (SurfaceKind::Rubber, 0.88, 0.0)]
    {
        let instances = mesh(&mut r, layer(kind, roughness, coat));
        let on = r.capture(96, 96, &frame(), &instances).unwrap();
        values.push(center(&on));
        r.set_render_experiments(RenderExperiments::default());
        let off = r.capture(96, 96, &frame(), &instances).unwrap();
        if kind != SurfaceKind::Composite {
            assert_eq!(on, off, "bare metal/rubber have no extra coat");
        }
        if kind == SurfaceKind::Composite {
            assert!(center(&on) > center(&off) + 5);
        }
        r.set_render_experiments(RenderExperiments { clearcoat: true, ..Default::default() });
    }
    assert!(values[0] > values[1] + 30 && values[1] > values[2] + 5, "metal/carbon/rubber: {values:?}");
}

#[test]
#[ignore = "needs a GPU"]
fn glass_preserves_tint_and_strengthens_reflections_at_grazing_without_global_coat() {
    let _serial = test_support::serial();
    let Some(mut r) = test_support::headless(SIZE) else { return };
    rig(&mut r, 0.0);
    r.set_lighting_rig(&LightingRig {
        ambient: Vec3::splat(0.4),
        ..LightingRig { lights: [DirectionalLight::new(Vec3::Z, Vec3::ZERO); 3], ..Default::default() }
    });
    environment(&mut r, 180);
    let desc = GlossyMaterial { diffuse_min: [0.1, 0.7, 0.1, 0.4], ..layer(SurfaceKind::Glass, 0.10, 0.0) };
    let mut instances = mesh_draw(&mut r, desc, 128, BlendMode::AlphaBlend);
    let front = r.capture(96, 96, &frame(), &instances).unwrap();
    let at = (48 * 96 + 48) * 4;
    assert!(front[at + 1] > front[at] + 15, "green tint must survive: {:?}", &front[at..at + 4]);
    r.set_render_experiments(RenderExperiments { clearcoat: true, ..Default::default() });
    assert_eq!(front, r.capture(96, 96, &frame(), &instances).unwrap());
    instances[0].transform = Mat4::from_rotation_y(70f32.to_radians());
    assert!(center(&r.capture(96, 96, &frame(), &instances).unwrap()) > center(&front) + 20);
}

#[test]
#[ignore = "needs a GPU"]
fn new_layers_filter_probe_energy_in_linear_light() {
    let _serial = test_support::serial();
    let Some(mut r) = test_support::headless(SIZE) else { return };
    rig(&mut r, 0.0);
    let instances = mesh(&mut r, layer(SurfaceKind::Conductor, 0.8, 0.0));
    environment(&mut r, 125);
    let uniform = center(&r.capture(96, 96, &frame(), &instances).unwrap());
    let face: Vec<_> = (0..128 * 128)
        .flat_map(|i| match ((i % 128) / 4 + (i / 128) / 4) & 1 {
            0 => [20u8, 20, 20, 255],
            _ => [230, 230, 230, 255],
        })
        .collect();
    r.set_environment_faces(128, [&face; 6]);
    let checker = r.capture(96, 96, &frame(), &instances).unwrap();
    assert!(
        center(&checker) > uniform + 12,
        "linear averaging must retain bright samples: {uniform} -> {}",
        center(&checker)
    );
    assert_eq!(checker, r.capture(96, 96, &frame(), &instances).unwrap(), "fixed inputs must not shimmer");
}

#[test]
#[ignore = "needs a GPU"]
fn paint_layers_are_distinct_and_all_classes_restore_legacy_and_ignore_original_modes() {
    let _serial = test_support::serial();
    let Some(mut r) = test_support::headless(SIZE) else { return };
    rig(&mut r, 0.0);
    environment(&mut r, 150);
    let mut paint_values = Vec::new();
    for kind in [
        SurfaceKind::Coated,
        SurfaceKind::Metallic,
        SurfaceKind::Pearl,
        SurfaceKind::Conductor,
        SurfaceKind::Glass,
        SurfaceKind::Composite,
        SurfaceKind::PaintedMetal,
        SurfaceKind::Rubber,
    ] {
        let desc = GlossyMaterial { diffuse_min: [0.5, 0.2, 0.1, 1.0], ..layer(kind, 0.32, 0.7) };
        let instances = mesh(&mut r, desc);
        r.set_render_experiments(RenderExperiments { clearcoat: true, ..Default::default() });
        let response = r.capture(96, 96, &frame(), &instances).unwrap();
        if matches!(kind, SurfaceKind::Coated | SurfaceKind::Metallic | SurfaceKind::Pearl) {
            paint_values.push(center(&response));
        }
        r.set_render_experiments(RenderExperiments { paint_response: false, clearcoat: true, ..Default::default() });
        let old = mesh(&mut r, GlossyMaterial { surface: None, ..desc });
        assert_eq!(r.capture(96, 96, &frame(), &instances).unwrap(), r.capture(96, 96, &frame(), &old).unwrap());
        r.set_render_experiments(RenderExperiments { clearcoat: true, ..Default::default() });
        for model in [GlossyModel::VertexLit, GlossyModel::Shadowed] {
            let original = mesh(&mut r, GlossyMaterial { model, ..desc });
            let reference = mesh(&mut r, GlossyMaterial { model, surface: None, ..desc });
            assert_eq!(
                r.capture(96, 96, &frame(), &original).unwrap(),
                r.capture(96, 96, &frame(), &reference).unwrap()
            );
        }
    }
    assert!(
        paint_values[1] > paint_values[0] + 3 && paint_values[2] > paint_values[0] + 3,
        "paint layers: {paint_values:?}"
    );
}
