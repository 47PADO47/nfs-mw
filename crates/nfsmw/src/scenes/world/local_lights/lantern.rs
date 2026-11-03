//! Glass emission for lantern meshes that retain the original texture UV layout.
use super::profile::{GLASS_LAYOUTS, GLASS_MASK, GlassLayout};
use blackbox_gfx::{PixelFormat, RenderBackend, TextureDesc, TextureHandle};

pub(in crate::scenes::world) use super::profile::LANTERN_TEXTURE as TEXTURE;

/// The name a split lantern's glass group gives its diffuse texture, so that group alone draws the masked copy of
/// [`TEXTURE`]; the rest of the mesh keeps the original texture and blend mode. Only the uploader's own lookup
/// resolves it, and only for split meshes.
pub(in crate::scenes::world) const MASKED_TEXTURE: u32 = TEXTURE ^ 0x9e37_79b9;

/// Upload the masked copy of the lantern texture with the same mip chain and texture-detail discard as the
/// ordinary upload. Other textures return `None`.
pub(in crate::scenes::world) fn upload(
    renderer: &mut dyn RenderBackend,
    texture: &blackbox_tpk::Texture,
    decoded: Option<&[u8]>,
    discard: u32,
) -> Option<TextureHandle> {
    if texture.name_hash != TEXTURE {
        return None;
    }
    let count = (0..texture.mip_levels.min(32)).take_while(|&level| texture.mip(level).is_some()).count() as u32;
    let first = discard.min(count.checked_sub(1)?);
    let mut levels = Vec::new();
    for level in first..count {
        let (width, height) = ((texture.width >> level).max(1), (texture.height >> level).max(1));
        let pixels = match (level, decoded) {
            (0, Some(data)) => data.to_vec(),
            _ => match blackbox_tpk::decode_mip_rgba8(texture, level) {
                Some(pixels) => pixels,
                None => break,
            },
        };
        let Some(masked) = mask_glass(width, height, pixels) else { break };
        levels.push(masked);
    }
    if levels.is_empty() {
        return None;
    }
    Some(renderer.create_texture(&TextureDesc {
        label: "masked fixture material",
        width: (texture.width >> first).max(1),
        height: (texture.height >> first).max(1),
        format: PixelFormat::Rgba8,
        mips: levels.iter().map(Vec::as_slice).collect(),
    }))
}

// The texture and pane geometry use the original normalized UV layout. This supports
// replacement artwork at any resolution without relying on its exact pixel contents.
fn mask_glass(width: u32, height: u32, mut pixels: Vec<u8>) -> Option<Vec<u8>> {
    let expected = (width as usize).checked_mul(height as usize)?.checked_mul(4)?;
    if width == 0 || height == 0 || pixels.len() != expected {
        return None;
    }
    let grid = GLASS_MASK.grid;
    for (i, pixel) in pixels.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let x = (i % width as usize) as u64 * grid / u64::from(width);
        let y = (i / width as usize) as u64 * grid / u64::from(height);
        pixel[3] = if GLASS_MASK.x.contains(&x) && GLASS_MASK.y.contains(&y) { 255 } else { 0 };
    }
    Some(pixels)
}

/// The inspected glass layout of an upper-case solid name, if it has one.
pub(in crate::scenes::world) fn layout(name: &str) -> Option<&'static GlassLayout> {
    GLASS_LAYOUTS.iter().find(|layout| layout.models.contains(&name))
}

/// Reuse the original indexed triangles, giving the glass triangles their own group that draws
/// [`MASKED_TEXTURE`]. Returns the rebuilt solid and its glass group. A layout that does not match exactly is left
/// unchanged rather than approximated.
pub(in crate::scenes::world) fn split_glass(
    solid: &blackbox_solid::Solid,
    layout: &GlassLayout,
) -> Option<(blackbox_solid::Solid, usize)> {
    let mut matching = solid
        .groups
        .iter()
        .enumerate()
        .filter(|(_, g)| g.diffuse_texture(solid) == Some(TEXTURE) && g.num_indices == layout.group_indices);
    let (group_index, _) = matching.next()?;
    if matching.next().is_some() {
        return None;
    }
    let mut rebuilt = solid.clone();
    let masked_slot = u8::try_from(rebuilt.texture_hashes.len()).ok()?;
    rebuilt.texture_hashes.push(MASKED_TEXTURE);
    rebuilt.indices.clear();
    rebuilt.groups.clear();
    let mut glass_group = None;
    for (index, group) in solid.groups.iter().enumerate() {
        let start = usize::try_from(group.first_index).ok()?;
        let end = start.checked_add(usize::try_from(group.num_indices).ok()?)?;
        let indices = solid.indices.get(start..end)?;
        if index != group_index {
            let mut group = group.clone();
            group.first_index = u32::try_from(rebuilt.indices.len()).ok()?;
            rebuilt.indices.extend_from_slice(indices);
            rebuilt.groups.push(group);
            continue;
        }
        let (triangles, remainder) = indices.as_chunks::<3>();
        if !remainder.is_empty() {
            return None;
        }
        let base = usize::try_from(group.base_vertex).ok()?;
        let mut glass = Vec::new();
        let mut rest = Vec::new();
        for triangle in triangles {
            let mut min = f32::INFINITY;
            let mut max = f32::NEG_INFINITY;
            for &index in triangle {
                let v = solid.vertices.get(base.checked_add(usize::from(index))?)?;
                min = min.min(v.uv[1]);
                max = max.max(v.uv[1]);
            }
            let inside = min >= layout.band.start && max <= layout.band.end;
            if inside && min < layout.reach.start && max > layout.reach.end {
                glass.extend_from_slice(triangle);
                continue;
            }
            rest.extend_from_slice(triangle);
        }
        if glass.len() != layout.glass_indices {
            return None;
        }
        for (pane, list) in [(false, rest), (true, glass)] {
            let mut draw = group.clone();
            draw.first_index = u32::try_from(rebuilt.indices.len()).ok()?;
            draw.num_indices = u32::try_from(list.len()).ok()?;
            draw.num_triangles = draw.num_indices / 3;
            rebuilt.indices.extend(list);
            if pane {
                draw.texture_slots[0] = masked_slot;
                glass_group = Some(rebuilt.groups.len());
            }
            rebuilt.groups.push(draw);
        }
    }
    Some((rebuilt, glass_group?))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replacement_mask_uses_normalized_coordinates_and_preserves_color() {
        for (w, h) in [(64, 64), (256, 256), (137, 83), (1, 1)] {
            let pixels = vec![17; w * h * 4];
            let result = mask_glass(w as u32, h as u32, pixels).unwrap();
            for (i, pixel) in result.as_chunks::<4>().0.iter().enumerate() {
                let x = (i % w) * 64 / w;
                let y = (i / w) * 64 / h;
                assert_eq!(&pixel[..3], &[17, 17, 17]);
                assert_eq!(pixel[3], if (4..60).contains(&x) && (5..44).contains(&y) { 255 } else { 0 });
            }
        }
        assert!(mask_glass(0, 64, vec![]).is_none());
        assert!(mask_glass(64, 64, vec![0; 12]).is_none());
    }
    #[test]
    fn only_inspected_variants_have_a_glass_layout() {
        assert!(layout("XO_PARKLAMPA_1B_00").is_some());
        assert!(layout("XO_PARKLAMPA_1Z_00").is_none());
        assert!(layout("xo_parklampa_1b_00").is_none(), "callers pass upper-case names");
    }
    #[test]
    #[ignore = "requires installed lantern meshes"]
    fn installed_glass_partition_preserves_every_triangle() {
        use game_install::GameDir;
        use nfsmw_data::world::{WorldIndex, load_section};
        let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
        let index = WorldIndex::open(&dir, "L2RA").unwrap();
        let mut stream = dir.open_file(&index.stream_file).unwrap();
        let mut names = std::collections::BTreeSet::new();
        for (i, section) in index.sections.iter().enumerate() {
            let Ok(data) = load_section(&mut stream, i, section) else { continue };
            for solid in &data.solids {
                let name = solid.name.to_ascii_uppercase();
                let Some(layout) = layout(&name) else { continue };
                if !names.insert(name.clone()) {
                    continue;
                }
                let (split, pane) = split_glass(solid, layout).expect("verified glass must be recognized");
                assert_eq!(split.groups[pane].num_indices as usize, layout.glass_indices);
                assert_eq!(split.groups[pane].diffuse_texture(&split), Some(MASKED_TEXTURE));
                let others = split.groups.iter().enumerate().filter(|(i, _)| *i != pane);
                assert!(others.into_iter().all(|(_, g)| g.diffuse_texture(&split) != Some(MASKED_TEXTURE)));
                let mut before = Vec::new();
                let mut after = Vec::new();
                for (mesh, output) in [(solid, &mut before), (&split, &mut after)] {
                    for group in &mesh.groups {
                        let start = group.first_index as usize;
                        let end = start + group.num_indices as usize;
                        for tri in mesh.indices[start..end].as_chunks::<3>().0 {
                            output.push((group.base_vertex, [tri[0], tri[1], tri[2]]));
                        }
                    }
                    output.sort();
                }
                assert_eq!(before, after, "{name} must retain its original geometry");
            }
        }
        let expected: usize = GLASS_LAYOUTS.iter().map(|layout| layout.models.len()).sum();
        assert_eq!(names.len(), expected, "all supported source models must be present");
    }
}
