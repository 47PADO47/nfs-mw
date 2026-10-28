use super::{Gpu, near, validate};
use crate::gpu::post::{PostChain, grade};
use blackbox_gfx::{FocusFilter, ScreenGrade};

#[test]
fn the_grade_shader_and_uniform_validate() {
    let module = validate("screen grade", &grade::source());
    assert_eq!(module.entry_points.len(), 2);
    let uniform = module.global_variables.iter().find(|(_, g)| g.space == wgpu::naga::AddressSpace::Uniform).unwrap().1;
    assert_eq!(module.types[uniform.ty].inner.size(module.to_ctx()), 32);
}

#[test]
fn the_parameter_block_carries_the_edge_the_inner_distance_the_tint_and_the_contrast() {
    let g = ScreenGrade { edge: [0.1, 0.2, 0.3], inner: 0.6, tint: [1.0, 0.9, 0.8], contrast: 1.2 };
    let p = grade::params(&g);
    assert_eq!((p.a, p.b), ([0.1, 0.2, 0.3, 0.6], [1.0, 0.9, 0.8, 1.2]));
    let dirty = ScreenGrade { contrast: f32::NAN, ..g };
    assert_eq!(grade::params(&dirty).b[3], 1.0, "a value that is not a number is neutral");
}

#[test]
#[ignore = "needs a Vulkan GPU"]
fn grade_vulkan() {
    check(wgpu::Backends::VULKAN);
}

fn check(backend: wgpu::Backends) {
    let _guard = crate::gpu::test_support::serial();
    let gpu = Gpu::new(backend);
    let mut chain = PostChain::new(&gpu.device, None);
    // A neutral grade is not in the chain at all.
    chain.set_focus_filter(&gpu.device, FocusFilter::default());
    assert!(chain.names().is_empty());
    // A tint multiplies a flat frame; with the vignette off at the edge (edge 1) the whole frame is the same.
    let tint = ScreenGrade { tint: [0.5, 1.0, 1.0], ..Default::default() };
    chain.set_focus_filter(&gpu.device, FocusFilter { grade: tint, ..Default::default() });
    assert_eq!(chain.names(), [grade::NAME]);
    let body = "return vec4<f32>(0.6,0.4,0.2,1.0);";
    let tinted = gpu.run(&mut chain, (64, 64), body);
    assert!(tinted.iter().all(|p| near(*p, [77, 102, 51, 255], 2)), "{:?}", tinted[0]);
    // A dark edge leaves the centre and darkens the corner.
    let dark = ScreenGrade { edge: [0.0; 3], inner: 0.5, ..Default::default() };
    chain.set_focus_filter(&gpu.device, FocusFilter { grade: dark, ..Default::default() });
    let frame = gpu.run(&mut chain, (64, 64), body);
    assert!(near(frame[32 * 64 + 32], [153, 102, 51, 255], 2), "{:?}", frame[32 * 64 + 32]);
    assert!(near(frame[0], [0, 0, 0, 255], 8), "{:?}", frame[0]);
    // Neutral again takes it out.
    chain.set_focus_filter(&gpu.device, FocusFilter::default());
    assert!(chain.names().is_empty());
}
