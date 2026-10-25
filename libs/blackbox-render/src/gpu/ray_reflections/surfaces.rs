//! Preserve each normal material's UV translation and normal source in hit vertices.
use super::mesh::Mesh;
use blackbox_gfx::{BlendMode, MeshDesc, NormalMaterialDesc, Shading};
use std::collections::HashMap;

pub(super) fn apply(
    mesh: &mut Mesh,
    desc: &MeshDesc<'_>,
    layers: &[Option<u32>],
    surfaces: &[Option<NormalMaterialDesc>],
) {
    let mut at = 0;
    let mut remapped = HashMap::new();
    for ((draw, layer), surface) in desc.draws.iter().zip(layers).zip(surfaces) {
        if layer.is_none()
            || !matches!(draw.blend, BlendMode::Opaque | BlendMode::AlphaTest)
            || matches!(draw.shading, Shading::Sky | Shading::SkyLayer(_))
        {
            continue;
        }
        let end = at + draw.index_count as usize / 3;
        let Some(surface) = surface else {
            at = end;
            continue;
        };
        mesh.textures.push(surface.normal.raw());
        for triangle in &mut mesh.triangles[at..end] {
            for index in &mut triangle[..3] {
                let key = (*index, surface.normal.raw(), surface.uv_offset.map(f32::to_bits));
                *index = *remapped.entry(key).or_insert_with(|| {
                    let mut vertex = mesh.vertices[*index as usize];
                    vertex.uv[0] += surface.uv_offset[0];
                    vertex.uv[1] += surface.uv_offset[1];
                    // Zero is the absent-map sentinel; the arena remaps logical handles to atlas layers.
                    vertex.uv[2] = surface.normal.raw() as f32 + 1.0;
                    let id = mesh.vertices.len() as u32;
                    mesh.vertices.push(vertex);
                    id
                });
            }
        }
        at = end;
    }
    mesh.textures.sort_unstable();
    mesh.textures.dedup();
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_gfx::*;
    #[test]
    fn shared_vertices_keep_each_draws_normal_map_and_uv_offset() {
        let vertices =
            [Vertex { position: [0.0, 0.0, 0.0], normal: [0.0, 0.0, 1.0], color_bgra: [128; 4], uv: [0.2, 0.3] }; 3];
        let indices = [0, 1, 2, 0, 1, 2];
        let draws = [
            DrawRange {
                first_index: 0,
                index_count: 3,
                base_vertex: 0,
                texture: None,
                blend: BlendMode::Opaque,
                shading: Shading::WorldNormal(NormalMaterialHandle::from_raw(1)),
                two_sided: true,
            },
            DrawRange {
                first_index: 3,
                index_count: 3,
                base_vertex: 0,
                texture: None,
                blend: BlendMode::Opaque,
                shading: Shading::WorldNormal(NormalMaterialHandle::from_raw(2)),
                two_sided: true,
            },
        ];
        let desc = MeshDesc {
            label: "shared source",
            vertices: &vertices,
            indices: &indices,
            draws: draws.to_vec(),
            tangents: None,
            secondary_uvs: None,
        };
        let layers = [Some(1), Some(1)];
        let mut mesh = Mesh::new(&desc, &layers).unwrap();
        let source = NormalMaterialDesc {
            model: NormalModel::Bump,
            diffuse: TextureHandle::from_raw(1),
            normal: TextureHandle::from_raw(2),
            specular: TextureHandle::from_raw(3),
            reflection: None,
            uv_offset: [0.5, 0.25],
        };
        apply(
            &mut mesh,
            &desc,
            &layers,
            &[
                Some(source),
                Some(NormalMaterialDesc { normal: TextureHandle::from_raw(4), uv_offset: [0.0; 2], ..source }),
            ],
        );
        let a = mesh.vertices[mesh.triangles[0][0] as usize];
        let b = mesh.vertices[mesh.triangles[1][0] as usize];
        assert_eq!(&a.uv[..2], &[0.7, 0.55]);
        assert_eq!(&b.uv[..2], &[0.2, 0.3]);
        assert_eq!((a.uv[2], b.uv[2]), (3.0, 5.0));
        assert_eq!(mesh.textures, [1, 2, 4]);
        assert_eq!(mesh.vertices.len(), 9);
    }
}
