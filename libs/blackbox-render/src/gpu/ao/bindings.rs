//! Reuse immutable AO bindings until an input image changes, including same-size replacement.
use super::pipelines::Pipelines;

pub(super) struct Inputs {
    views: [wgpu::TextureView; 4],
    pub groups: [wgpu::BindGroup; 3],
}

impl Inputs {
    pub fn matches(&self, views: [&wgpu::TextureView; 4]) -> bool {
        self.views.iter().zip(views).all(|(stored, current)| stored == current)
    }

    pub fn new(
        device: &wgpu::Device,
        pipelines: &Pipelines,
        params: &wgpu::Buffer,
        views: [&wgpu::TextureView; 4],
    ) -> Self {
        let [depth, normals, raw, filtered] = views;
        let uniform = || wgpu::BindGroupEntry { binding: 0, resource: params.as_entire_binding() };
        let view = |binding, view| wgpu::BindGroupEntry { binding, resource: wgpu::BindingResource::TextureView(view) };
        let make = |label, layout, entries: &[wgpu::BindGroupEntry<'_>]| {
            device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some(label), layout, entries })
        };
        let groups = [
            make(
                "AO generate inputs",
                &pipelines.generate.get_bind_group_layout(0),
                &[uniform(), view(1, depth), view(2, raw), view(4, normals)],
            ),
            make(
                "AO filter inputs",
                &pipelines.filter.get_bind_group_layout(0),
                &[uniform(), view(2, raw), view(3, filtered)],
            ),
            make(
                "AO composite inputs",
                &pipelines.resolve.get_bind_group_layout(0),
                &[uniform(), view(1, depth), view(2, filtered), view(4, normals)],
            ),
        ];
        Self { views: views.map(Clone::clone), groups }
    }
}
