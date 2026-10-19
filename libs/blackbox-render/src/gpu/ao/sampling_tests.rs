//! Compare fixed-pattern error against a dense estimate of the same visibility integral.
use super::tests::scene_shading;
use crate::gpu::test_support;
use blackbox_gfx::{AoMethod, AoQuality, AoSettings, GraphicsSettings, RenderBackend, Shading};

#[test]
#[ignore = "needs a GPU"]
fn spatial_sampling_reduces_error_against_a_dense_reference() {
    let _serial = test_support::serial();
    let size = (319, 237);
    let mut r = test_support::headless(size).expect("GPU adapter required");
    let (frame, instances) = scene_shading(&mut r, size, true, Shading::Prelit);
    let mut improved = true;
    for (quality, old_budget) in [(AoQuality::Low, (2, 3)), (AoQuality::Medium, (3, 4)), (AoQuality::High, (4, 4))] {
        r.apply_graphics(&GraphicsSettings {
            ao: AoSettings { method: AoMethod::XeGtao, quality, ..Default::default() },
            ..Default::default()
        });
        r.render(&frame, &instances).unwrap();
        let candidate = r.read_raw_texture(r.shared.shadows.ambient.texture(), size, 1).unwrap();
        let original = r.ao.as_ref().unwrap().pipelines.xegtao.clone();
        let dense = pipeline(&r, 24, 24);
        r.ao.as_mut().unwrap().pipelines.xegtao = dense;
        r.render(&frame, &instances).unwrap();
        let reference = r.read_raw_texture(r.shared.shadows.ambient.texture(), size, 1).unwrap();
        r.ao.as_mut().unwrap().pipelines.xegtao = pipeline(&r, old_budget.0, old_budget.1);
        r.render(&frame, &instances).unwrap();
        let legacy = r.read_raw_texture(r.shared.shadows.ambient.texture(), size, 1).unwrap();
        r.ao.as_mut().unwrap().pipelines.xegtao = original;
        let error = |mask: &[u8]| {
            let mut sum = 0.0;
            let mut count = 0;
            for (&value, &target) in mask.iter().zip(&reference) {
                if target >= 245 {
                    continue;
                }
                sum += f64::from(value.abs_diff(target)).powi(2);
                count += 1;
            }
            assert!(count > 100, "reference must contain contact occlusion");
            (sum / f64::from(count)).sqrt()
        };
        let before = error(&legacy);
        let after = error(&candidate);
        println!("{quality}: AO contact error RMS vs dense: legacy={before:.4}, candidate={after:.4}");
        improved &= after < before * 0.8;
    }
    assert!(improved, "each quality must reduce fixed sampling error");
}

pub(super) fn pipeline(renderer: &crate::Renderer, slices: u32, steps: u32) -> wgpu::ComputePipeline {
    let source = super::sources()[0]
        .1
        .replace("params.sampling.y", &format!("{slices}u"))
        .replace("params.sampling.z", &format!("{steps}u"));
    source_pipeline(renderer, source)
}

pub(super) fn source_pipeline(renderer: &crate::Renderer, source: String) -> wgpu::ComputePipeline {
    let module = renderer.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("dense AO sampling reference"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let group = renderer.ao.as_ref().unwrap().pipelines.xegtao.get_bind_group_layout(0);
    let layout = renderer.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("dense AO reference"),
        bind_group_layouts: &[Some(&group)],
        immediate_size: 0,
    });
    renderer.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("dense AO reference"),
        layout: Some(&layout),
        module: &module,
        entry_point: Some("generate_xegtao"),
        compilation_options: Default::default(),
        cache: None,
    })
}
