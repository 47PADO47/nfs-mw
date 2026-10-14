//! Shadow pipelines have their own vertex layout and no colour target.

use bevy_mesh::{Mesh, MeshVertexBufferLayoutRef};
use bevy_render::render_resource::{RenderPipelineDescriptor, SpecializedMeshPipelineError};
use bevy_shader::ShaderRef;

use super::{BlendKind, ShadingKind};
use crate::mesh::ATTRIBUTE_COLOR_BGRA;

pub(super) fn shader() -> ShaderRef {
    ShaderRef::Path(
        bevy_asset::AssetPath::from_path_buf(bevy_asset::embedded_path!("shadow.wesl")).with_source("embedded"),
    )
}

pub(super) fn specialize(
    descriptor: &mut RenderPipelineDescriptor,
    layout: &MeshVertexBufferLayoutRef,
    shading: ShadingKind,
    blend: BlendKind,
) -> Result<(), SpecializedMeshPipelineError> {
    descriptor.vertex.buffers = vec![layout.0.get_layout(&[
        Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
        Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
        ATTRIBUTE_COLOR_BGRA.at_shader_location(2),
    ])?];
    descriptor.primitive.cull_mode = None;
    if let Some(fragment) = descriptor.fragment.as_mut() {
        if blend == BlendKind::AlphaTest {
            fragment.shader_defs.push("ALPHA_TEST".into());
        }
        if shading == ShadingKind::Glossy {
            fragment.shader_defs.push("GLOSSY".into());
        }
    }
    Ok(())
}
