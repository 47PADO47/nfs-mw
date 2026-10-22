//! Selected shared models without decoding unrelated meshes or texture pixels.
use super::*;

impl SharedAssets {
    /// Read only models with the supplied name hashes and the textures/animation frames they use.
    /// Two bounded passes allow textures and animation definitions to precede their models in other sections.
    pub fn load_selected(dir: &GameDir, track: &str, hashes: &[u32]) -> Result<Self> {
        if hashes.is_empty() {
            return Ok(Self { assets: Vec::new() });
        }
        let index = WorldIndex::open(dir, track)?;
        let mut solids = Vec::new();
        let mut anims = Vec::new();
        for i in index.shared() {
            let section = &index.sections[i];
            let bytes = dir
                .read_range(&index.stream_file, u64::from(section.file_offset), section.size as usize)
                .with_context(|| format!("reading section {}", section.name))?;
            solids.extend(
                blackbox_solid::read_solids_filtered(&bytes, |hash| hashes.contains(&hash))
                    .with_context(|| format!("selected solids in section {}", section.name))?,
            );
            anims.extend(blackbox_tpk::read_texture_anims(&bytes));
        }
        let needed = texture_hashes(solids.iter().flat_map(|solid| solid.texture_hashes.iter().copied()), &anims);
        let mut textures = HashMap::new();
        for i in index.shared() {
            let section = &index.sections[i];
            let bytes = dir
                .read_range(&index.stream_file, u64::from(section.file_offset), section.size as usize)
                .with_context(|| format!("reading section {}", section.name))?;
            for pack in blackbox_tpk::read_texture_packs_filtered(&bytes, |hash| needed.contains(&hash))
                .with_context(|| format!("selected textures in section {}", section.name))?
            {
                for texture in pack.textures {
                    textures.entry(texture.name_hash).or_insert(texture);
                }
            }
        }
        Ok(Self::assemble(solids, anims, textures))
    }
}

pub(super) fn texture_hashes(hashes: impl Iterator<Item = u32>, anims: &[TextureAnim]) -> HashSet<u32> {
    let mut hashes: HashSet<u32> = hashes.collect();
    // Match SharedAssets::assemble: only animations referenced directly by a selected model contribute frames.
    let frames: Vec<u32> = anims
        .iter()
        .filter(|anim| hashes.contains(&anim.name_hash))
        .flat_map(|anim| anim.frames.iter().copied())
        .collect();
    hashes.extend(frames);
    hashes
}
