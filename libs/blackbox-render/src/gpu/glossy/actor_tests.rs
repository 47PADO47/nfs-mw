//! Background cargo retains authored diffuse energy instead of the subject's fill uplift.
use super::*;
use blackbox_gfx::{SurfaceKind, SurfaceLight};

#[test]
#[ignore = "needs a GPU"]
fn background_diffuse_matches_authored_fill_without_subject_uplift() {
    let _serial = test_support::serial();
    let mut r = Renderer::headless(SIZE, test_support::options()).expect("selected GPU API");
    rig(&mut r, 0.0);
    r.set_render_experiments(RenderExperiments {
        car_skylight: true,
        sky_strength: 1.0,
        ground_strength: 1.0,
        clearcoat: true,
        ..Default::default()
    });
    let mut view = frame();
    view.surface_light = Some(SurfaceLight {
        ambient: [0.2; 3],
        ambient_alpha: 1.0,
        diffuse: [0.0; 3],
        specular: [0.0; 3],
        specular_power: 16.0,
        reflection_scale: 1.0,
        reflective_specular_scale: 1.0,
        shadow_strength: 1.0,
    });
    let material = GlossyMaterial {
        background_actor: true,
        surface: Some(GlossySurface { kind: SurfaceKind::Diffuse, roughness: 0.85, coat: 0.0 }),
        diffuse_min: [0.5, 0.5, 0.5, 1.0],
        diffuse_range: [0.0; 4],
        specular_min: [0.0; 3],
        specular_range: [0.0; 3],
        envmap_min: 0.0,
        envmap_range: 0.0,
        ..Default::default()
    };
    let actor = mesh(&mut r, material);
    let subject = mesh(&mut r, GlossyMaterial { background_actor: false, ..material });
    let baseline = mesh(&mut r, GlossyMaterial { surface: None, ..material });
    let mut actors = Vec::new();
    let mut subjects = Vec::new();
    for colour in [[0, 0, 0, 255], [255, 255, 255, 255], [30, 80, 255, 255]] {
        let face = colour.repeat(64);
        r.set_environment_faces(8, [&face; 6]);
        let actual = center(&r.capture(96, 96, &view, &actor).unwrap());
        assert_eq!(
            actual,
            center(&r.capture(96, 96, &view, &baseline).unwrap()),
            "cargo must retain authored diffuse energy"
        );
        actors.push(actual);
        subjects.push(center(&r.capture(96, 96, &view, &subject).unwrap()));
    }
    assert!(actors[1] > actors[0] + 20, "restored capture fill must respond: {actors:?}");
    assert!(subjects[2] > actors[2] + 5, "primary subject keeps its existing uplift: {subjects:?}, {actors:?}");
    rig(&mut r, 0.7);
    r.set_environment_faces(8, [&[0, 0, 0, 255].repeat(64); 6]);
    assert_eq!(
        center(&r.capture(96, 96, &view, &actor).unwrap()),
        center(&r.capture(96, 96, &view, &baseline).unwrap()),
        "matte cargo must retain authored sunlight energy"
    );
}
