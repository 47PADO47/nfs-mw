//! The fused tone mapping against the separate passes, and the shader text it relies on.

use super::*;
use crate::gpu::post::{treatment, treatment_composite};
use blackbox_gfx::ColourTreatment;

#[test]
fn the_filmic_curve_is_spelled_the_same_in_every_shader_that_has_it() {
    const CURVE: &str = "(x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14)";
    for (name, source) in [
        ("tonemap", tonemap_wgsl()),
        ("bloom", bloom_wgsl()),
        ("treatment composite", treatment_composite::SOURCE.to_string()),
    ] {
        assert!(source.contains(CURVE), "{name}");
    }
}

#[test]
fn the_fused_entry_points_exist() {
    let module = validate("treatment composite", treatment_composite::SOURCE);
    for entry in ["fs_main", "fs_main_tonemap"] {
        assert!(module.entry_points.iter().any(|e| e.name == entry), "treatment composite lacks {entry}");
    }
}

#[test]
#[ignore = "needs a Vulkan GPU"]
fn fused_tonemap_vulkan() {
    check_fused(wgpu::Backends::VULKAN);
}

#[cfg(target_os = "windows")]
#[test]
#[ignore = "needs a Direct3D 12 GPU"]
fn fused_tonemap_dx12() {
    check_fused(wgpu::Backends::DX12);
}

fn grade() -> ColourTreatment {
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

/// The same chain with the tone mapping fused and as a separate pass: the same pass names, and pixels that
/// agree within two 8-bit steps (the separate pass reads the half-float intermediate of the one before it).
fn check_fused(backend: wgpu::Backends) {
    let _guard = crate::gpu::test_support::serial();
    let gpu = Gpu::new(backend);
    let size = (64, 64);
    let square = "let on = abs(pos.x - 32.0) < 6.0 && abs(pos.y - 32.0) < 6.0;
                  return select(vec4<f32>(0.1, 0.2, 0.3, 1.0), vec4<f32>(4.0, 4.0, 4.0, 1.0), on);";
    let ramp = "return vec4<f32>(pos.x / 16.0, pos.y / 64.0, 0.5, 1.0);";
    for exposure in [0.5, 1.0, 2.0] {
        let bloom = PostSettings { bloom_intensity: 1.0, tonemap: Tonemap::Aces, exposure, ..PostSettings::default() };
        let treated = PostSettings { colour_treatment: 1.0, bloom_intensity: 0.0, ..bloom };
        for (label, settings, names) in
            [("bloom", bloom, &["bloom", "tonemap"][..]), ("treatment", treated, &["scene treatment", "tonemap"][..])]
        {
            let mut fused = PostChain::new(&gpu.device, None);
            let mut separate = PostChain::new(&gpu.device, None);
            for chain in [&mut fused, &mut separate] {
                chain.set_treatment(
                    &gpu.device,
                    Some(treatment::Profile {
                        settings: grade(),
                        gain: treatment::gain_texture(&gpu.device, &gpu.queue, None),
                        luma_format: HDR_FORMAT,
                    }),
                );
                chain.set_effects(&gpu.device, settings);
            }
            separate.fuse = false;
            assert_eq!(fused.names(), names, "{label}");
            assert_eq!(separate.names(), names, "{label}");
            for body in [square, ramp] {
                let one = gpu.run(&mut fused, size, body);
                let two = gpu.run(&mut separate, size, body);
                let worst = one.iter().zip(&two).filter(|(a, b)| !near(**a, **b, 2)).count();
                assert_eq!(worst, 0, "{label} exposure {exposure}: {:?} vs {:?}", one[0], two[0]);
            }
        }
    }
}
