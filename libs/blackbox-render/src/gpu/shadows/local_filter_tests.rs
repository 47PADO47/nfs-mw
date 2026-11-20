//! The lamp shadow filter radius widens the penumbra without moving or removing the shadow.
use super::*;
use crate::gpu::test_support;
use blackbox_gfx::{LocalLight, RenderBackend, RenderExperiments};
use blackbox_gfx_testkit::mesh::{MeshBuilder, prelit};

const SIZE: (u32, u32) = (240, 160);

/// Per pixel the lamp reaches strongly, the fraction of its light (red channel) that passes the caster.
fn visibility(
    r: &mut Renderer,
    radius: f32,
    lamp: &[LocalLight],
    scene: &(FrameParams, [Instance; 1], Vec<Instance>),
) -> Vec<f32> {
    let (frame, ground, casters) = scene;
    // The smallest map makes a texel span several screen pixels, so the filter's footprint is visible.
    let options = RenderExperiments { local_shadow_resolution: 512, local_shadow_filter: radius, ..Default::default() };
    r.set_render_experiments(options);
    r.set_local_lights(&[]);
    let unlit = r.capture(SIZE.0, SIZE.1, frame, ground.as_slice()).unwrap();
    r.set_local_lights(lamp);
    r.set_shadow_casters(Some(&[]));
    let lit = r.capture(SIZE.0, SIZE.1, frame, ground.as_slice()).unwrap();
    r.set_shadow_casters(Some(casters));
    let shadowed = r.capture(SIZE.0, SIZE.1, frame, ground.as_slice()).unwrap();
    let red = |image: &[u8]| image.as_chunks::<4>().0.iter().map(|p| f32::from(p[0])).collect::<Vec<_>>();
    let (unlit, lit, shadowed) = (red(&unlit), red(&lit), red(&shadowed));
    (0..unlit.len())
        .filter(|&i| lit[i] - unlit[i] > 40.0)
        .map(|i| (shadowed[i] - unlit[i]) / (lit[i] - unlit[i]))
        .collect()
}

/// Pixels the caster shadows partly: the penumbra.
fn penumbra(visibility: &[f32]) -> usize {
    visibility.iter().filter(|&&v| v > 0.1 && v < 0.9).count()
}

#[test]
#[ignore = "requires GPU"]
fn filter_radius_softens_the_lamp_shadow_edge_and_keeps_its_area() {
    let _gpu = test_support::serial();
    let mut r = test_support::headless(SIZE).expect("lamp filter GPU");
    let (frame, _, casters) = super::tests::scene(&mut r);
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
    let ground = [Instance::new(m.finish().upload(&mut r, "dark lamp receiver"), Mat4::IDENTITY)];
    let lamp = [LocalLight {
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
    let scene = (frame, ground, casters);
    let hard = visibility(&mut r, 0.0, &lamp, &scene);
    let soft = visibility(&mut r, 1.0, &lamp, &scene);
    let wide = visibility(&mut r, 3.0, &lamp, &scene);
    let (hard_edge, soft_edge, wide_edge) = (penumbra(&hard), penumbra(&soft), penumbra(&wide));
    assert!(soft_edge > hard_edge * 2, "radius 1 must soften the edge: {hard_edge} -> {soft_edge}");
    assert!(wide_edge * 2 > soft_edge * 3, "radius 3 must soften it further: {soft_edge} -> {wide_edge}");
    // Filtering redistributes the shadow; it must not grow, shrink or drop it.
    let total = |v: &[f32]| v.iter().map(|&v| 1.0 - v.clamp(0.0, 1.0)).sum::<f32>();
    let (hard_total, soft_total, wide_total) = (total(&hard), total(&soft), total(&wide));
    assert!(hard_total > 100.0, "the caster must shadow the lamp: {hard_total}");
    for (radius, value) in [(1, soft_total), (3, wide_total)] {
        let ratio = value / hard_total;
        assert!((0.95..1.05).contains(&ratio), "radius {radius} keeps the shadow's area: {ratio}");
    }
}
