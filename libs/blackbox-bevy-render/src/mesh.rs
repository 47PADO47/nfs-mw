//! Native meshes into Bevy meshes.
//!
//! A [`MeshDesc`] is one vertex and index buffer with several [`DrawRange`]s, each with its own texture and
//! blend. Bevy draws one material per entity, so every range becomes its own Bevy [`Mesh`], compacted to the
//! vertices it uses. Meshes live in the render world only, so the CPU copy is dropped after upload.

use bevy_asset::RenderAssetUsages;
use bevy_camera::primitives::Aabb;
use bevy_mesh::{Indices, Mesh, MeshVertexAttribute, PrimitiveTopology, VertexAttributeValues, VertexFormat};
use blackbox_gfx::{BlendMode, DrawRange, MeshDesc, Shading, TextureHandle};

/// The vertex colour, `Unorm8x4` in the games' D3DCOLOR byte order (B, G, R, A), so the shader swizzles it.
pub const ATTRIBUTE_COLOR_BGRA: MeshVertexAttribute =
    MeshVertexAttribute::new("BlackboxColorBgra", 0x4e46_5301, VertexFormat::Unorm8x4);

/// One draw range as a ready Bevy mesh plus how to draw it.
#[derive(Debug)]
pub struct RangeData {
    pub mesh: Mesh,
    pub texture: Option<TextureHandle>,
    pub blend: BlendMode,
    pub shading: Shading,
    /// Bounds of the vertices the range uses (game axes).
    pub aabb: Aabb,
    /// The average vertex colour, in the games' gamma bytes (see [`average_colour`]).
    pub colour: [u8; 3],
}

/// The average of vertex colours in the games' BGRA byte order, as RGB gamma bytes (the shader multiplies these
/// in gamma space). White when there are no vertices. Rounded to the nearest byte.
pub fn average_colour(colors: &[[u8; 4]]) -> [u8; 3] {
    let count = colors.len() as u64;
    if count == 0 {
        return [255; 3];
    }
    let mut sums = [0u64; 3];
    for &[b, g, r, _] in colors {
        sums[0] += u64::from(r);
        sums[1] += u64::from(g);
        sums[2] += u64::from(b);
    }
    sums.map(|sum| ((sum + count / 2) / count) as u8)
}

/// Every non-empty, valid draw range of `desc` as its own mesh. Ranges whose indices point outside the vertex
/// buffer or the index buffer are skipped with a warning.
pub fn split(desc: &MeshDesc<'_>) -> Vec<RangeData> {
    desc.draws.iter().filter_map(|range| range_mesh(desc, range)).collect()
}

fn range_mesh(desc: &MeshDesc<'_>, range: &DrawRange) -> Option<RangeData> {
    let first = range.first_index as usize;
    let end = first.checked_add(range.index_count as usize)?;
    if range.index_count == 0 {
        return None;
    }
    let Some(source) = desc.indices.get(first..end) else {
        log::warn!("{}: draw range {first}..{end} is outside the {} indices", desc.label, desc.indices.len());
        return None;
    };
    let base = i64::from(range.base_vertex);
    let resolved: Option<Vec<usize>> = source
        .iter()
        .map(|&i| usize::try_from(base + i64::from(i)).ok().filter(|&v| v < desc.vertices.len()))
        .collect();
    let Some(resolved) = resolved else {
        log::warn!("{}: draw range {first}..{end} indexes outside the {} vertices", desc.label, desc.vertices.len());
        return None;
    };
    Some(build(desc, &resolved, range))
}

/// Compact: the used vertices keep their order, so the vertex cache locality of the source survives.
fn build(desc: &MeshDesc<'_>, resolved: &[usize], range: &DrawRange) -> RangeData {
    let vertices = desc.vertices;
    let mut remap = vec![u32::MAX; vertices.len()];
    let mut used = Vec::new();
    for &v in resolved {
        if remap[v] == u32::MAX {
            remap[v] = 0;
            used.push(v);
        }
    }
    used.sort_unstable();
    for (new, &old) in used.iter().enumerate() {
        remap[old] = new as u32;
    }

    let mut positions = Vec::with_capacity(used.len());
    let mut normals = Vec::with_capacity(used.len());
    let mut uvs = Vec::with_capacity(used.len());
    let mut secondary_uvs = Vec::with_capacity(used.len());
    let mut colors = Vec::with_capacity(used.len());
    let (mut min, mut max) = ([f32::MAX; 3], [f32::MIN; 3]);
    for &old in &used {
        let v = &vertices[old];
        positions.push(v.position);
        normals.push(v.normal);
        uvs.push(v.uv);
        secondary_uvs.push(desc.secondary_uvs.and_then(|uvs| uvs.get(old)).copied().unwrap_or(v.uv));
        colors.push(v.color_bgra);
        for axis in 0..3 {
            min[axis] = min[axis].min(v.position[axis]);
            max[axis] = max[axis].max(v.position[axis]);
        }
    }
    // The source indices are u16, so a range uses at most 65536 distinct vertices and the new ones fit as well.
    let indices = Indices::U16(resolved.iter().map(|&v| remap[v] as u16).collect());

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, secondary_uvs);
    let aabb = Aabb::from_min_max(min.into(), max.into());
    let colour = average_colour(&colors);
    mesh.insert_attribute(ATTRIBUTE_COLOR_BGRA, VertexAttributeValues::Unorm8x4(colors));
    mesh.insert_indices(indices);
    RangeData { mesh, texture: range.texture, blend: range.blend, shading: range.shading, aabb, colour }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_gfx::Vertex;

    fn vertex(x: f32) -> Vertex {
        Vertex { position: [x, x + 1.0, x + 2.0], normal: [0.0, 0.0, 1.0], color_bgra: [1, 2, 3, 4], uv: [x, 0.0] }
    }

    fn range(first: u32, count: u32, base: i32) -> DrawRange {
        DrawRange {
            first_index: first,
            index_count: count,
            base_vertex: base,
            texture: Some(TextureHandle::from_raw(7)),
            blend: BlendMode::AlphaTest,
            shading: Shading::Prelit,
            two_sided: true,
        }
    }

    fn desc<'a>(vertices: &'a [Vertex], indices: &'a [u16], draws: Vec<DrawRange>) -> MeshDesc<'a> {
        MeshDesc { secondary_uvs: None, tangents: None, label: "test", vertices, indices, draws }
    }

    #[test]
    fn the_average_colour_is_the_rounded_mean_in_rgb_order() {
        // Stored BGRA: [B, G, R, A]. Reading the red channel from the wrong byte would swap the first and last values.
        assert_eq!(average_colour(&[[0, 0, 200, 255]]), [200, 0, 0]);
        assert_eq!(average_colour(&[[10, 20, 30, 255], [30, 40, 50, 255]]), [40, 30, 20]);
        assert_eq!(average_colour(&[[0, 0, 0, 0], [0, 0, 1, 0]]), [1, 0, 0], "0.5 rounds up");
        assert_eq!(average_colour(&[]), [255; 3], "no vertices is white");
    }

    fn u16_indices(mesh: &Mesh) -> Vec<u16> {
        match mesh.indices() {
            Some(Indices::U16(i)) => i.clone(),
            other => panic!("expected u16 indices, got {other:?}"),
        }
    }

    #[test]
    fn each_range_becomes_its_own_compacted_mesh() {
        let vertices: Vec<Vertex> = (0..8).map(|i| vertex(i as f32)).collect();
        let indices = [0, 1, 2, 5, 6, 7];
        let parts = split(&desc(&vertices, &indices, vec![range(0, 3, 0), range(3, 3, 0)]));
        assert_eq!(parts.len(), 2);
        assert_eq!(u16_indices(&parts[0].mesh), [0, 1, 2]);
        assert_eq!(parts[0].mesh.count_vertices(), 3);
        assert_eq!(u16_indices(&parts[1].mesh), [0, 1, 2], "rebased to the range's own vertices");
        let xs: Vec<f32> = match parts[1].mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
            Some(VertexAttributeValues::Float32x3(p)) => p.iter().map(|p| p[0]).collect(),
            other => panic!("{other:?}"),
        };
        assert_eq!(xs, [5.0, 6.0, 7.0]);
        assert_eq!(parts[1].blend, BlendMode::AlphaTest);
        assert_eq!(parts[1].texture, Some(TextureHandle::from_raw(7)));
    }

    #[test]
    fn base_vertex_is_added_and_shared_vertices_are_kept_once() {
        let vertices: Vec<Vertex> = (0..6).map(|i| vertex(i as f32)).collect();
        let indices = [0, 1, 2, 2, 1, 0];
        let parts = split(&desc(&vertices, &indices, vec![range(0, 6, 3)]));
        assert_eq!(parts[0].mesh.count_vertices(), 3);
        assert_eq!(u16_indices(&parts[0].mesh), [0, 1, 2, 2, 1, 0]);
        let uv = match parts[0].mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
            Some(VertexAttributeValues::Float32x2(u)) => u[0],
            other => panic!("{other:?}"),
        };
        assert_eq!(uv, [3.0, 0.0], "base_vertex 3 picks vertex 3 for index 0");
    }

    #[test]
    fn independent_uv_channel_compacts_with_vertices_and_missing_entries_use_primary() {
        let vertices: Vec<Vertex> = (0..6).map(|i| vertex(i as f32)).collect();
        let uvs = [[0.0, 9.0], [1.0, 9.0], [2.0, 9.0], [3.0, 9.0], [4.0, 9.0]];
        let mut desc = desc(&vertices, &[0, 1, 2], vec![range(0, 3, 3)]);
        desc.secondary_uvs = Some(&uvs);
        let parts = split(&desc);
        match parts[0].mesh.attribute(Mesh::ATTRIBUTE_UV_1) {
            Some(VertexAttributeValues::Float32x2(uv)) => assert_eq!(uv, &[[3.0, 9.0], [4.0, 9.0], [5.0, 0.0]]),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_bgra_colour_bytes_are_kept_as_they_are() {
        let vertices = [vertex(0.0), vertex(1.0), vertex(2.0)];
        let parts = split(&desc(&vertices, &[0, 1, 2], vec![range(0, 3, 0)]));
        match parts[0].mesh.attribute(ATTRIBUTE_COLOR_BGRA) {
            Some(VertexAttributeValues::Unorm8x4(c)) => assert_eq!(c[0], [1, 2, 3, 4]),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn bad_ranges_are_skipped() {
        let vertices = [vertex(0.0), vertex(1.0), vertex(2.0)];
        let indices = [0, 1, 2, 9];
        let draws = vec![range(0, 3, 0), range(0, 0, 0), range(3, 1, 0), range(2, 9, 0), range(0, 3, -1)];
        let parts = split(&desc(&vertices, &indices, draws));
        assert_eq!(parts.len(), 1, "only the first range is valid");
    }

    #[test]
    fn bounds_cover_the_used_vertices_only() {
        let vertices = [vertex(0.0), vertex(1.0), vertex(100.0), vertex(2.0)];
        let parts = split(&desc(&vertices, &[0, 1, 3], vec![range(0, 3, 0)]));
        let aabb = parts[0].aabb;
        assert_eq!(aabb.center, bevy_math::Vec3A::new(1.0, 2.0, 3.0));
        assert_eq!(aabb.half_extents, bevy_math::Vec3A::new(1.0, 1.0, 1.0));
    }

    #[test]
    fn a_base_vertex_far_into_a_big_buffer_still_compacts_to_small_indices() {
        let vertices: Vec<Vertex> = (0..70_000).map(|i| vertex(i as f32)).collect();
        let parts = split(&desc(&vertices, &[0, 1, 2], vec![range(0, 3, 69_000)]));
        assert_eq!(parts[0].mesh.count_vertices(), 3);
        assert_eq!(u16_indices(&parts[0].mesh), [0, 1, 2]);
    }
}
