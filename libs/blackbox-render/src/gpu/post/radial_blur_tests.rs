//! Checks of the radial blur in the post chain on a real backend: where it sits, that it leaves the chain at zero
//! strength, and that a flat frame stays flat under it (its weights add up to one).

use super::PostChain;
use super::radial_blur::NAME;
use super::tests::{Gpu, near};
use blackbox_gfx::{Antialiasing, FocusFilter, PostSettings, RgbaImage};

#[test]
#[ignore = "needs a Vulkan GPU"]
fn radial_blur_vulkan() {
    check(wgpu::Backends::VULKAN);
}

fn check(backend: wgpu::Backends) {
    let _lock = crate::gpu::test_support::serial();
    let gpu = Gpu::new(backend);
    let mut chain = PostChain::new(&gpu.device, None);
    assert!(chain.names().is_empty());

    // Zero strength is not in the chain at all: no pass, no scratch image.
    chain.set_focus_filter(&gpu.device, FocusFilter::default());
    assert!(chain.names().is_empty());

    // A positive strength puts the blur in the chain.
    chain.set_focus_filter(&gpu.device, FocusFilter { radial_blur: 0.5, ..Default::default() });
    assert_eq!(chain.names(), [NAME]);

    // The speedbreaker mask follows the blur, and the post effects stay in front of both.
    let mask = RgbaImage::new(1, 1, vec![255, 128, 128, 255]).unwrap();
    chain.set_focus_mask(&gpu.device, &gpu.queue, Some(&mask));
    assert_eq!(chain.names(), [NAME, "screen mask"]);
    chain.set_effects(&gpu.device, PostSettings { antialiasing: Antialiasing::Fxaa, ..Default::default() });
    assert_eq!(chain.names(), ["fxaa", NAME, "screen mask"]);

    // A flat frame stays flat under the full blur.
    let flat = wgpu::Color { r: 0.6, g: 0.4, b: 0.2, a: 1.0 };
    chain.set_focus_filter(&gpu.device, FocusFilter { radial_blur: 1.0, ..Default::default() });
    let blurred = gpu.run(&mut chain, (64, 64), flat, (64, 64));
    assert!(blurred.iter().all(|&p| near(p, [153, 102, 51, 255])), "{:?}", blurred[0]);

    // Zero strength takes it out again, and the rest of the chain is untouched.
    chain.set_focus_filter(&gpu.device, FocusFilter::default());
    assert_eq!(chain.names(), ["fxaa", "screen mask"]);
}
