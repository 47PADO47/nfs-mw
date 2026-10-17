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
    Ok(SectionData { index, solids, solid_vertices, textures, anims, scenery, spawners, collision })
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
