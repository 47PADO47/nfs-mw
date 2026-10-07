//! Parsing one streamed section.

use std::io::{Read, Seek, SeekFrom};

use anyhow::{Context, Result};
use blackbox_scenery::ScenerySection;
use blackbox_solid::Solid;
use blackbox_streaming::StreamingSection;
use blackbox_tpk::Texture;

/// Everything a section contributes to the world.
pub struct SectionData {
    /// Index into [`super::WorldIndex::sections`].
    pub index: usize,
    pub solids: Vec<Solid>,
    pub textures: Vec<Texture>,
    pub scenery: Vec<ScenerySection>,
}

/// Parse a section's bytes. Sections start 0x800-aligned in the stream file, so
/// alignment computed from the start of `bytes` matches the file's.
pub fn parse_section(index: usize, bytes: &[u8]) -> Result<SectionData> {
    let solids = blackbox_solid::read_solids(bytes).context("solids")?;
    let mut textures = Vec::new();
    for pack in blackbox_tpk::read_texture_packs(bytes).context("texture packs")? {
        for (hash, why) in &pack.failed {
            log::debug!("texture 0x{hash:08X} in {}: {why}", pack.name);
        }
        textures.extend(pack.textures);
    }
    let scenery =
        blackbox_scenery::read_scenery_sections(bytes, &blackbox_scenery::layout::MOST_WANTED).context("scenery")?;
    Ok(SectionData { index, solids, textures, scenery })
}

/// Read and parse one section from the open stream file.
pub fn load_section(file: &mut std::fs::File, index: usize, section: &StreamingSection) -> Result<SectionData> {
    let mut bytes = vec![0; section.size as usize];
    file.seek(SeekFrom::Start(u64::from(section.file_offset)))?;
    file.read_exact(&mut bytes).with_context(|| format!("reading section {}", section.name))?;
    parse_section(index, &bytes).with_context(|| format!("parsing section {}", section.name))
}
