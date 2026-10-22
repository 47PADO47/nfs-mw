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
    /// What the receiver let a loader thread compute from this section ([`super::Streamer::set_prepare`]); the
    /// receiver knows the type. `None` when no loader did.
    pub prepared: Option<Box<dyn std::any::Any + Send>>,
}

/// The bytes an upload of `textures` writes: the decoded pixels of the ones a loader thread decoded (`decoded`, as
/// [`SectionData::decoded`]), the stored level data of the others (block-compressed ones stay compressed on the GPU).
pub fn texture_upload_bytes(textures: &[Texture], decoded: &[Option<Vec<u8>>]) -> usize {
    let bytes = |(i, t): (usize, &Texture)| decoded.get(i).and_then(Option::as_ref).map_or(t.data.len(), Vec::len);
    textures.iter().enumerate().map(bytes).sum()
}

/// The bytes an upload of `solids` writes as meshes: vertices and indices.
pub fn solid_upload_bytes(solids: &[Solid], vertices: &[Vec<Vertex>]) -> usize {
    let vertices: usize = vertices.iter().map(|v| v.len() * std::mem::size_of::<Vertex>()).sum();
    let indices: usize = solids.iter().map(|s| s.indices.len() * std::mem::size_of::<u16>()).sum();
    vertices + indices
}

impl SectionData {
    /// The bytes an upload of this section writes or converts: texture data (see [`texture_upload_bytes`]), vertices
    /// and indices. The residency spends a per-frame budget of these.
    pub fn upload_bytes(&self) -> usize {
        self.texture_bytes() + self.mesh_bytes()
    }

    /// The texture part of [`SectionData::upload_bytes`].
    pub fn texture_bytes(&self) -> usize {
        texture_upload_bytes(&self.textures, &self.decoded)
    }

    /// The vertex and index part of [`SectionData::upload_bytes`].
    pub fn mesh_bytes(&self) -> usize {
        solid_upload_bytes(&self.solids, &self.solid_vertices)
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
    Ok(SectionData {
        index,
        solids,
        solid_vertices,
        textures,
        decoded,
        anims,
        scenery,
        spawners,
        collision,
        prepared: None,
    })
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

/// The chunk header: id and size, eight bytes.
const HEADER: u64 = 8;
/// The alignment `CollisionPack::parse` expects its payload at (`Chunk::aligned_payload`).
const PACK_ALIGN: usize = 16;

/// Read only static physics; traffic support must not decode textures or allocate GPU mesh data. The collision packs
/// are top-level chunks of the section (`docs/formats/maps.md`), so only their headers and payloads are read. A
/// section whose top level cannot be walked is read whole.
pub(super) fn load_collision<F: Read + Seek>(file: &mut F, section: &StreamingSection) -> Result<Vec<CollisionPack>> {
    let payloads = match collision_payloads(file, section) {
        Ok(payloads) => payloads,
        Err(e) => {
            log::debug!("section {}: walking the chunk headers failed ({e}); reading it whole", section.name);
            return load_collision_whole(file, section);
        }
    };
    payloads
        .iter()
        .map(|(offset, payload)| {
            let skip = offset.next_multiple_of(PACK_ALIGN) - offset;
            CollisionPack::parse(payload.get(skip..).unwrap_or(&[])).context("collision")
        })
        .collect()
}

fn load_collision_whole<F: Read + Seek>(file: &mut F, section: &StreamingSection) -> Result<Vec<CollisionPack>> {
    let mut bytes = vec![0; section.size as usize];
    file.seek(SeekFrom::Start(u64::from(section.file_offset)))?;
    file.read_exact(&mut bytes).with_context(|| format!("reading collision section {}", section.name))?;
    blackbox_collision::read_collision_packs(&bytes).context("collision")
}

/// The payloads of the section's top-level collision chunks, each with its offset in the section. The walk follows
/// the top-level headers like the chunk iterator does: a chunk that runs past the section ends it, keeping what came
/// before. A compressed blob at the top level is not walked, which is an error.
fn collision_payloads<F: Read + Seek>(
    file: &mut F,
    section: &StreamingSection,
) -> std::io::Result<Vec<(usize, Vec<u8>)>> {
    let (base, len) = (u64::from(section.file_offset), u64::from(section.size));
    let mut payloads = Vec::new();
    let mut at = 0u64;
    while at + HEADER <= len {
        file.seek(SeekFrom::Start(base + at))?;
        let mut header = [0u8; 8];
        file.read_exact(&mut header)?;
        let id = u32::from_le_bytes(header[0..4].try_into().expect("four bytes"));
        let size = u64::from(u32::from_le_bytes(header[4..8].try_into().expect("four bytes")));
        if id == blackbox_chunk::BARE_JDLZ_ID {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "a compressed chunk at the top level"));
        }
        if at + HEADER + size > len {
            return Ok(payloads);
        }
        if id == blackbox_chunk::ids::CARP_WCOLLISION_PACK {
            let mut payload = vec![0u8; size as usize];
            file.read_exact(&mut payload)?;
            payloads.push(((at + HEADER) as usize, payload));
        }
        at += HEADER + size;
    }
    Ok(payloads)
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
            prepared: None,
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

    #[test]
    fn block_compressed_textures_count_their_stored_bytes() {
        // 8x8 DXT5 is 64 bytes on the GPU, not 8 * 8 * 4.
        let data = section(vec![texture(PixelFormat::Dxt5, 8, 8)]);
        assert_eq!(data.texture_bytes(), 64);
        assert_eq!(data.upload_bytes(), 64);
    }

    #[test]
    fn a_decoded_texture_counts_its_decoded_pixels() {
        let mut data = section(vec![texture(PixelFormat::Dxt5, 8, 8), texture(PixelFormat::Argb8888, 4, 4)]);
        data.decode_for_upload(true);
        // The DXT5 stays compressed (64); the ARGB texture is decoded to the pixels the upload writes.
        let argb = data.decoded[1].as_ref().expect("decoded").len();
        assert_eq!(data.texture_bytes(), 64 + argb);
    }

    /// A stream of top-level chunks: `(id, payload)` each, as `header + payload`.
    fn stream(chunks: &[(u32, Vec<u8>)]) -> Vec<u8> {
        let mut out = Vec::new();
        for (id, payload) in chunks {
            out.extend_from_slice(&id.to_le_bytes());
            out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            out.extend_from_slice(payload);
        }
        out
    }

    fn streaming_section(size: usize, file_offset: u32) -> blackbox_streaming::StreamingSection {
        blackbox_streaming::StreamingSection {
            name: "TEST".into(),
            number: 1,
            file_type: 0,
            file_offset,
            size: size as u32,
            compressed_size: 0,
            perm_size: 0,
            priority: 0,
            center: [0.0, 0.0],
            radius: 0.0,
            checksum: 0,
        }
    }

    #[test]
    fn the_collision_walk_reads_only_the_collision_chunks() {
        let pack = blackbox_chunk::ids::CARP_WCOLLISION_PACK;
        let bytes = stream(&[
            (0x8013_4000, vec![7; 40]),
            (pack, vec![1, 2, 3, 4]),
            (0x0003_4159, vec![9; 12]),
            (pack, vec![5; 8]),
        ]);
        // The section starts 100 bytes into the file.
        let mut file = vec![0xEE; 100];
        file.extend_from_slice(&bytes);
        let section = streaming_section(bytes.len(), 100);
        let found = super::collision_payloads(&mut std::io::Cursor::new(file), &section).unwrap();
        // The first collision header is at 8 + 40, so its payload starts at 56; the second at 56 + 4 + 8 + 12 + 8 = 88.
        assert_eq!(found, vec![(56, vec![1, 2, 3, 4]), (88, vec![5; 8])]);
    }

    #[test]
    fn the_collision_walk_stops_at_a_chunk_that_overruns_the_section() {
        let pack = blackbox_chunk::ids::CARP_WCOLLISION_PACK;
        let mut bytes = stream(&[(pack, vec![1, 2, 3, 4])]);
        bytes.extend_from_slice(&stream(&[(pack, vec![0; 64])])[..16]); // cut short
        let section = streaming_section(bytes.len(), 0);
        let found = super::collision_payloads(&mut std::io::Cursor::new(bytes), &section).unwrap();
        assert_eq!(found, vec![(8, vec![1, 2, 3, 4])]);
    }

    #[test]
    fn the_collision_walk_refuses_a_compressed_top_level_chunk() {
        let bytes = stream(&[(blackbox_chunk::BARE_JDLZ_ID, vec![0; 16])]);
        let section = streaming_section(bytes.len(), 0);
        assert!(super::collision_payloads(&mut std::io::Cursor::new(bytes), &section).is_err());
    }

    #[test]
    #[ignore = "needs the game (set NFSMW_GAME_DIR)"]
    fn the_collision_walk_finds_what_a_whole_section_read_finds() {
        let Some(path) = std::env::var_os("NFSMW_GAME_DIR") else { return };
        let dir = game_install::GameDir::open(path).unwrap();
        let index = crate::world::WorldIndex::open(&dir, crate::world::DEFAULT_TRACK).unwrap();
        let mut file = dir.open_file(&index.stream_file).unwrap();
        let summary = |packs: &[super::CollisionPack]| {
            packs.iter().map(|p| (p.section, p.instances.len(), p.articles.len())).collect::<Vec<_>>()
        };
        for section in index.sections.iter().filter(|s| s.is_spatial()) {
            let walked = super::load_collision(&mut file, section).unwrap();
            let whole = super::load_collision_whole(&mut file, section).unwrap();
            assert_eq!(summary(&walked), summary(&whole), "{}", section.name);
        }
    }
}
