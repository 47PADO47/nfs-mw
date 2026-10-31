//! The rider's skinned meshes as renderer meshes: which part wears which picture (`parts.json`), the index buffers and draw
//! ranges of each mesh's level of detail, the pictures decoded and given mip levels.
//!
//! The vertices of a part change every frame (the skin follows the pose); everything here is made once.

use anyhow::{Context, Result, bail, ensure};
use blackbox_gfx::{BlendMode, DrawRange, PixelFormat, RenderBackend, Shading, TextureDesc, TextureHandle};
use blackbox_skeleton::Pack;
use serde::Deserialize;

/// The level of detail of every mesh that is drawn. A choice: the suit's most detailed level has 4,458 vertices, the
/// whole rider about 8,000, and skinning that on the CPU every frame is well under a millisecond; the coarser levels
/// (`MeshLod::screen_size`) are not used until the rider is drawn far from the camera.
pub const LEVEL: usize = 0;

/// The longest side of a picture the rider takes, pixels. The exporter writes at most 2,048.
const MAX_SIDE: u32 = 4096;

/// One line of `parts.json`: the mesh (by the name it has in the pack), the material slot it names, the picture that slot
/// wears, and whether it is drawn at all and drawn translucent.
#[derive(Debug, Clone, Deserialize)]
pub struct Entry {
    pub mesh: String,
    pub slot: String,
    pub picture: String,
    #[serde(default)]
    pub translucent: bool,
    #[serde(default = "yes")]
    pub drawn: bool,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Deserialize)]
struct File {
    parts: Vec<Entry>,
}

/// The entries of a `parts.json`.
pub fn parse_entries(bytes: &[u8]) -> Result<Vec<Entry>> {
    Ok(serde_json::from_slice::<File>(bytes).context("parts.json")?.parts)
}

/// One run of a mesh's indices with its picture.
#[derive(Debug, Clone, PartialEq)]
pub struct Draw {
    pub first_index: u32,
    pub index_count: u32,
    /// The picture's file name (`None`: the slot has no entry, the section is drawn plain).
    pub picture: Option<String>,
    pub translucent: bool,
}

/// What to upload for one mesh at one level of detail.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    /// The index in `Pack::meshes`.
    pub mesh: usize,
    pub level: usize,
    pub indices: Vec<u16>,
    pub draws: Vec<Draw>,
}

/// The parts to draw: for each mesh of the pack that has a drawn entry, its level [`LEVEL`] (the finest; a mesh without levels is left out), the
/// indices as 16-bit, and a draw for each section with the picture its material slot wears. A mesh with too many vertices
/// for 16-bit indices, or with no level, is left out and named in the second list.
pub fn plan(pack: &Pack, entries: &[Entry]) -> (Vec<Plan>, Vec<String>) {
    let (mut plans, mut skipped) = (Vec::new(), Vec::new());
    for (mesh_index, mesh) in pack.meshes.iter().enumerate() {
        let drawn: Vec<&Entry> = entries.iter().filter(|e| e.mesh == mesh.name).collect();
        if drawn.is_empty() || drawn.iter().all(|e| !e.drawn) {
            continue;
        }
        let Some(level) = (!mesh.lods.is_empty()).then_some(LEVEL) else {
            skipped.push(format!("{}: no level of detail", mesh.name));
            continue;
        };
        let lod = &mesh.lods[level];
        let Ok(indices) = lod.indices.iter().map(|&i| u16::try_from(i)).collect::<Result<Vec<u16>, _>>() else {
            skipped.push(format!("{}: {} vertices do not fit 16-bit indices", mesh.name, lod.vertices.len()));
            continue;
        };
        let draws = lod
            .sections
            .iter()
            .map(|section| {
                let slot = mesh.materials.get(section.material);
                let entry = slot.and_then(|slot| drawn.iter().find(|e| &e.slot == slot));
                Draw {
                    first_index: section.first_index,
                    index_count: section.index_count,
                    picture: entry.map(|e| e.picture.clone()),
                    translucent: entry.is_some_and(|e| e.translucent),
                }
            })
            .collect();
        plans.push(Plan { mesh: mesh_index, level, indices, draws });
    }
    (plans, skipped)
}

/// A picture as straight RGBA8.
pub struct Picture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Decode a PNG of any colour type and depth to RGBA8.
pub fn decode_png(bytes: &[u8]) -> Result<Picture> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().context("reading the PNG header")?;
    let mut data = vec![0; reader.output_buffer_size().context("the PNG is too large")?];
    let frame = reader.next_frame(&mut data).context("decoding the PNG")?;
    ensure!(frame.width > 0 && frame.height > 0, "the PNG is empty");
    ensure!(frame.width <= MAX_SIDE && frame.height <= MAX_SIDE, "the PNG is larger than {MAX_SIDE} pixels");
    let data = &data[..frame.buffer_size()];
    let rgba = match frame.color_type {
        png::ColorType::Rgba => data.to_vec(),
        png::ColorType::Rgb => data.as_chunks::<3>().0.iter().flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        png::ColorType::GrayscaleAlpha => {
            data.as_chunks::<2>().0.iter().flat_map(|p| [p[0], p[0], p[0], p[1]]).collect()
        }
        png::ColorType::Grayscale => data.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => bail!("an indexed PNG was not expanded"),
    };
    Ok(Picture { width: frame.width, height: frame.height, rgba })
}

/// The mip levels of a picture, the largest first, each the 2 x 2 average of the one before, down to 1 x 1.
pub fn mip_chain(picture: &Picture) -> Vec<(u32, u32, Vec<u8>)> {
    let mut levels = vec![(picture.width, picture.height, picture.rgba.clone())];
    loop {
        let (width, height, data) = &levels[levels.len() - 1];
        if *width == 1 && *height == 1 {
            return levels;
        }
        let (next_w, next_h) = ((*width / 2).max(1), (*height / 2).max(1));
        let mut next = Vec::with_capacity((next_w * next_h * 4) as usize);
        for y in 0..next_h {
            for x in 0..next_w {
                let mut sum = [0u32; 4];
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let (sx, sy) = (((x * 2 + dx).min(width - 1)) as usize, ((y * 2 + dy).min(height - 1)) as usize);
                    let at = (sy * *width as usize + sx) * 4;
                    for (total, &value) in sum.iter_mut().zip(&data[at..at + 4]) {
                        *total += u32::from(value);
                    }
                }
                next.extend(sum.map(|total| ((total + 2) / 4) as u8));
            }
        }
        levels.push((next_w, next_h, next));
    }
}

/// Upload a picture with its mip levels.
pub fn upload_picture(renderer: &mut dyn RenderBackend, label: &str, picture: &Picture) -> TextureHandle {
    let levels = mip_chain(picture);
    renderer.create_texture(&TextureDesc {
        label,
        width: picture.width,
        height: picture.height,
        format: PixelFormat::Rgba8,
        mips: levels.iter().map(|(_, _, data)| data.as_slice()).collect(),
    })
}

/// The draw ranges of a plan, given the texture of each picture (`texture` answers for a file name).
pub fn draw_ranges(plan: &Plan, texture: &dyn Fn(&str) -> Option<TextureHandle>) -> Vec<DrawRange> {
    plan.draws
        .iter()
        .map(|draw| DrawRange {
            first_index: draw.first_index,
            index_count: draw.index_count,
            base_vertex: 0,
            texture: draw.picture.as_deref().and_then(texture),
            blend: if draw.translucent { BlendMode::AlphaBlend } else { BlendMode::Opaque },
            shading: Shading::Lit,
            // Cloth and a helmet seen from inside are never seen, but a wrong winding in a converter would hide the whole
            // rider; the rider is drawn two-sided so that a mistake shows as a flat shade rather than as nothing.
            two_sided: true,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_skeleton::{Bone, Influences, MeshLod, Section, Skeleton, SkinVertex, SkinnedMesh, Transform};
    use glam::{Mat4, Vec3};

    fn pack(meshes: Vec<SkinnedMesh>) -> Pack {
        let bone = Bone { name: "root".into(), parent: None, rest: Transform::IDENTITY };
        Pack { skeleton: Skeleton::new(vec![bone]).unwrap(), sockets: Vec::new(), clips: Vec::new(), meshes }
    }

    /// A mesh of two sections (one per material slot) over `count` vertices.
    fn mesh(name: &str, slots: &[&str], count: usize) -> SkinnedMesh {
        let vertex =
            SkinVertex { position: Vec3::ZERO, normal: Vec3::Z, uv: [0.0; 2], influences: Influences::default() };
        let lod = MeshLod {
            screen_size: 1.0,
            vertices: vec![vertex; count],
            indices: vec![0, 1, 2, 0, 2, 1, 1, 2, 0],
            sections: vec![
                Section { material: 0, first_index: 0, index_count: 6 },
                Section { material: 1usize.min(slots.len() - 1), first_index: 6, index_count: 3 },
            ],
        };
        SkinnedMesh {
            name: name.into(),
            bone_map: vec![Some(0)],
            inverse_bind: vec![Mat4::IDENTITY],
            materials: slots.iter().map(|s| (*s).to_owned()).collect(),
            lods: vec![lod],
        }
    }

    fn entry(mesh: &str, slot: &str, picture: &str, translucent: bool, drawn: bool) -> Entry {
        Entry { mesh: mesh.into(), slot: slot.into(), picture: picture.into(), translucent, drawn }
    }

    #[test]
    fn the_entries_of_a_file_read_with_their_defaults() {
        let entries = parse_entries(br#"{"parts": [{"mesh": "a", "slot": "s", "picture": "p.png"},
            {"mesh": "b", "slot": "t", "picture": "q.png", "translucent": true, "drawn": false}], "gloves_picture": "g.png"}"#)
        .unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries[0].drawn && !entries[0].translucent);
        assert!(!entries[1].drawn && entries[1].translucent);
        assert!(parse_entries(b"{}").is_err());
    }

    #[test]
    fn a_plan_has_each_sections_picture_by_its_material_slot() {
        let pack = pack(vec![mesh("suit", &["cloth", "glove"], 4)]);
        let entries = [entry("suit", "cloth", "suit.png", false, true), entry("suit", "glove", "hand.png", true, true)];
        let (plans, skipped) = plan(&pack, &entries);
        assert!(skipped.is_empty());
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].indices, vec![0, 1, 2, 0, 2, 1, 1, 2, 0]);
        assert_eq!(plans[0].draws[0].picture.as_deref(), Some("suit.png"));
        assert_eq!(plans[0].draws[1].picture.as_deref(), Some("hand.png"));
        assert!(plans[0].draws[1].translucent && !plans[0].draws[0].translucent);
    }

    #[test]
    fn meshes_that_are_not_drawn_or_not_listed_are_left_out() {
        let pack = pack(vec![mesh("suit", &["cloth"], 4), mesh("arms", &["cloth"], 4), mesh("other", &["cloth"], 4)]);
        let entries =
            [entry("suit", "cloth", "suit.png", false, true), entry("arms", "cloth", "suit.png", false, false)];
        let (plans, _) = plan(&pack, &entries);
        assert_eq!(plans.iter().map(|p| p.mesh).collect::<Vec<_>>(), vec![0]);
    }

    #[test]
    fn a_slot_without_an_entry_is_drawn_plain_and_a_mesh_too_big_for_16_bits_is_named() {
        let pack = pack(vec![mesh("suit", &["cloth", "extra"], 4)]);
        let (plans, _) = plan(&pack, &[entry("suit", "cloth", "suit.png", false, true)]);
        assert_eq!(plans[0].draws[1].picture, None);
        let mut big = mesh("big", &["cloth"], 4);
        big.lods[0].indices[0] = 70_000;
        let (plans, skipped) = plan(&pack_of(big), &[entry("big", "cloth", "x.png", false, true)]);
        assert!(plans.is_empty());
        assert_eq!(skipped.len(), 1);
        assert!(skipped[0].starts_with("big"));
    }

    fn pack_of(mesh: SkinnedMesh) -> Pack {
        pack(vec![mesh])
    }

    #[test]
    fn a_level_beyond_the_meshs_last_is_the_last() {
        // LEVEL is 0 and a mesh has at least one level, so the first is taken; a mesh without any is named.
        let mut empty = mesh("none", &["cloth"], 4);
        empty.lods.clear();
        let (plans, skipped) = plan(&pack(vec![empty]), &[entry("none", "cloth", "x.png", false, true)]);
        assert!(plans.is_empty() && skipped.len() == 1);
    }

    #[test]
    fn draw_ranges_carry_the_blend_and_the_texture_of_their_picture() {
        let plan = Plan {
            mesh: 0,
            level: 0,
            indices: vec![0, 1, 2],
            draws: vec![
                Draw { first_index: 0, index_count: 3, picture: Some("a.png".into()), translucent: false },
                Draw { first_index: 3, index_count: 3, picture: Some("missing.png".into()), translucent: true },
            ],
        };
        let known = TextureHandle::from_raw(7);
        let ranges = draw_ranges(&plan, &|name| (name == "a.png").then_some(known));
        assert_eq!(ranges[0].texture, Some(known));
        assert_eq!(ranges[1].texture, None);
        assert_eq!(ranges[0].blend, BlendMode::Opaque);
        assert_eq!(ranges[1].blend, BlendMode::AlphaBlend);
        assert!(ranges.iter().all(|r| r.two_sided && r.shading == Shading::Lit && r.base_vertex == 0));
    }

    #[test]
    fn a_picture_halves_down_to_one_pixel_averaging_each_block() {
        // Four pixels: red, green, blue, white. The one-pixel level is their mean.
        let rgba = vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255];
        let chain = mip_chain(&Picture { width: 2, height: 2, rgba });
        assert_eq!(chain.len(), 2);
        assert_eq!((chain[1].0, chain[1].1), (1, 1));
        assert_eq!(chain[1].2, vec![128, 128, 128, 255]);
        // A wide picture keeps halving the side that is still above one.
        let wide = mip_chain(&Picture { width: 4, height: 1, rgba: vec![10; 16] });
        assert_eq!(wide.iter().map(|l| (l.0, l.1)).collect::<Vec<_>>(), vec![(4, 1), (2, 1), (1, 1)]);
        assert!(wide.iter().all(|l| l.2.iter().all(|&v| v == 10)), "a flat picture stays flat");
    }

    #[test]
    fn a_png_decodes_to_rgba_whatever_its_colour_type() {
        let encode = |color: png::ColorType, data: &[u8]| {
            let mut out = Vec::new();
            let mut encoder = png::Encoder::new(&mut out, 2, 1);
            encoder.set_color(color);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header().unwrap().write_image_data(data).unwrap();
            out
        };
        let rgb = decode_png(&encode(png::ColorType::Rgb, &[1, 2, 3, 4, 5, 6])).unwrap();
        assert_eq!((rgb.width, rgb.height), (2, 1));
        assert_eq!(rgb.rgba, vec![1, 2, 3, 255, 4, 5, 6, 255]);
        let grey = decode_png(&encode(png::ColorType::Grayscale, &[9, 8])).unwrap();
        assert_eq!(grey.rgba, vec![9, 9, 9, 255, 8, 8, 8, 255]);
        let rgba = decode_png(&encode(png::ColorType::Rgba, &[1, 2, 3, 4, 5, 6, 7, 8])).unwrap();
        assert_eq!(rgba.rgba, vec![1, 2, 3, 4, 5, 6, 7, 8]);
        assert!(decode_png(b"not a png").is_err());
    }
}
