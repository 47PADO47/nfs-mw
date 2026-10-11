//! Independent flat-field and spatial probes of the installed-look compositor.

use super::*;
use crate::gpu::post::{treatment, treatment_composite};
use blackbox_gfx::{ColourTreatment, RgbaImage};

#[test]
fn treatment_shaders_and_uniform_layout_validate() {
    validate("treatment filter", &treatment::filter_wgsl());
    let module = validate("treatment composite", treatment_composite::SOURCE);
    let uniform = module.global_variables.iter().find(|(_, g)| g.space == wgpu::naga::AddressSpace::Uniform).unwrap().1;
    assert_eq!(
        module.types[uniform.ty].inner.size(module.to_ctx()) as usize,
        std::mem::size_of::<treatment_composite::Parameters>()
    );
}

fn profile() -> ColourTreatment {
    ColourTreatment {
        tint: [1.0, 0.5, 0.25],
        luminance: [1.0 / 3.0; 3],
        desaturation: 0.25,
        black_intensity: 0.5,
        colour_intensity: 1.0,
        black_curve: [0.4; 4],
        colour_curve: [0.2; 4],
    }
}

fn settings(grade: bool) -> PostSettings {
    PostSettings { colour_treatment: f32::from(grade), ..Default::default() }
}

#[test]
#[ignore = "needs a Vulkan GPU"]
fn scene_treatment_vulkan() {
    check(wgpu::Backends::VULKAN);
}

#[cfg(target_os = "windows")]
#[test]
#[ignore = "needs a Direct3D 12 GPU"]
fn scene_treatment_dx12() {
    check(wgpu::Backends::DX12);
}

fn check(backend: wgpu::Backends) {
    let _guard = crate::gpu::test_support::serial();
    let gpu = Gpu::new(backend);
    let mut chain = PostChain::new(&gpu.device);
    chain.set_treatment(
        &gpu.device,
        Some(treatment::Profile {
            settings: profile(),
            gain: treatment::gain_texture(&gpu.device, &gpu.queue, None),
            luma_format: HDR_FORMAT,
        }),
    );
    let flat = "return vec4<f32>(0.6, 0.4, 0.2, 1.0);";
    for (grade, expected) in [(false, [0.6, 0.4, 0.2]), (true, [0.505, 0.32, 0.185])] {
        chain.set_effects(&gpu.device, settings(grade));
        for size in [(64, 64), (13, 7), (1, 1)] {
            let image = gpu.run(&mut chain, size, flat);
            let wanted = [byte(expected[0]), byte(expected[1]), byte(expected[2]), 255];
            assert!(
                image.iter().all(|&p| near(p, wanted, 2)),
                "{backend:?} grade={grade} {size:?}: {:?} vs {wanted:?}",
                image[0]
            );
            assert_eq!(image, gpu.run(&mut chain, size, flat), "no frame history or animated noise");
        }
    }
    // A missing profile bypasses grading, and gain-map orientation follows the original UV flip.
    chain.set_treatment(&gpu.device, None);
    chain.set_effects(&gpu.device, settings(true));
    assert_eq!(chain.names(), ["resolve"], "missing profile adds no colour pass");
    assert!(gpu.run(&mut chain, (16, 16), flat).iter().all(|&p| near(p, [153, 102, 51, 255], 1)));
    chain.set_effects(&gpu.device, PostSettings { tonemap: Tonemap::Aces, ..settings(true) });
    let hdr = gpu.run(&mut chain, (16, 16), "return vec4<f32>(1.5,1.5,1.5,1.0);");
    let expected = byte(aces(1.5));
    assert!(
        hdr.iter().all(|&p| near(p, [expected, expected, expected, 255], 2)),
        "missing look preserves HDR for later effects"
    );
    chain.set_effects(&gpu.device, PostSettings::default());
    let gain = RgbaImage::new(1, 2, vec![0, 0, 64, 255, 0, 0, 192, 255]).unwrap();
    let identity = ColourTreatment { desaturation: 0.0, black_intensity: 0.0, colour_intensity: 0.0, ..profile() };
    chain.set_treatment(
        &gpu.device,
        Some(treatment::Profile {
            settings: identity,
            gain: treatment::gain_texture(&gpu.device, &gpu.queue, Some(&gain)),
            luma_format: HDR_FORMAT,
        }),
    );
    chain.set_effects(&gpu.device, settings(true));
    let image = gpu.run(&mut chain, (16, 16), "return vec4<f32>(0.4,0.4,0.4,1.0);");
    assert!(near(image[0], [77, 77, 77, 255], 1), "top samples bottom gain: {:?}", image[0]);
    assert!(near(image[15 * 16], [26, 26, 26, 255], 1), "bottom samples top gain: {:?}", image[15 * 16]);
    chain.set_treatment(
        &gpu.device,
        Some(treatment::Profile {
            settings: profile(),
            gain: treatment::gain_texture(&gpu.device, &gpu.queue, None),
            luma_format: OUTPUT_FORMAT,
        }),
    );
    assert!(
        gpu.run(&mut chain, (13, 7), flat).iter().all(|&p| near(p, [129, 82, 47, 255], 2)),
        "grading also runs with non-HDR luminance targets"
    );
}
