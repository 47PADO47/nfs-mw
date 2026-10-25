use super::*;

#[test]
fn rt_vertex_layout_matches_shader_storage_stride() {
    assert_eq!(std::mem::size_of::<mesh::Vertex>(), 64);
}

#[cfg(windows)]
#[test]
#[ignore = "requires DX12"]
fn dx12_keeps_the_existing_renderer_even_with_prototype_opt_in() {
    let _gpu = super::super::test_support::serial();
    let renderer =
        Renderer::headless((17, 13), crate::RendererOptions { backend: crate::Backend::Dx12, ..Default::default() })
            .unwrap();
    assert!(renderer.ray_reflections.is_none());
    assert!(!renderer.device.features().contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY));
}
