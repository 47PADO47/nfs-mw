//! Parsing one streamed section.

use std::io::{Read, Seek, SeekFrom};

use anyhow::{Context, Result};
use blackbox_collision::CollisionPack;
use blackbox_gfx::Vertex;
use blackbox_scenery::ScenerySection;
use blackbox_solid::Solid;
use blackbox_streaming::StreamingSection;
use blackbox_tpk::{Texture, TextureAnim};

/// Everything a section contributes to the world.
pub struct SectionData {
    /// Index into [`super::WorldIndex::sections`].
    pub index: usize,
    pub solids: Vec<Solid>,
    /// The renderer vertices of each solid in `solids`, in the same order. The loader thread builds them
    /// (the CPU half of a solid upload), so the render thread only copies them to the GPU.
    pub solid_vertices: Vec<Vec<Vertex>>,
    pub textures: Vec<Texture>,
    /// The RGBA8 of each texture in `textures` that the renderer needs decoded (`Texture::needs_decode`), in the
    /// same order; `None` for the others. The loader thread fills it ([`SectionData::decode_for_upload`]), so the
    /// upload copies these instead of decoding on the render thread.
    pub decoded: Vec<Option<Vec<u8>>>,
    /// Animated textures (water, signals) defined in this section.
    pub anims: Vec<TextureAnim>,
    pub scenery: Vec<ScenerySection>,
    /// Physics placements, joined to scenery by override hooks.
    pub spawners: Vec<blackbox_scenery::ObjectSpawner>,
    /// The section's static collision (`docs/formats/collision.md`); map tiles have one.
    pub collision: Vec<CollisionPack>,
}

impl SectionData {
    /// The bytes an upload of this section writes or converts: decoded texture pixels, vertices and indices.
    /// The residency spends a per-frame budget of these.
    pub fn upload_bytes(&self) -> usize {
        let textures: usize = self.textures.iter().map(|t| t.width as usize * t.height as usize * 4).sum();
        let vertices: usize = self.solid_vertices.iter().map(|v| v.len() * std::mem::size_of::<Vertex>()).sum();
        let indices: usize = self.solids.iter().map(|s| s.indices.len() * std::mem::size_of::<u16>()).sum();
        textures + vertices + indices
    }

    /// Decode the textures the renderer cannot draw block-compressed, into `decoded`. `bc_supported` is the renderer's
    /// BC support. Runs on the loader thread, with the same rule the upload uses.
    pub fn decode_for_upload(&mut self, bc_supported: bool) {
        self.decoded = self
            .textures
            .iter()
            .map(|t| t.needs_decode(bc_supported).then(|| blackbox_tpk::decode_rgba8(t)).flatten())
            .collect();
    }
}

/// Parse a section's bytes. Sections start 0x800-aligned in the stream file, so
/// alignment computed from the start of `bytes` matches the file's.
pub fn parse_section(index: usize, bytes: &[u8]) -> Result<SectionData> {
    let solids = blackbox_solid::read_solids(bytes).context("solids")?;
    let solid_vertices: Vec<Vec<Vertex>> = solids.iter().map(gpu_vertices).collect();
    let mut textures = Vec::new();
    for pack in blackbox_tpk::read_texture_packs(bytes).context("texture packs")? {
        for (hash, why) in &pack.failed {
            log::debug!("texture 0x{hash:08X} in {}: {why}", pack.name);
        }
        textures.extend(pack.textures);
    }
    let scenery =
        blackbox_scenery::read_scenery_sections(bytes, &blackbox_scenery::layout::MOST_WANTED).context("scenery")?;
    let spawners = blackbox_scenery::read_object_spawners(bytes, &blackbox_scenery::layout::MOST_WANTED.spawners)
        .context("physics placements")?;
    let anims = blackbox_tpk::read_texture_anims(bytes);
    let collision = blackbox_collision::read_collision_packs(bytes).context("collision")?;
    let decoded = vec![None; textures.len()];
    Ok(SectionData { index, solids, solid_vertices, textures, decoded, anims, scenery, spawners, collision })
}

/// A solid's renderer vertices, the copy `blackbox_scene` makes for an upload.
fn gpu_vertices(solid: &Solid) -> Vec<Vertex> {
    solid
        .vertices
        .iter()
        .map(|v| Vertex { position: v.position, normal: v.normal, color_bgra: v.color_bgra, uv: v.uv })
        .collect()
}

/// Read and parse one section from the open stream file.
pub fn load_section(file: &mut std::fs::File, index: usize, section: &StreamingSection) -> Result<SectionData> {
    let mut bytes = vec![0; section.size as usize];
    file.seek(SeekFrom::Start(u64::from(section.file_offset)))?;
    file.read_exact(&mut bytes).with_context(|| format!("reading section {}", section.name))?;
    parse_section(index, &bytes).with_context(|| format!("parsing section {}", section.name))
}

#[cfg(test)]
mod tests {
    use blackbox_tpk::{AlphaUsage, PixelFormat, Texture};

    use super::SectionData;

    /// A texture whose level-0 pixels follow a fixed byte pattern.
    fn texture(format: PixelFormat, width: u32, height: u32) -> Texture {
        let size = blackbox_tpk::mip_level_size(format, width, height, 0);
        Texture {
            name: "TEST".into(),
            name_hash: 0,
            width,
            height,
            mip_levels: 1,
            format,
            compression_type: 0,
            alpha_usage: AlphaUsage::None,
            alpha_sorting: false,
            alpha_blend: 0,
            data: (0..size).map(|i| (i * 37 + 11) as u8).collect(),
            palette: Vec::new(),
        }
    }

    fn section(textures: Vec<Texture>) -> SectionData {
        SectionData {
            index: 0,
            solids: Vec::new(),
            solid_vertices: Vec::new(),
            decoded: Vec::new(),
            textures,
            anims: Vec::new(),
            scenery: Vec::new(),
            spawners: Vec::new(),
            collision: Vec::new(),
        }
    }

    #[test]
    fn worker_decode_matches_the_render_thread_decode() {
        for format in [PixelFormat::Dxt1, PixelFormat::Dxt3, PixelFormat::Dxt5] {
            let t = texture(format, 8, 8);
            let mut data = section(vec![t.clone()]);
            data.decode_for_upload(false);
            assert_eq!(data.decoded[0], blackbox_tpk::decode_rgba8(&t), "{format:?}");
            assert!(data.decoded[0].is_some(), "{format:?}");
        }
    }

    #[test]
    fn block_compressed_textures_stay_compressed_with_bc() {
        let mut data = section(vec![texture(PixelFormat::Dxt5, 8, 8), texture(PixelFormat::Dxt1, 6, 6)]);
        data.decode_for_upload(true);
        // 8x8 is whole blocks, so it is uploaded as it is. 6x6 is not, so it is decoded as the upload would.
        assert_eq!(data.decoded[0], None);
        assert_eq!(data.decoded[1], blackbox_tpk::decode_rgba8(&data.textures[1]));
    }

    #[test]
    fn argb_is_decoded_even_with_bc() {
        let t = texture(PixelFormat::Argb8888, 4, 4);
        let mut data = section(vec![t.clone()]);
        data.decode_for_upload(true);
        assert_eq!(data.decoded[0], blackbox_tpk::decode_rgba8(&t));
        assert!(data.decoded[0].is_some());
    }
}
