use super::{Gpu, near, validate};
use crate::gpu::post::{PostChain, focus};
use blackbox_gfx::{Antialiasing, FocusFilter, PostSettings, RgbaImage};

#[test]
fn retained_focus_shader_and_uniform_validate() {
    let module = validate("screen mask", &focus::source());
    assert_eq!(module.entry_points.len(), 2);
    let uniform = module.global_variables.iter().find(|(_, g)| g.space == wgpu::naga::AddressSpace::Uniform).unwrap().1;
    assert_eq!(module.types[uniform.ty].inner.size(module.to_ctx()), 32);
    let dirty = FocusFilter { vignette: 3.0, ..Default::default() };
    let clean = dirty.sanitized();
    assert_eq!(clean, FocusFilter { vignette: 1.0, ..Default::default() });
}

#[test]
#[ignore = "needs a Vulkan GPU"]
fn retained_focus_vulkan() {
    check(wgpu::Backends::VULKAN);
}

#[cfg(target_os = "windows")]
#[test]
#[ignore = "needs a Direct3D 12 GPU"]
fn retained_focus_dx12() {
    check(wgpu::Backends::DX12);
}

fn check(backend: wgpu::Backends) {
    let _guard = crate::gpu::test_support::serial();
    let gpu = Gpu::new(backend);
    let mut chain = PostChain::new(&gpu.device);
    let mask = RgbaImage::new(1, 1, vec![255, 128, 128, 0]).unwrap();
    chain.set_focus_mask(&gpu.device, &gpu.queue, Some(&mask));
    assert_eq!(chain.names(), ["screen mask", "resolve"]);
    for size in [(64, 64), (13, 7), (1, 1)] {
        let body = "return vec4<f32>(0.6,0.4,0.2,1.0);";
        chain.set_focus_filter(&gpu.device, FocusFilter::default());
        let plain = gpu.run(&mut chain, size, body);
        assert!(plain.iter().all(|p| near(*p, [153, 102, 51, 255], 1)));
        for _ in 0..4 {
            chain.set_focus_filter(&gpu.device, FocusFilter { vignette: 1.0, ..Default::default() });
            let held = gpu.run(&mut chain, size, body);
            assert!(held.iter().all(|p| near(*p, [217, 166, 115, 255], 1)), "{backend:?}{size:?}: {:?}", held[0]);
            chain.set_focus_filter(&gpu.device, FocusFilter::default());
            assert_eq!(plain, gpu.run(&mut chain, size, body), "uniform release preserves exact inactive texels");
        }
    }
    // Green zero is neutral. Alpha/red do not control this shader operation.
    let transparent = RgbaImage::new(1, 1, vec![255, 0, 255, 0]).unwrap();
    chain.set_focus_mask(&gpu.device, &gpu.queue, Some(&transparent));
    let striped = "let white = (u32(pos.x) / 2u) % 2u == 0u;
        return vec4<f32>(vec3<f32>(select(0.0,1.0,white)),1.0);";
    let transparent_baseline = gpu.run(&mut chain, (64, 64), striped);
    chain.set_focus_filter(&gpu.device, FocusFilter { vignette: 1.0, ..Default::default() });
    assert_eq!(
        transparent_baseline,
        gpu.run(&mut chain, (64, 64), striped),
        "zero green preserves sharp nonuniform pixels; no invented radial sampling"
    );
    let flat = "return vec4<f32>(0.6,0.4,0.2,1.0);";
    assert!(gpu.run(&mut chain, (64, 64), flat).iter().all(|p| near(*p, [153, 102, 51, 255], 1)));
    // Synthetic cases checked against the unchanged native pixel shader's
    // on/off contribution. The input here already includes ordinary blue gain.
    let corner = RgbaImage::new(1, 1, vec![255, 139, 100, 255]).unwrap();
    chain.set_focus_mask(&gpu.device, &gpu.queue, Some(&corner));
    let baseline = "return vec4<f32>(60.0/255.0,40.0/255.0,20.0/255.0,1.0);";
    for (weight, expected) in [(0.0, [60, 40, 20, 255]), (0.5, [87, 67, 47, 255]), (1.0, [115, 95, 75, 255])] {
        chain.set_focus_filter(&gpu.device, FocusFilter { vignette: weight, ..Default::default() });
        assert!(gpu.run(&mut chain, (64, 64), baseline).iter().all(|p| near(*p, expected, 1)));
    }
    chain.set_focus_filter(&gpu.device, FocusFilter::default());
    chain.set_focus_mask(&gpu.device, &gpu.queue, Some(&mask));
    let stripes = "let white = (u32(pos.x) / 2u) % 2u == 0u;
        return vec4<f32>(vec3<f32>(select(0.0,1.0,white)),1.0);";
    let off = gpu.run(&mut chain, (64, 64), stripes);
    chain.set_focus_filter(&gpu.device, FocusFilter { vignette: 1.0, ..Default::default() });
    let held = gpu.run(&mut chain, (64, 64), stripes);
    assert_ne!(held, off, "green/blue contribution brightens the dark samples");
    assert!(held.iter().all(|p| near(*p, [64, 64, 64, 255], 1) || near(*p, [255; 4], 1)));
    chain.set_effects(&gpu.device, PostSettings { antialiasing: Antialiasing::Fxaa, ..Default::default() });
    assert_eq!(chain.names(), ["fxaa", "screen mask", "resolve"]);
    chain.set_effects(&gpu.device, PostSettings::default());
    assert_eq!(chain.names(), ["screen mask", "resolve"]);
    assert_eq!(held, gpu.run(&mut chain, (64, 64), stripes), "profile and uniforms survive settings changes");
    chain.set_focus_mask(&gpu.device, &gpu.queue, None);
    assert_eq!(chain.names(), ["resolve"]);
    assert_eq!(off, gpu.run(&mut chain, (64, 64), stripes), "scene-resource removal restores original rendering");
}
