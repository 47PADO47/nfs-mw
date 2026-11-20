//! The pictures of the 2012 install, read by byte range: only the table of `HAWAII/GLOBALRESOURCES.BNDL` (97 MB) is read when it is
//! opened, and a texture's own blocks (the compressed stream of its pixels, 0.2 to 3 MB) when a recipe asks for it. The textures
//! the last few recipes used stay decoded, because a handful of pictures (the asphalt, the slabs, the grass) serve dozens of
//! textures of the 2005 game.

use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use anyhow::{Context, Result, bail};
use chameleon_bundle::Bundle;
use chameleon_gfx::Texture;

use super::layer::Pictures;
use super::resample::Rgba;
use crate::trees::ranges;

/// The bundle that holds the world's shared textures, under the install.
pub const BUNDLE: &str = "HAWAII/GLOBALRESOURCES.BNDL";
/// The class and sub-type bytes of the ids of the textures: "game changer" resources whose key is a plain number.
const CLASS: u64 = 0x0100_0000_0000_0000;
/// The decoded levels kept.
const KEEP: usize = 8;

/// The bundle and what was decoded from it.
pub struct Sources {
    bundle: Bundle<'static>,
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    textures: Vec<(u32, Arc<Texture>)>,
    levels: Vec<((u32, u32), Arc<Rgba>)>,
    /// Bytes of the file asked for so far, for the log.
    reads: u64,
}

impl Sources {
    /// Open the bundle under the install `root`.
    pub fn open(root: &Path) -> Result<Self> {
        let path = root.join(BUNDLE);
        let (bundle, _reader) = ranges::open(&path)?;
        Ok(Self { bundle, state: Mutex::new(State::default()) })
    }

    fn texture(&self, state: &mut State, key: u32) -> Result<Arc<Texture>> {
        if let Some((_, hit)) = state.textures.iter().find(|(known, _)| *known == key) {
            return Ok(Arc::clone(hit));
        }
        let resource = self
            .bundle
            .resource_by_id(CLASS | u64::from(key))
            .with_context(|| format!("texture {key:#x} of {BUNDLE}"))?;
        let disk = |block: usize| u64::from(resource.entry.disk_size[block] & 0x0FFF_FFFF);
        state.reads += disk(0) + disk(1);
        let texture = Arc::new(Texture::parse(&resource.blocks[0], &resource.blocks[1])?);
        state.textures.insert(0, (key, Arc::clone(&texture)));
        state.textures.truncate(KEEP);
        Ok(texture)
    }

    /// Bytes read from the install so far (the compressed blocks of the textures used).
    pub fn bytes_read(&self) -> u64 {
        self.state.lock().unwrap_or_else(PoisonError::into_inner).reads
    }
}

impl Pictures for Sources {
    fn size(&self, key: u32) -> Result<(u32, u32, u32)> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let texture = self.texture(&mut state, key)?;
        Ok((texture.width, texture.height, texture.mip_levels))
    }

    fn level(&self, key: u32, level: u32) -> Result<Arc<Rgba>> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((_, hit)) = state.levels.iter().find(|(known, _)| *known == (key, level)) {
            return Ok(Arc::clone(hit));
        }
        let texture = self.texture(&mut state, key)?;
        let Some(pixels) = texture.mip(level) else { bail!("texture {key:#x} has no mip {level}") };
        let (width, height) = ((texture.width >> level).max(1), (texture.height >> level).max(1));
        let single = Texture {
            width,
            height,
            depth: 1,
            array_size: 1,
            mip_levels: 1,
            dxgi_format: texture.dxgi_format,
            format: texture.format,
            data: pixels.to_vec(),
        };
        let Some(rgba) = single.to_rgba8() else { bail!("texture {key:#x} is in a format that is not read") };
        let picture = Arc::new(Rgba::new(width, height, rgba));
        state.levels.insert(0, ((key, level), Arc::clone(&picture)));
        state.levels.truncate(KEEP);
        Ok(picture)
    }
}
